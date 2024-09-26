use std::sync::Arc;

use crate::engine::atlas;
use crate::render::{BindingBuffer, Resources};
use crate::shaders::{self, airbrush::*};
use crate::util::PiecewiseLinear;
use glam::{vec2, Vec2};
use itertools::Itertools;
use wgpu::util::DeviceExt;

use super::embedded_shapes;

fn create_vertex_buffer(
	device: &wgpu::Device,
) -> wgpu::Buffer {
	let layout = VertexInput::vertex_buffer_layout(wgpu::VertexStepMode::Vertex);
	let buffer = device.create_buffer(&wgpu::BufferDescriptor {
		label: Some("airbrush::create_vertex_buffer"),
		size: layout.array_stride * 12,
		usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
		mapped_at_creation: false,
	});
	buffer
}

pub fn preprocess_shape_row(
	data: impl ExactSizeIterator<Item = f32>,
	opacity: f32,
) -> impl Iterator<Item = f32> {
	let data = data.into_iter();
	let scale = 1.0 / data.len() as f32;
	data
		.map(move |v| scale * (-opacity * v.max(0.0)).ln_1p())
		.scan(0.0, move |sum, value| {
			let result = Some((*sum + 0.5 * value).min(0.0));
			*sum += value;
			result
		})
}

pub fn preprocess_shape(
	shape: &embedded_shapes::Shape,
	opacity: f32,
) -> impl Iterator<Item = f32> + use<'_> {
	shape
		.values
		.chunks_exact(shape.width as usize)
		.flat_map(move |row| preprocess_shape_row(row.into_iter().copied(), opacity))
}

pub fn uniform_samples(size: u32) -> impl ExactSizeIterator<Item = f32> {
	let scale = 1.0 / (size as f32 - 1.0);
	(0..size).map(move |i| scale * i as f32)
}

pub fn centered_uniform_samples(size: u32) -> impl ExactSizeIterator<Item = f32> {
	uniform_samples(size).map(|x| 2.0 * x - 1.0)
}

pub fn generate_test_shape_row(y: f32, width: u32) -> impl ExactSizeIterator<Item = f32> {
	const SHAPE: f32 = 1.0;
	centered_uniform_samples(width).map(move |x| (1.0 - (x * x + y * y).powf(SHAPE)).max(0.0))
}

pub fn generate_test_shape(size: u32) -> embedded_shapes::Shape {
	let values = centered_uniform_samples(size)
		.flat_map(move |y| generate_test_shape_row(y, size))
		.collect();
	embedded_shapes::Shape {
		width: size,
		height: size,
		values,
	}
}

fn create_shape_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::TextureView {
	let opacity_levels = 4;

	// let shape = generate_test_shape(64);
	let shape = embedded_shapes::get_shape_00507();

	let texture_data =
		uniform_samples(opacity_levels).flat_map(|opacity| preprocess_shape(&shape, opacity));

	// let format = wgpu::TextureFormat::R8Snorm;
	// let data = data.map(|v| (v.clamp(-1.0, 1.0) * 127.0) as i8);

	let format = wgpu::TextureFormat::R16Float;
	let texture_data = texture_data.map(half::f16::from_f32);

	let texture_data: Vec<_> = texture_data.collect();
	let texture = device.create_texture_with_data(
		queue,
		&wgpu::TextureDescriptor {
			label: None,
			size: wgpu::Extent3d {
				width: shape.width,
				height: shape.height,
				depth_or_array_layers: opacity_levels,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D3,
			format,
			usage: wgpu::TextureUsages::TEXTURE_BINDING,
			view_formats: &[format],
		},
		wgpu::util::TextureDataOrder::default(),
		bytemuck::cast_slice(&texture_data),
	);
	texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn create_shape_sampler(device: &wgpu::Device) -> wgpu::Sampler {
	// It would be nice if we had feature ADDRESS_MODE_CLAMP_TO_ZERO.
	// let address_mode = wgpu::AddressMode::ClampToBorder;
	let address_mode = wgpu::AddressMode::ClampToEdge;
	device.create_sampler(&wgpu::SamplerDescriptor {
		address_mode_u: address_mode,
		address_mode_v: address_mode,
		address_mode_w: address_mode,
		mag_filter: wgpu::FilterMode::Linear,
		min_filter: wgpu::FilterMode::Linear,
		mipmap_filter: wgpu::FilterMode::Linear,
		// border_color: Some(wgpu::SamplerBorderColor::Zero),
		..Default::default()
	})
}

#[derive(Clone, Copy)]
pub struct InputPoint {
	pub position: glam::Vec2,
	pub pressure: f32,
	pub color: glam::Vec3,
	pub size: f32,
	pub opacity: f32,
	pub rate: f32,
}

pub struct Airbrush {
	pipeline: Arc<wgpu::RenderPipeline>,
	bind_group: shaders::airbrush::BindGroup0,
	action_buffer: BindingBuffer<AirbrushAction>,
	vertex_buffer: wgpu::Buffer,
	last_point: Option<InputPoint>,
}

pub struct AirbrushDrawable<'tool> {
	tool: &'tool Airbrush,
	vertex_count: u32,
	chart_keys: Vec<atlas::ChartKey>,
}

impl Airbrush {
	pub fn new(
		device: &wgpu::Device,
		queue: &wgpu::Queue,
		resources: &Resources,
		texture_format: wgpu::TextureFormat,
	) -> Self {
		let pipeline_layout = resources.airbrush.pipeline_layout().shape_texture_filterable(true).shape_sampler_filtering(wgpu::SamplerBindingType::Filtering).get();
		let pipeline = pipeline_layout.
			vs_main_pipeline(wgpu::VertexStepMode::Vertex)
			.primitive(wgpu::PrimitiveState {
				topology: wgpu::PrimitiveTopology::TriangleStrip,
				..Default::default()
			}).
			fragment(FragmentEntry::fs_main {
				targets: [Some(wgpu::ColorTargetState {
					format: texture_format,
					blend: Some(wgpu::BlendState::ALPHA_BLENDING),
					write_mask: wgpu::ColorWrites::ALL,
				})]}).
			get();

		let vertex_buffer = create_vertex_buffer(device);

		let shape_texture = create_shape_texture(device, queue);
		let shape_sampler = create_shape_sampler(device);
		
		let action_buffer = BindingBuffer::new_sized()
			.label("airbrush")
			.usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
			.create(device);
		let bind_group = pipeline_layout.bind_group_layouts().0.bind_group().action(action_buffer.as_entire_buffer_binding()).shape_texture(&shape_texture).shape_sampler(&shape_sampler).create();

		Self {
			pipeline,
			bind_group,
			action_buffer,
			vertex_buffer,
			last_point: None,
		}
	}

	pub fn start(&mut self) {}

	pub fn drag(&mut self, queue: &wgpu::Queue, point: InputPoint) -> Option<AirbrushDrawable<'_>> {
		if let Some(last_point) = self.last_point {
			let point_size = point.size * point.pressure;
			let last_point_size = last_point.size * last_point.pressure;
			let min_spacing = 0.05 * (point_size + last_point_size);
			// let min_spacing = 1.5 * (point_size + last_point_size);
			let delta_squared = (point.position - last_point.position).length_squared();
			if delta_squared < min_spacing.powi(2) {
				return None;
			}
		}

		let last_point = self.last_point.replace(point)?;

		let p0 = last_point.position;
		let p1 = point.position;

		let tangent = p1 - p0;
		let length = tangent.length();
		let tangent = tangent.normalize_or(Vec2::X);
		let normal = tangent.perp();
		let s0 = last_point.size * last_point.pressure;
		let s1 = point.size * point.pressure;

		let o0 = last_point.opacity * last_point.pressure.sqrt();
		let o1 = point.opacity * point.pressure.sqrt();
		let r0 = last_point.rate * last_point.pressure.sqrt();
		let r1 = point.rate * point.pressure.sqrt();

		let action = AirbrushAction {
			seed: glam::Vec2::new(fastrand::f32(), fastrand::f32()),
			color: point.color,
		};
		self.action_buffer.write(queue, action);

		let shift_fraction = ((s0 - s1) / length).clamp(-1.0, 1.0);
		let blend = if length > s0 + s1 {
			PiecewiseLinear::new([
				(-s0, 0.0),
				(s0 * shift_fraction, 0.0),
				(length + s1 * shift_fraction, 1.0),
				(length + s1, 1.0),
			])
		} else {
			let (b0, b1) = if s1 > length + s0 {
				((1.0 - length / (s1 - s0)).max(0.0), 1.0)
			} else if s0 > length + s1 {
				(0.0, (length / (s0 - s1)).min(1.0))
			} else {
				(0.0, 1.0)
			};
			PiecewiseLinear::new([
				(0.0 - (s0 + b0 * (s1 - s0)), b0),
				(length + (s0 + b1 * (s1 - s0)), b1),
			])
		};
		let blend = blend.unwrap();

		let u_start = {
			let (d, b) = blend.last_inflection_point();
			let s = s0 + b * (s1 - s0);
			PiecewiseLinear::new([(d - 2.0 * s, 0.0), (d, 1.0)])
		};
		let u_end = {
			let (d, b) = blend.first_inflection_point();
			let s = s0 + b * (s1 - s0);
			PiecewiseLinear::new([(d, 0.0), (d + 2.0 * s, 1.0)])
		};
		let (u_start, u_end) = (u_start.unwrap(), u_end.unwrap());

		let u_bounds = u_start.bilinear_map(&u_end, vec2);
		let events = blend
			.map_merged_inflection_points(&u_bounds, move |distance, blend, u_bounds| {
				(distance, blend, u_bounds)
			});

		let mut vertices = Vec::with_capacity(2 * events.len());
		for (distance, blend, u_bounds) in events {
			let p = p0 + distance * tangent;
			let width = s0 + blend * (s1 - s0);
			let opacity = o0 + blend * (o1 - o0);
			let rate = r0 + blend * (r1 - r0);
			vertices.extend([
				VertexInput {
					position: p - width * normal,
					u_bounds,
					opacity,
					rate,
					width,
				},
				VertexInput {
					position: p + width * normal,
					u_bounds,
					opacity,
					rate,
					width,
				},
			])
		}
		queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));

		let chart_keys = get_triangle_strip_chart_keys(vertices.iter().map(|v| v.position)).collect();

		Some(AirbrushDrawable {
			tool: self,
			vertex_count: vertices.len() as u32,
			chart_keys,
		})
	}

	pub fn stop(&mut self) {
		self.last_point = None;
	}
}

fn get_triangle_strip_chart_keys(
	vertices: impl IntoIterator<Item = Vec2>,
) -> impl Iterator<Item = atlas::ChartKey> {
	let triangles = vertices.into_iter().tuple_windows();
	triangles
		.flat_map(|(a, b, c)| atlas::ChartKey::find_covering(a, b, c))
		.collect::<std::collections::HashSet<_>>()
		.into_iter()
}

impl<'tool> AirbrushDrawable<'tool> {
	pub fn get_chart_keys(&self) -> impl Iterator<Item = atlas::ChartKey> + '_ {
		self.chart_keys.iter().cloned()
	}

	pub fn draw(&self, render_pass: &mut wgpu::RenderPass<'_>) {
		render_pass.set_pipeline(&self.tool.pipeline);
		self.tool.bind_group.set(render_pass);
		render_pass.set_vertex_buffer(0, self.tool.vertex_buffer.slice(..));
		render_pass.draw(0..self.vertex_count, 0..1);
	}
}

#[cfg(test)]
mod tests {
	use glam::*;
	use itertools::Itertools;

	use super::*;
	use crate::render::*;
	use crate::test;

	#[test]
	fn shape() {
		for y in [0.0, 0.5, 1.0] {
			println!("y = {y}");
			let shape = generate_test_shape_row(y, 8).collect_vec();
			println!("  {shape:?}");
		}
	}

	#[test]
	fn preprocess_shape() {
		for opacity in [0.0, 0.25, 0.5, 0.75, 1.0] {
			println!("opacity = {opacity}");
			let shape = preprocess_shape_row(generate_test_shape_row(0.0, 8), opacity).collect_vec();
			println!("  {shape:?}");
		}
	}

	#[test]
	fn draw() -> anyhow::Result<()> {
		let context = test::WgpuTestContext::new()?;
		let device = context.device();
		let queue = context.queue();

		let resources = Resources::new(device);

		let texture_format = wgpu::TextureFormat::Rgba8Unorm;
		let mut airbrush = Airbrush::new(device, queue, &resources, texture_format);

		let tile_data = TileData {
			chart_to_canvas_scale: Vec2::ONE,
			chart_to_canvas_translation: Vec2::ZERO,
		};
		let tile_data_buffer = BindingBuffer::init_sized(&tile_data).create(device);
		let layer_index_buffer = BindingBuffer::init_sized(&0u32).create(device);
		let tile_data_bind_group = BindGroupLayout1::new(device.clone()).bind_group().tile_data(tile_data_buffer.as_entire_buffer_binding()).layer_index(
				layer_index_buffer.as_entire_buffer_binding(),
		).create();

		airbrush.start();

		let input_point = InputPoint {
			position: vec2(0.3, 0.3),
			pressure: 0.5f32,
			color: Vec3::ONE,
			size: 0.4f32,
			opacity: 15f32,
			rate: 1f32,
		};
		assert!(airbrush.drag(queue, input_point.clone()).is_none());

		let input_point = InputPoint {
			position: vec2(0.8, 0.9),
			size: 0.1f32,
			..input_point
		};
		let drawable = airbrush.drag(queue, input_point.clone()).unwrap();

		context.render_golden_commands(
			"engine/airbrush/draw",
			test::GoldenOptions {
				width: 256,
				height: 256,
				texture_format,
				..Default::default()
			},
			move |view, encoder| {
				let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
					color_attachments: &[Some(wgpu::RenderPassColorAttachment {
						view: &view,
						resolve_target: None,
						ops: wgpu::Operations {
							load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
							store: wgpu::StoreOp::Store,
						},
					})],
					..Default::default()
				});
				tile_data_bind_group.set(&mut render_pass);
				drawable.draw(&mut render_pass);
			},
		)
	}
}
