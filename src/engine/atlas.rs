use super::tile::{self, TextureLayerDescriptor};
use crate::shaders::atlas::bind_groups::{BindGroup0, BindGroupLayout0};
use crate::shaders::atlas::*;
use crate::WgpuContext;
use glam::Vec2;
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

const CHART_SIZE: u32 = 256;
const CHART_SCALE: f32 = CHART_SIZE as f32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ChartKey(i32, i32);

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

	pub fn chart_to_image(&self) -> glam::Affine2 {
		glam::Affine2::from_mat2_translation(
			CHART_SCALE * glam::Mat2::IDENTITY,
			glam::Vec2::new(CHART_SCALE * self.0 as f32, CHART_SCALE * self.1 as f32),
		)
	}
}

type Chart = tile::Tile<ChartData>;

#[derive(Clone)]
struct Atlas {
	tile_pool: tile::Pool<ChartData>,
	charts: HashMap<ChartKey, Arc<Chart>>,
	usage_bind_group: Arc<BindGroup0>,
}

impl Atlas {
	pub fn new(context: Arc<WgpuContext>) -> Self {
		let device = context.device();
		let chart_sampler = &device.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			address_mode_w: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Nearest,
			min_filter: wgpu::FilterMode::Nearest,
			mipmap_filter: wgpu::FilterMode::Linear,
			..Default::default()
		});
		let usage_bind_group =
			BindGroup0::from_bindings(device, BindGroupLayout0 { chart_sampler }).into();

		Atlas {
			tile_pool: tile::Pool::new(
				context,
				TextureLayerDescriptor {
					format: wgpu::TextureFormat::Rgba16Float,
					..Default::default()
				},
			),
			charts: HashMap::new(),
			usage_bind_group,
		}
	}

	pub fn get_chart(&self, key: &ChartKey) -> Option<Arc<Chart>> {
		self.charts.get(key).cloned()
	}

	pub fn get_chart_mut(&mut self, key: ChartKey) -> &mut Chart {
		let chart = self
			.charts
			.entry(key)
			.or_insert_with(|| self.tile_pool.allocate_tile().into());
		Arc::make_mut(chart)
	}
}

// TODO: Test with wgpu-test (https://github.com/gfx-rs/wgpu/tree/v0.20.0/tests)
