use crate::{render, util::PiecewiseLinear};
use crate::render::Resources;
use crate::shaders::airbrush::*;
use encase::ShaderType;
use glam::{vec2, Vec2};
use itertools::Itertools;
use wgpu::util::DeviceExt;

fn create_vertex_buffer(
	device: &wgpu::Device,
) -> (wgpu::VertexBufferLayout<'static>, wgpu::Buffer) {
	let layout = VertexInput::vertex_buffer_layout(wgpu::VertexStepMode::Vertex);
	let buffer = device.create_buffer(&wgpu::BufferDescriptor {
		label: Some("airbrush::create_vertex_buffer"),
		size: layout.array_stride * 12,
		usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
		mapped_at_creation: false,
	});
	(layout, buffer)
}

fn create_pipeline(
	device: &wgpu::Device,
	texture_format: wgpu::TextureFormat,
	shader: &render::Shader,
	vertex_buffer_layout: wgpu::VertexBufferLayout<'_>,
) -> wgpu::RenderPipeline {
	device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some("airbrush"),
		layout: Some(&shader.layout),
		vertex: wgpu::VertexState {
			module: &shader.module,
			entry_point: ENTRY_VS_MAIN,
			compilation_options: Default::default(),
			buffers: &[vertex_buffer_layout],
		},
		fragment: Some(fragment_state(
			&shader.module,
			&fs_main_entry([Some(wgpu::ColorTargetState {
				format: texture_format,
				blend: Some(wgpu::BlendState::ALPHA_BLENDING),
				write_mask: wgpu::ColorWrites::ALL,
			})]),
		)),
		primitive: wgpu::PrimitiveState {
			topology: wgpu::PrimitiveTopology::TriangleStrip,
			strip_index_format: None,
			front_face: wgpu::FrontFace::Ccw,
			cull_mode: None,
			polygon_mode: wgpu::PolygonMode::Fill,
			unclipped_depth: false,
			conservative: false,
		},
		depth_stencil: None,
		multisample: wgpu::MultisampleState::default(),
		multiview: None,
		cache: None,
	})
}

fn create_bind_group(
	device: &wgpu::Device,
	shape_texture: &wgpu::TextureView,
	shape_sampler: &wgpu::Sampler,
) -> (bind_groups::BindGroup0, wgpu::Buffer) {
	use bind_groups::*;
	let contents: Vec<_> = std::iter::repeat(0u8)
		.take(AirbrushAction::min_size().get() as usize)
		.collect();
	let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("airbrush"),
		contents: bytemuck::cast_slice(&contents),
		usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
	});
	let bind_group = BindGroup0::from_bindings(
		device,
		BindGroupLayout0 {
			action: buffer.as_entire_buffer_binding(),
			shape_texture: shape_texture,
			shape_sampler: shape_sampler,
		},
	);
	(bind_group, buffer)
}

pub fn integrate_shape_row(data: impl IntoIterator<Item = f32>) -> impl Iterator<Item = f32> {
	let data = data.into_iter();
	data.scan(0.0, |sum, value| {
		let result = Some(*sum + 0.5 * value);
		*sum += value;
		result
	})
}

pub fn integrate_shape_rows<'a>(data: &'a [f32], width: u32) -> impl Iterator<Item = f32> + 'a {
	data
		.chunks_exact(width as usize)
		.flat_map(|row| integrate_shape_row(row.iter().copied()))
}

pub fn uniform_samples(size: u32) -> impl Iterator<Item = f32> {
	let scale = 2.0 / (size as f32 - 1.0);
	(0..size).map(move |i| scale * i as f32 - 1.0)
}

pub fn generate_shape_row(y: f32, width: u32) -> impl Iterator<Item = f32> {
	uniform_samples(width).map(move |x| (x * x + y * y).min(1.0).ln())
}

fn create_shape_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::TextureView {
	let size = 64u32;

	let data =
		uniform_samples(size).flat_map(move |y| integrate_shape_row(generate_shape_row(y, size)));

	// let format = wgpu::TextureFormat::R8Snorm;
	// let data = data.map(|v| ((v / 2.4).clamp(-1.0, 1.0) * 127.0) as i8);

	let format = wgpu::TextureFormat::R16Float;
	let data = data.map(half::f16::from_f32);

	let data: Vec<_> = data.collect();
	let texture = device.create_texture_with_data(
		queue,
		&wgpu::TextureDescriptor {
			label: None,
			size: wgpu::Extent3d {
				width: size,
				height: size,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format,
			usage: wgpu::TextureUsages::TEXTURE_BINDING,
			view_formats: &[format],
		},
		wgpu::util::TextureDataOrder::default(),
		bytemuck::cast_slice(&data),
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
	pub hardness: f32,
}

pub struct Airbrush {
	pipeline: wgpu::RenderPipeline,
	bind_group: bind_groups::BindGroup0,
	action_buffer: wgpu::Buffer,
	vertex_buffer: wgpu::Buffer,
	last_point: Option<InputPoint>,
}

pub struct AirbrushDrawable<'tool> {
	tool: &'tool Airbrush,
	vertex_count: u32,
}

impl Airbrush {
	pub fn new(
		device: &wgpu::Device,
		queue: &wgpu::Queue,
		resources: &Resources,
		texture_format: wgpu::TextureFormat,
	) -> Self {
		let (vertex_buffer_layout, vertex_buffer) = create_vertex_buffer(device);
		let pipeline = create_pipeline(
			device,
			texture_format,
			&resources.airbrush,
			vertex_buffer_layout,
		);
		let shape_texture = create_shape_texture(device, queue);
		let shape_sampler = create_shape_sampler(device);
		let (bind_group, action_buffer) = create_bind_group(device, &shape_texture, &shape_sampler);
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

		let mut contents = encase::UniformBuffer::new(Vec::<u8>::new());
		contents
			.write(&AirbrushAction {
				seed: glam::Vec2::new(fastrand::f32(), fastrand::f32()),
				color: point.color,
				opacity: point.pressure * point.opacity,
				hardness: point.hardness,
			})
			.unwrap();
		queue.write_buffer(&self.action_buffer, 0, &contents.into_inner());

		let shift_fraction = ((s0 - s1) * s0 / length).clamp(-1.0, 1.0);
		let (width, u_start, u_end) = if length > s0 + s1 {
			(
				PiecewiseLinear::new([(-s0, s0), (s0 * shift_fraction, s0), (length + s1 * shift_fraction, s1), (length + s1, s1)]),
				PiecewiseLinear::new([(length-s1, 0.0), (length+s1, 1.0)]),
				PiecewiseLinear::new([(-s0, 0.0), (s0, 1.0)]),
			)
		} else {
			let s = s0.max(s1);
			(
				PiecewiseLinear::new([(-s, s), (length + s, s)]),
				PiecewiseLinear::new([(length-s, 0.0), (length+s, 1.0)]),
				PiecewiseLinear::new([(-s, 0.0), (s, 1.0)]),
			)
		};
		let (width, u_start, u_end) = (width.unwrap(), u_start.unwrap(), u_end.unwrap());

		let u_bounds = u_start.bilinear_map(&u_end, vec2);
		let events = width.map_merged_inflection_points(&u_bounds, |d, w, b| (d, w, b));

		let mut vertices = Vec::with_capacity(12);
		for (distance, width, u_bounds) in events {
			let p = p0 + distance * tangent;
			vertices.extend([
				VertexInput {
					position: p - width * normal,
					u_bounds,
				},
				VertexInput {
					position: p + width * normal,
					u_bounds,
				},
			])
		}
		queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));

		Some(AirbrushDrawable {
			tool: self,
			vertex_count: vertices.len() as u32,
		})
	}

	pub fn stop(&mut self) {
		self.last_point = None;
	}
}

impl<'tool> AirbrushDrawable<'tool> {
	pub fn draw(&self, render_pass: &mut wgpu::RenderPass<'_>) {
		render_pass.set_pipeline(&self.tool.pipeline);
		self.tool.bind_group.set(render_pass);
		render_pass.set_vertex_buffer(0, self.tool.vertex_buffer.slice(..));
		// TODO: Pass in uniforms for the position and other parameters.
		// https://sotrh.github.io/learn-wgpu/beginner/tutorial6-uniforms/#uniform-buffers-and-bind-groups
		render_pass.draw(0..self.vertex_count, 0..1);
	}
}

#[cfg(test)]
mod tests {
	use itertools::Itertools;

	use super::*;

	#[test]
	fn shape() {
		for y in [-0.25, 0.0, 0.5, 1.0] {
			let shape = generate_shape_row(y, 8).collect_vec();
			println!("shape at y = {y}:\n  {shape:?}");
		}
	}

	#[test]
	fn integrate_shape() {
		for y in [-0.25, 0.0, 0.5, 1.0] {
			let shape = integrate_shape_row(generate_shape_row(y, 8)).collect_vec();
			println!("integrated shape at y = {y}:\n  {shape:?}");
		}
	}
}
