use crate::{shaders::chart::*, util::QueueExt, WgpuContext};
use encase::{internal::WriteInto, CalculateSizeFor, ShaderType};
use std::{cell::RefCell, marker::PhantomData, ops::Deref, rc::Rc};
use wgpu::{BufferAddress, Extent3d};
use std::pin::Pin;

#[derive(Clone)]
struct StableVec<T> {
	vec: RefCell<Vec<Pin<Box<T>>>>,
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
		let mut vec = self.vec.borrow_mut();
		let index = vec.len();
		vec.push(Box::pin(value));
		index
	}

	pub fn len(&self) -> usize {
		self.vec.borrow().len()
	}
}

impl<T> std::ops::Index<usize> for StableVec<T> {
	type Output = T;
	fn index(&self, index: usize) -> &Self::Output {
		let vec = self.vec.borrow();
		let r: &T = vec[index].deref();
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

#[derive(Default, Clone)]
struct FreeList {
	indices: RefCell<Vec<Index>>,
}

impl FreeList {
	fn try_allocate(&self) -> Option<Index> {
		self.indices.borrow_mut().pop()
	}

	fn release(&self, index: Index) {
		self.indices.borrow_mut().push(index);
	}
}

struct PoolInternal<Data> {
	context: Rc<WgpuContext>,
	blocks: StableVec<Block>,
	free_list: FreeList,
	texture_layer_descriptor: TextureLayerDescriptor,
	_data: PhantomData<*const Data>,
}

impl<Data> PoolInternal<Data> {
	fn get_block(&self, block_index: usize) -> &Block {
		&self.blocks[block_index]
	}

	fn release_index(&self, index: Index) {
		self.free_list.release(index)
	}
}

impl<Data: 'static> PoolInternal<Data>
where
	[Data]: CalculateSizeFor,
{
	pub fn allocate_tile(self: Rc<Self>) -> Tile<Data> {
		let index = self.allocate_index();
		Tile::<Data> {
			pool: self.clone(),
			index,
			_data: PhantomData {},
		}
	}

	fn allocate_index(&self) -> Index {
		if let Some(index) = self.free_list.try_allocate() {
			return index;
		}

		let block_index = self.blocks.len();
		let block_size = 1 << (block_index as u32).min(u32::BITS - 1);
		let block_size = block_size.min(self.context.device().limits().max_texture_array_layers);
		assert!(block_size > 0);

		let device = self.context.device();
		let texture =
			device.create_texture(&self.texture_layer_descriptor.with_array_layers(block_size));

		let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
			dimension: Some(wgpu::TextureViewDimension::D2Array),
			..Default::default()
		});

		let data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label: Some("tile::Block::data_buffer"),
			usage: wgpu::BufferUsages::STORAGE
				| wgpu::BufferUsages::COPY_DST
				| wgpu::BufferUsages::COPY_SRC,
			size: <[Data]>::calculate_size_for(block_size as u64).get(),
			mapped_at_creation: false,
		});

		let bind_group = BindGroup::from_bindings(
			device,
			BindGroupLayout {
				chart_texture: &texture_view,
				chart_data: data_buffer.as_entire_buffer_binding(),
			},
		);

		let block = Block {
			texture,
			texture_view,
			data_buffer,
			bind_group,
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
pub struct Pool<Data> {
	internal: Rc<PoolInternal<Data>>,
}

impl<Data> Pool<Data> {
	pub fn context(&self) -> Rc<WgpuContext> {
		self.internal.context.clone()
	}
}

impl<Data: 'static> Pool<Data>
where
	[Data]: CalculateSizeFor,
{
	pub fn new(context: Rc<WgpuContext>, texture_layer_descriptor: TextureLayerDescriptor) -> Self {
		Pool {
			internal: PoolInternal {
				context,
				blocks: Default::default(),
				free_list: Default::default(),
				texture_layer_descriptor,
				_data: Default::default(),
			}
			.into(),
		}
	}

	pub fn allocate_tile(&self) -> Tile<Data> {
		self.internal.clone().allocate_tile()
	}
}

pub struct Tile<Data> {
	pool: Rc<PoolInternal<Data>>,
	index: Index,
	_data: PhantomData<Data>,
}

impl<Data: ShaderType + WriteInto + 'static> Tile<Data>
where
	[Data]: CalculateSizeFor,
{
	fn get_buffer_offset(&self) -> BufferAddress {
		// `calculate_size_for` has surprising semantics for zero-length arrays, so we have to special-case the zero index.
		let layer_index = self.index.layer_index;
		if layer_index == 0 {
			0
		} else {		
			<[Data]>::calculate_size_for(self.index.layer_index as u64).get()
		}
	}

	fn get_block(&self) -> &Block {
		self.pool.get_block(self.index.block_index)
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

	pub fn set_data(&self, data: &Data) {
		let mut contents = encase::UniformBuffer::new(Vec::<u8>::new());
		contents.write(&data).unwrap();

		let pool = self.pool.deref();
		let context = &pool.context;
		let offset = self.get_buffer_offset();

		context.queue().write_buffer(
			&self.get_block().data_buffer,
			offset,
			&contents.into_inner(),
		)
	}

	pub fn fill_texture(&self, pixel_data: &[u8]) {
		let pool = &self.pool;
		pool.context.queue().fill_texture_layer(
			&self.get_block().texture,
			pixel_data,
			self.index.layer_index,
		);
	}
}

impl<Data: ShaderType + WriteInto + 'static> Clone for Tile<Data>
where
	[Data]: CalculateSizeFor,
{
	fn clone(&self) -> Self {
		let pool = &self.pool;
		let source_block = self.get_block();
		let destination = pool.clone().allocate_tile();
		let destination_block = destination.get_block();

		let context = pool.context.deref();
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
			<[Data]>::calculate_size_for(1).get(),
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

impl<Data> Drop for Tile<Data> {
	fn drop(&mut self) {
		let pool = self.pool.deref();
		pool.release_index(self.index);
	}
}

struct Block {
	texture: wgpu::Texture,
	texture_view: wgpu::TextureView,
	data_buffer: wgpu::Buffer,
	bind_group: BindGroup,
}

fn draw_tile_internal<Data>(
	render_pass: &mut wgpu::RenderPass,
	vertices: std::ops::Range<u32>,
	pool: &PoolInternal<Data>,
	tile_indices: impl IntoIterator<Item = Index>,
) {
	use itertools::Itertools;
	use wgpu::util::DeviceExt;

	for (block_index, block_tile_indices) in tile_indices
		.into_iter()
		.sorted_by_key(|i| i.block_index)
		.chunk_by(|i| i.block_index)
		.into_iter()
	{
		let block = pool.get_block(block_index);
		block.bind_group.set(render_pass);

		let layer_indices = block_tile_indices.map(|i| i.layer_index).collect_vec();
		let instance_input_buffer =
			pool
				.context
				.device()
				.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some("draw_tile::instance_input_buffer"),
					contents: bytemuck::cast_slice(&layer_indices),
					usage: wgpu::BufferUsages::VERTEX,
				});
		render_pass.set_vertex_buffer(0, instance_input_buffer.slice(..));
		render_pass.draw(vertices.clone(), 0..(layer_indices.len() as u32));
	}
}

pub fn draw_tiles<Data>(
	render_pass: &mut wgpu::RenderPass,
	vertices: std::ops::Range<u32>,
	tiles: &[&Tile<Data>],
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
	use itertools::Itertools;

	use super::*;
	use crate::*;

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

		tiles[0].set_data(&ChartData {
			chart_to_canvas: glam::Mat4::IDENTITY,
		});
		tiles[0].fill_texture(bytemuck::cast_slice(&[192u8, 64u8, 0u8, 128u8]));

		tiles[1].set_data(&ChartData {
			chart_to_canvas: glam::Mat4::from_translation(glam::Vec3::new(-1f32, 0f32, 0f32)),
		});
		tiles[1].fill_texture(bytemuck::cast_slice(&[128u8, 0u8, 64u8, 192u8]));

		tiles[2].set_data(&ChartData {
			chart_to_canvas: glam::Mat4::from_translation(glam::Vec3::new(0f32, -1f32, 0f32)),
		});
		tiles[2].fill_texture(bytemuck::cast_slice(&[0u8, 64u8, 128u8, 255u8]));

		let device = context.device();
		let module = shaders::atlas::create_shader_module(device);
		let layout = shaders::atlas::create_pipeline_layout(device);

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
		let pipeline = context
			.device()
			.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
				label: None,
				layout: Some(&layout),
				vertex: wgpu::VertexState {
					module: &module,
					entry_point: shaders::atlas::ENTRY_VS_MAIN,
					compilation_options: Default::default(),
					buffers: &[InstanceInput::vertex_buffer_layout(
						wgpu::VertexStepMode::Instance,
					)],
				},
				fragment: Some(shaders::atlas::fragment_state(
					&module,
					&shaders::atlas::fs_main_entry([Some(wgpu::ColorTargetState {
						format: texture_format,
						blend: Some(wgpu::BlendState::ALPHA_BLENDING),
						write_mask: wgpu::ColorWrites::ALL,
					})]),
				)),
				primitive: wgpu::PrimitiveState {
					topology: wgpu::PrimitiveTopology::TriangleStrip,
					strip_index_format: None,
					front_face: wgpu::FrontFace::Ccw,
					cull_mode: Some(wgpu::Face::Back),
					polygon_mode: wgpu::PolygonMode::Fill,
					unclipped_depth: false,
					conservative: false,
				},
				depth_stencil: None,
				multisample: wgpu::MultisampleState::default(),
				multiview: None,
				cache: None,
			});

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
