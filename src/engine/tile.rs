use crate::render::BindingBuffer;
use crate::{
	shaders::tile_read as read, shaders::tile_write as write, shaders::TileData, util::QueueExt,
	WgpuContext,
};
use bon::bon;
use encase::ShaderSize;
use encase::ShaderType;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use wgpu::{util::DeviceExt, BufferAddress, Extent3d};

struct StableVec<T> {
	vec: Mutex<Vec<Pin<Box<T>>>>,
}

impl<T> Default for StableVec<T> {
	fn default() -> Self {
		Self {
			vec: Default::default(),
		}
	}
}

impl<T> StableVec<T> {
	pub fn push(&self, value: T) -> usize {
		let mut vec = self.vec.lock().unwrap();
		let index = vec.len();
		vec.push(Box::pin(value));
		index
	}

	pub fn len(&self) -> usize {
		self.vec.lock().unwrap().len()
	}
}

impl<T> std::ops::Index<usize> for StableVec<T> {
	type Output = T;
	fn index(&self, index: usize) -> &Self::Output {
		let vec = self.vec.lock().unwrap();
		let r: &T = &vec[index];
		// SAFETY: `r` remains pinned for the lifetime of `self`.
		unsafe { std::mem::transmute(r) }
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent2d {
	pub width: u32,
	pub height: u32,
}

impl Default for Extent2d {
	fn default() -> Self {
		Self {
			width: 1,
			height: 1,
		}
	}
}

impl Extent2d {
	fn with_array_layers(self, array_layers: u32) -> wgpu::Extent3d {
		Extent3d {
			width: self.width,
			height: self.height,
			depth_or_array_layers: array_layers,
		}
	}
}

#[derive(Debug, Clone)]
pub struct TextureLayerDescriptor {
	pub size: Extent2d,
	pub mip_level_count: u32,
	pub sample_count: u32,
	pub format: wgpu::TextureFormat,
	pub usage: wgpu::TextureUsages,
	pub view_formats: Vec<wgpu::TextureFormat>,
}

impl Default for TextureLayerDescriptor {
	fn default() -> Self {
		Self {
			size: Default::default(),
			mip_level_count: 1,
			sample_count: 1,
			format: wgpu::TextureFormat::Rgba8Unorm,
			usage: wgpu::TextureUsages::all(),
			view_formats: Default::default(),
		}
	}
}

impl TextureLayerDescriptor {
	fn with_array_layers(&self, block_size: u32) -> wgpu::TextureDescriptor<'_> {
		wgpu::TextureDescriptor {
			label: Some("tile::Block::texture"),
			size: self.size.with_array_layers(block_size),
			mip_level_count: self.mip_level_count,
			sample_count: self.sample_count,
			dimension: wgpu::TextureDimension::D2,
			format: self.format,
			usage: self.usage | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
			view_formats: &self.view_formats,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
struct Index {
	block_index: usize,
	layer_index: u32,
}

#[derive(Default)]
struct FreeList {
	indices: Mutex<Vec<Index>>,
}

impl FreeList {
	fn try_allocate(&self) -> Option<Index> {
		self.indices.lock().unwrap().pop()
	}

	fn release(&self, index: Index) {
		self.indices.lock().unwrap().push(index);
	}
}

struct PoolInternal {
	context: Arc<WgpuContext>,
	blocks: StableVec<Block>,
	free_list: FreeList,
	texture_layer_descriptor: TextureLayerDescriptor,
}

impl PoolInternal {
	fn get_block(&self, block_index: usize) -> &Block {
		&self.blocks[block_index]
	}

	fn release_index(&self, index: Index) {
		self.free_list.release(index)
	}

	pub fn allocate_tile(self: Arc<Self>) -> Tile {
		let index = self.allocate_index();
		Tile::new(self.clone(), index)
	}

	fn allocate_index(&self) -> Index {
		if let Some(index) = self.free_list.try_allocate() {
			return index;
		}

		// TODO: If this were actually multi-threaded, this would be subtly suboptimal because we
		// could add another block between this call to `len()` and pushing the new block.
		let block_index = self.blocks.len();
		let block_size = 1 << (block_index as u32).min(u32::BITS - 1);
		let block_size = block_size.min(self.context.device().limits().max_texture_array_layers);
		assert!(block_size > 0);

		let device = self.context.device();
		let texture =
			device.create_texture(&self.texture_layer_descriptor.with_array_layers(block_size));

		let read_texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
			dimension: Some(wgpu::TextureViewDimension::D2Array),
			..Default::default()
		});

		let data_buffer = BindingBuffer::<[TileData]>::with_capacity(block_size as u64)
			.label("tile::Block::data_buffer")
			.usage(
				// When drawing batches, this is bound to storage.
				wgpu::BufferUsages::STORAGE
				// // When drawing to individual tiles, slices are bound to uniforms.
				// | wgpu::BufferUsages::UNIFORM 
					| wgpu::BufferUsages::COPY_DST
					| wgpu::BufferUsages::COPY_SRC,
			)
			.create(device);

		let read_bind_group = read::BindGroup::from_bindings(
			device,
			read::BindGroupLayout {
				tile_texture: &read_texture_view,
				tile_data: data_buffer.as_entire_buffer_binding(),
			},
		);

		let block = Block {
			texture,
			read_texture_view,
			data_buffer,
			read_bind_group,
		};
		self.blocks.push(block);

		for layer_index in 1..block_size {
			self.free_list.release(Index {
				block_index,
				layer_index,
			})
		}
		Index {
			block_index,
			layer_index: 0,
		}
	}
}

#[derive(Clone)]
pub struct Pool {
	internal: Arc<PoolInternal>,
}

impl Pool {
	pub fn context(&self) -> Arc<WgpuContext> {
		self.internal.context.clone()
	}

	pub fn new(context: Arc<WgpuContext>, texture_layer_descriptor: TextureLayerDescriptor) -> Self {
		Pool {
			internal: PoolInternal {
				context,
				blocks: Default::default(),
				free_list: Default::default(),
				texture_layer_descriptor,
			}
			.into(),
		}
	}

	pub fn buffer_layout(&self) -> wgpu::VertexBufferLayout<'static> {
		read::InstanceInput::vertex_buffer_layout(wgpu::VertexStepMode::Instance)
	}

	pub fn allocate_tile(&self) -> Tile {
		self.internal.clone().allocate_tile()
	}
}

pub struct Tile {
	pool: Arc<PoolInternal>,
	index: Index,
	write_bind_group: write::BindGroup,
	texture_view: wgpu::TextureView,
	layer_index_buffer: BindingBuffer<u32>,
}

#[bon]
impl Tile {
	pub fn new(pool: Arc<PoolInternal>, index: Index) -> Self {
		let layer_index = index.layer_index;
		let block = pool.get_block(index.block_index);
		let texture_descriptor = &pool.texture_layer_descriptor;

		let layer_index_buffer = BindingBuffer::init_sized(&layer_index)
			.label("Tile::layer_index_buffer")
			.usage(wgpu::BufferUsages::UNIFORM)
			.create(&pool.context.device());

		let write_bind_group = write::BindGroup::from_bindings(
			&pool.context.device(),
			write::BindGroupLayout {
				tile_data: block.data_buffer.as_entire_buffer_binding(),
				layer_index: layer_index_buffer.as_entire_buffer_binding(),
			},
		);

		let texture_view = block.texture.create_view(&wgpu::TextureViewDescriptor {
			label: Some("Tile::view"),
			format: Some(texture_descriptor.format),
			dimension: Some(wgpu::TextureViewDimension::D2),
			aspect: wgpu::TextureAspect::All,
			base_mip_level: 0,
			mip_level_count: Some(texture_descriptor.mip_level_count),
			base_array_layer: index.layer_index,
			array_layer_count: Some(1),
		});

		Self {
			pool,
			index,
			write_bind_group,
			texture_view,
			layer_index_buffer,
		}
	}

	fn get_block(&self) -> &Block {
		self.pool.get_block(self.index.block_index)
	}

	#[builder]
	pub fn create_texture_view(
		&self,
		label: Option<&str>,
		format: Option<wgpu::TextureFormat>,
		#[builder(default)] base_mip_level: u32,
		mip_level_count: Option<u32>,
	) -> wgpu::TextureView {
		let block = self.get_block();
		block.texture.create_view(&wgpu::TextureViewDescriptor {
			label,
			format,
			dimension: Some(wgpu::TextureViewDimension::D2),
			aspect: wgpu::TextureAspect::All,
			base_mip_level,
			mip_level_count,
			base_array_layer: self.index.layer_index,
			array_layer_count: Some(1),
		})
	}

	pub fn write_bind_group(&self) -> &write::BindGroup {
		&self.write_bind_group
	}

	pub fn texture_view(&self) -> &wgpu::TextureView {
		&self.texture_view
	}

	fn get_buffer_offset(&self) -> BufferAddress {
		BindingBuffer::<[TileData]>::raw_offset(self.index.layer_index as u64)
	}

	fn get_copy_texture(&self) -> wgpu::ImageCopyTexture<'_> {
		wgpu::ImageCopyTexture {
			texture: &self.get_block().texture,
			mip_level: 0,
			origin: wgpu::Origin3d {
				z: self.index.layer_index,
				..Default::default()
			},
			aspect: wgpu::TextureAspect::All,
		}
	}

	fn context(&self) -> &Arc<WgpuContext> {
		&self.pool.context
	}

	fn device(&self) -> &wgpu::Device {
		&self.context().device()
	}

	fn queue(&self) -> &wgpu::Queue {
		&self.context().queue()
	}

	pub fn set_data(&self, data: &TileData) {
		tracing::trace!(?data, "Tile::set_data");
		self.get_block().data_buffer.write_slice(
			self.queue(),
			self.index.layer_index as u64,
			std::slice::from_ref(data),
		)
	}

	pub fn fill_texture(&self, pixel_data: &[u8]) {
		self.queue().fill_texture_layer(
			&self.get_block().texture,
			pixel_data,
			self.index.layer_index,
		);
	}

	pub fn encode_texture_as_url(&self) -> impl Future<Output = anyhow::Result<String>> {
		crate::debug::encode_texture_layer_as_url(
			self.context(),
			&self.get_block().texture,
			self.index.layer_index,
		)
	}
}

impl Clone for Tile {
	fn clone(&self) -> Self {
		let pool = &self.pool;
		let source_block = self.get_block();
		let destination = pool.clone().allocate_tile();
		let destination_block = destination.get_block();

		let context = &*pool.context;
		let queue = context.queue();
		let mut encoder = context
			.device()
			.create_command_encoder(&wgpu::CommandEncoderDescriptor {
				label: Some("Tile::clone"),
			});
		encoder.copy_buffer_to_buffer(
			&source_block.data_buffer,
			self.get_buffer_offset(),
			&destination_block.data_buffer,
			destination.get_buffer_offset(),
			TileData::SHADER_SIZE.get(),
		);
		encoder.copy_texture_to_texture(
			self.get_copy_texture(),
			destination.get_copy_texture(),
			pool.texture_layer_descriptor.size.with_array_layers(1),
		);
		queue.submit([encoder.finish()]);
		destination
	}
}

impl Drop for Tile {
	fn drop(&mut self) {
		self.pool.release_index(self.index);
	}
}

struct Block {
	texture: wgpu::Texture,
	read_texture_view: wgpu::TextureView,
	data_buffer: BindingBuffer<[TileData]>,
	read_bind_group: read::BindGroup,
}

fn draw_tile_internal(
	render_pass: &mut wgpu::RenderPass,
	vertices: std::ops::Range<u32>,
	pool: &PoolInternal,
	tile_indices: impl IntoIterator<Item = Index>,
) {
	use itertools::Itertools;

	for (block_index, block_tile_indices) in tile_indices
		.into_iter()
		.sorted_by_key(|i| i.block_index)
		.chunk_by(|i| i.block_index)
		.into_iter()
	{
		let block = pool.get_block(block_index);
		block.read_bind_group.set(render_pass);

		let layer_indices = block_tile_indices.map(|i| i.layer_index).collect_vec();
		let instance_input_buffer =
			pool
				.context
				.device()
				.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some("draw_tile::instance_input_buffer"),
					usage: wgpu::BufferUsages::VERTEX,
					contents: bytemuck::cast_slice(&layer_indices),
				});
		render_pass.set_vertex_buffer(0, instance_input_buffer.slice(..));
		render_pass.draw(vertices.clone(), 0..(layer_indices.len() as u32));
	}
}

pub fn draw_tiles(
	render_pass: &mut wgpu::RenderPass,
	vertices: std::ops::Range<u32>,
	tiles: &[&Tile],
) {
	let Some(first) = tiles.first() else { return };
	draw_tile_internal(
		render_pass,
		vertices,
		&first.pool,
		tiles.iter().map(|t| t.index),
	);
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::*;

	use glam::*;
	use itertools::Itertools;
	use shaders::tile_read::InstanceInput;

	#[test]
	fn draw_tiles() -> anyhow::Result<()> {
		let context = test::WgpuTestContext::new()?;

		let texture_layer_descriptor = TextureLayerDescriptor {
			size: Extent2d {
				width: 128,
				height: 128,
			},
			..Default::default()
		};
		let pool = Pool::new(context.clone(), texture_layer_descriptor);

		let tiles = [
			pool.allocate_tile(),
			pool.allocate_tile(),
			pool.allocate_tile(),
		];

		tiles[0].set_data(&TileData {
			chart_to_canvas_scale: Vec2::ONE,
			chart_to_canvas_translation: Vec2::ZERO,
		});
		tiles[0].fill_texture(bytemuck::cast_slice(&[192u8, 64u8, 0u8, 128u8]));

		tiles[1].set_data(&TileData {
			chart_to_canvas_scale: Vec2::ONE,
			chart_to_canvas_translation: vec2(-1f32, 0f32),
		});
		tiles[1].fill_texture(bytemuck::cast_slice(&[128u8, 0u8, 64u8, 192u8]));

		tiles[2].set_data(&TileData {
			chart_to_canvas_scale: Vec2::ONE,
			chart_to_canvas_translation: vec2(0f32, -1f32),
		});
		tiles[2].fill_texture(bytemuck::cast_slice(&[0u8, 64u8, 128u8, 255u8]));

		let device = context.device();

		let chart_sampler = context.device().create_sampler(&wgpu::SamplerDescriptor {
			..Default::default()
		});
		let usage_bind_group = shaders::atlas::bind_groups::BindGroup0::from_bindings(
			device,
			shaders::atlas::bind_groups::BindGroupLayout0 {
				chart_sampler: &chart_sampler,
			},
		);

		let texture_format = wgpu::TextureFormat::Rgba8Unorm;

		let pipeline = shaders::atlas::Shader::new(device)
			.pipeline()
			.vertex_buffer_layouts(&[InstanceInput::vertex_buffer_layout(
				wgpu::VertexStepMode::Instance,
			)])
			.targets([Some(wgpu::ColorTargetState {
				format: texture_format,
				blend: Some(wgpu::BlendState::ALPHA_BLENDING),
				write_mask: wgpu::ColorWrites::ALL,
			})])
			.create(device);

		context.render_golden_commands(
			"engine/tile/draw_tiles",
			test::GoldenOptions {
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
				// https://github.com/gfx-rs/wgpu-rs/blob/master/examples/texture-arrays/main.rs
				render_pass.set_pipeline(&pipeline);
				usage_bind_group.set(&mut render_pass);

				super::draw_tiles(&mut render_pass, 0..4, &tiles.iter().collect_vec())
			},
		)
	}
}
