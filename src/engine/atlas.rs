use itertools::Itertools;
use std::collections::HashMap;
use std::rc::Rc;
use wgpu::util::DeviceExt;

use crate::geom::*;
use crate::render::*;

const CHART_SIZE: u32 = 256;
const CHART_SCALE: f32 = CHART_SIZE as f32;

struct Point<T>(T, T);

struct AxisAlignedBox<T> {
	min: Point<T>,
	max: Point<T>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ChartKey(i32, i32);

impl ChartKey {
	pub fn find_containing(p: Point<f32>) -> Self {
		ChartKey(
			(p.0 / CHART_SCALE).floor() as i32,
			(p.1 / CHART_SCALE).floor() as i32,
		)
	}

	pub fn find_intersecting(shape: AxisAlignedBox<f32>) -> impl Iterator<Item = ChartKey> {
		let min = ChartKey::find_containing(shape.min);
		let max = ChartKey::find_containing(shape.max);
		(min.0..=max.0)
			.cartesian_product(min.1..=max.1)
			.map(|(x, y)| ChartKey(x, y))
	}

	pub fn chart_to_image(&self) -> Similar2f {
		Similar2f::new(
			Scale2f(CHART_SCALE),
			Trans2f::new(CHART_SCALE * self.0 as f32, CHART_SCALE * self.1 as f32),
		)
	}
}

struct Chart {
	texture: wgpu::Texture,
	texture_view: wgpu::TextureView,
	bind_group: wgpu::BindGroup,
}

impl Clone for Chart {
	fn clone(&self) -> Self {
		// 1. Copy the texture. We may need a compute kennel and queue for this.
		// 2. Create a new view.
		// 3. Create a new bind group, but re-use `chart_to_image_buffer` which we'll need to hold in
		//    an RC.
		todo!()
	}
}

impl Chart {
	fn new(
		device: &wgpu::Device,
		bind_group_layout: &wgpu::BindGroupLayout,
		sampler: &wgpu::Sampler,
		format: wgpu::TextureFormat,
		key: ChartKey,
	) -> Self {
		let texture = device.create_texture(&wgpu::TextureDescriptor {
			label: Some("chart_texture"),
			size: wgpu::Extent3d {
				width: CHART_SIZE,
				height: CHART_SIZE,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format,
			usage: wgpu::TextureUsages::all(),
			view_formats: &[format],
		});
		let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

		let chart_to_image_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("drawing_action"),
			contents: bytemuck::cast_slice(&[key.chart_to_image().to_mat4x4_uniform()]),
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		});

		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("chart_bind_group"),
			layout: &bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: chart_to_image_buffer.as_entire_binding(),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::TextureView(&texture_view),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::Sampler(sampler),
				},
			],
		});
		Self {
			texture,
			texture_view,
			bind_group,
		}
	}
}

#[derive(Clone)]
struct Atlas {
	device: Rc<wgpu::Device>,
	charts: HashMap<ChartKey, Rc<Chart>>,
	bind_group_layout: WgpuBindGroupLayout,
	sampler: WgpuSampler,
}

impl Atlas {
	pub fn new(device: Rc<wgpu::Device>, resources: &Resources) -> Self {
		Atlas {
			device: device.clone(),
			charts: HashMap::new(),
			bind_group_layout: resources.chart_bind_group_layout.clone(),
			sampler: Rc::new(device.create_sampler(&wgpu::SamplerDescriptor {
				address_mode_u: wgpu::AddressMode::ClampToEdge,
				address_mode_v: wgpu::AddressMode::ClampToEdge,
				address_mode_w: wgpu::AddressMode::ClampToEdge,
				mag_filter: wgpu::FilterMode::Nearest,
				min_filter: wgpu::FilterMode::Nearest,
				mipmap_filter: wgpu::FilterMode::Nearest,
				..Default::default()
			})),
		}
	}

	pub fn get_chart(&self, key: &ChartKey) -> Option<Rc<Chart>> {
		self.charts.get(key).cloned()
	}

	pub fn get_chart_mut(&mut self, key: ChartKey) -> &mut Chart {
		Rc::make_mut(self.charts.entry(key).or_insert_with(|| {
			Rc::new(Chart::new(
				&self.device,
				&self.bind_group_layout,
				&self.sampler,
				wgpu::TextureFormat::Rgba16Float,
				key,
			))
		}))
	}
}

// TODO: Test with wgpu-test (https://github.com/gfx-rs/wgpu/tree/v0.20.0/tests)
