use super::tile::{self, TextureLayerDescriptor};
use super::Extent2d;
use crate::shaders::TileData;
use crate::WgpuContext;
use glam::*;
use itertools::Itertools;
use std::collections::HashMap;
use std::sync::Arc;

pub struct AABox {
	min: Vec2,
	max: Vec2,
}

impl AABox {
	pub fn new(min: Vec2, max: Vec2) -> Self {
		Self { min, max }
	}

	pub fn empty() -> Self {
		Self::new(Vec2::MAX, Vec2::MIN)
	}

	pub fn is_empty(&self) -> bool {
		self.min.x > self.max.x && self.min.y > self.max.y
	}

	pub fn expanded_to_contain(self, point: Vec2) -> Self {
		Self::new(self.min.min(point), self.max.max(point))
	}

	pub fn containing(points: impl Iterator<Item = Vec2>) -> Self {
		points.fold(Self::empty(), |b, p| b.expanded_to_contain(p))
	}

	pub fn contains(&self, point: Vec2) -> bool {
		point.x < self.max.x
			&& point.y < self.max.y
			&& !(point.x < self.min.x)
			&& !(point.y < self.min.y)
	}

	pub fn corners(&self) -> [Vec2; 4] {
		[
			self.min,
			Vec2::new(self.min[0], self.max[1]),
			self.max,
			Vec2::new(self.max[0], self.min[1]),
		]
	}
}

pub const CHART_SIZE: u32 = 256;
pub const CHART_SCALE: f32 = CHART_SIZE as f32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChartKey(pub i32, pub i32);

impl ChartKey {
	pub fn find_containing(p: glam::Vec2) -> Self {
		ChartKey(
			(p.x / CHART_SCALE).floor() as i32,
			(p.y / CHART_SCALE).floor() as i32,
		)
	}

	pub fn find_intersecting(shape: AABox) -> impl Iterator<Item = ChartKey> {
		let min = ChartKey::find_containing(shape.min);
		let max = ChartKey::find_containing(shape.max);
		(min.0..=max.0)
			.cartesian_product(min.1..=max.1)
			.map(|(x, y)| ChartKey(x, y))
	}

	pub fn chart_to_canvas_scale_and_translation(&self) -> (Vec2, Vec2) {
		let scale = vec2(CHART_SCALE, CHART_SCALE);
		(scale, scale * vec2(self.0 as f32, self.1 as f32))
	}

	pub fn chart_to_canvas(&self) -> Affine2 {
		let (scale, translation) = self.chart_to_canvas_scale_and_translation();
		Affine2::from_mat2_translation(Mat2::from_diagonal(scale), translation)
	}

	pub fn chart_to_canvas_mat4(&self) -> Mat4 {
		let (scale, translation) = self.chart_to_canvas_scale_and_translation();
		Mat4::from_scale_rotation_translation(
			vec3(scale.x, scale.y, 1.0),
			Quat::IDENTITY,
			vec3(translation.x, translation.y, 0.0),
		)
	}
}

// TODO: Unify this with `Tile`?
#[derive(Clone)]
pub struct Chart {
	tile: tile::Tile,
}

impl Chart {
	fn new(tile: tile::Tile) -> Self {
		Self { tile }
	}

	pub fn tile(&self) -> &tile::Tile {
		&self.tile
	}
}

#[derive(Clone)]
pub struct Atlas {
	tile_pool: tile::Pool,
	charts: HashMap<ChartKey, Arc<Chart>>,
	// usage_bind_group: Arc<BindGroup0>,
}

impl Atlas {
	pub fn new(context: Arc<WgpuContext>, format: wgpu::TextureFormat) -> Self {
		// let device = context.device();
		// let chart_sampler = &device.create_sampler(&wgpu::SamplerDescriptor {
		// 	address_mode_u: wgpu::AddressMode::ClampToEdge,
		// 	address_mode_v: wgpu::AddressMode::ClampToEdge,
		// 	address_mode_w: wgpu::AddressMode::ClampToEdge,
		// 	mag_filter: wgpu::FilterMode::Nearest,
		// 	min_filter: wgpu::FilterMode::Nearest,
		// 	mipmap_filter: wgpu::FilterMode::Nearest,
		// 	..Default::default()
		// });
		// let usage_bind_group =
		// 	BindGroup0::from_bindings(device, BindGroupLayout0 { chart_sampler }).into();

		Atlas {
			tile_pool: tile::Pool::new(
				context,
				TextureLayerDescriptor {
					size: Extent2d {
						width: CHART_SIZE,
						height: CHART_SIZE,
					},
					format,
					..Default::default()
				},
			),
			charts: HashMap::new(),
			// usage_bind_group,
		}
	}

	pub fn buffer_layout(&self) -> wgpu::VertexBufferLayout<'static> {
		self.tile_pool.buffer_layout()
	}

	pub fn charts(&self) -> impl Iterator<Item = Arc<Chart>> + '_ {
		self.charts.values().cloned()
	}

	pub fn get_chart(&self, key: &ChartKey) -> Option<Arc<Chart>> {
		self.charts.get(key).cloned()
	}

	pub fn get_chart_mut(&mut self, key: ChartKey) -> &mut Chart {
		let chart = self.charts.entry(key).or_insert_with(|| {
			let tile = self.tile_pool.allocate_tile();
			let (chart_to_canvas_scale, chart_to_canvas_translation) =
				key.chart_to_canvas_scale_and_translation();
			let tile_data = TileData {
				chart_to_canvas_scale,
				chart_to_canvas_translation,
			};
			tile.set_data(&tile_data);

			let zero = half::f16::from_f32(0f32);
			tile.fill_texture(bytemuck::cast_slice(&[zero, zero, zero, zero]));
			Chart::new(tile).into()
		});
		// TODO: When this clones, we need to put that back in the atlas.
		Arc::make_mut(chart)
	}
}

// TODO: Test with wgpu-test (https://github.com/gfx-rs/wgpu/tree/v0.20.0/tests)
