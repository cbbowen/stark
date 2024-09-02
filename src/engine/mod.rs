// mod stroke;
// pub use stroke::*;

// mod atlas;
// pub use atlas::*;

use std::num::NonZeroU32;
use std::{cell::RefCell, rc::Rc};

use wgpu::util::DeviceExt;
use wgpu::{BufferAddress, Extent3d};

use crate::WgpuContext;

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
pub struct ChartTextureDescriptor {
	pub size: Extent2d,
	pub mip_level_count: u32,
	pub sample_count: u32,
	pub format: wgpu::TextureFormat,
	pub usage: wgpu::TextureUsages,
	pub view_formats: Vec<wgpu::TextureFormat>,
}

impl Default for ChartTextureDescriptor {
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

impl ChartTextureDescriptor {
	fn to_texture_descriptor(&self, block_size: u32) -> wgpu::TextureDescriptor<'_> {
		wgpu::TextureDescriptor {
			label: None,
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

#[derive(Debug, Clone, Default)]
pub struct ChartInstanceDataDescriptor {
	pub array_stride: wgpu::BufferAddress,
	pub attributes: Vec<wgpu::VertexAttribute>,
}

impl ChartInstanceDataDescriptor {
	fn to_buffer_layout(&self) -> wgpu::VertexBufferLayout<'_> {
		wgpu::VertexBufferLayout {
			array_stride: self.array_stride,
			step_mode: wgpu::VertexStepMode::Instance,
			attributes: &self.attributes,
		}
	}
}

#[derive(Debug, Clone, Default)]
pub struct TypedChartDescriptor<Data> {
	pub texture: ChartTextureDescriptor,
	pub default_data: Data,
}

trait ChartDescriptor {
	fn to_texture_descriptor(&self, block_size: u32) -> wgpu::TextureDescriptor<'_>;
	fn create_instance_data_buffer(&self, device: &wgpu::Device, block_size: u32) -> wgpu::Buffer;
	fn instance_data_stride(&self) -> wgpu::BufferAddress;
}

impl<Data: encase::ShaderSize + encase::internal::WriteInto> ChartDescriptor
	for TypedChartDescriptor<Data>
{
	fn to_texture_descriptor(&self, block_size: u32) -> wgpu::TextureDescriptor<'_> {
		self.texture.to_texture_descriptor(block_size)
	}

	fn create_instance_data_buffer(&self, device: &wgpu::Device, block_size: u32) -> wgpu::Buffer {
		let mut datum = encase::UniformBuffer::new(Vec::<u8>::new());
		datum.write(&self.default_data).unwrap();
		let data = datum.into_inner().repeat(block_size as usize);

		device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: None,
			contents: &data,
			usage: wgpu::BufferUsages::STORAGE
				| wgpu::BufferUsages::COPY_DST
				| wgpu::BufferUsages::COPY_SRC,
		})
	}

	fn instance_data_stride(&self) -> wgpu::BufferAddress {
		 Data::min_size().get()
	}
}

struct ChartPoolBlock {
	texture: wgpu::Texture,
	texture_view: wgpu::TextureView,
	instance_data_buffer: Rc<wgpu::Buffer>,
	bind_group: Rc<crate::shaders::atlas::bind_groups::BindGroup1>,
}

impl ChartPoolBlock {
	pub fn new(
		context: &WgpuContext,
		descriptor: &dyn ChartDescriptor,
		// bind_group_layout: &wgpu::BindGroupLayout,
		block_size: NonZeroU32,
	) -> Self {
		let device = context.device();
		let texture = device.create_texture(&descriptor.to_texture_descriptor(block_size.get()));

		let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
			dimension: Some(wgpu::TextureViewDimension::D2Array),
			..Default::default()
		});
		let instance_data_buffer = descriptor.create_instance_data_buffer(device, block_size.get());
		use crate::shaders::atlas::bind_groups::*;
		let bind_group = BindGroup1::from_bindings(
			device,
			BindGroupLayout1 {
				chart_texture: &texture_view,
				chart_data: instance_data_buffer.as_entire_buffer_binding(),
			},
		);

		let instance_data_buffer = Rc::new(instance_data_buffer);
		let bind_group = Rc::new(bind_group);
		Self {
			texture,
			texture_view,
			instance_data_buffer,
			bind_group,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChartPoolIndex {
	block_index: usize,
	layer_index: u32,
}

impl ChartPoolIndex {
	pub fn new(block_index: usize, layer_index: u32) -> Self {
		Self {
			block_index,
			layer_index,
		}
	}
}

pub struct ChartPool {
	context: Rc<WgpuContext>,
	descriptor: Box<dyn ChartDescriptor>,
	blocks: RefCell<Vec<ChartPoolBlock>>,
	free_list: RefCell<Vec<ChartPoolIndex>>,
}

impl ChartPool {
	pub fn new(context: Rc<WgpuContext>, descriptor: Box<dyn ChartDescriptor>) -> Self {
		Self {
			context,
			descriptor,
			blocks: Default::default(),
			free_list: Default::default(),
		}
	}

	fn allocate_index(&self) -> ChartPoolIndex {
		let mut free_list = self.free_list.borrow_mut();

		if let Some(index) = free_list.pop() {
			return index;
		}

		let mut blocks = self.blocks.borrow_mut();
		let block_index = blocks.len();
		let block_size = 1 << (block_index as u32).min(u32::BITS - 1);
		let block_size = block_size.min(self.context.device().limits().max_texture_array_layers);
		let block_size = NonZeroU32::new(block_size).unwrap();
		let block = ChartPoolBlock::new(
			&self.context,
			self.descriptor.as_ref(),
			// &self.bind_group_layout,
			block_size,
		);
		blocks.push(block);

		for layer_index in 1..block_size.get() {
			free_list.push(ChartPoolIndex {
				block_index,
				layer_index,
			})
		}
		ChartPoolIndex {
			block_index,
			layer_index: 0,
		}
	}

	pub fn allocate(self: &Rc<ChartPool>) -> Chart {
		Chart {
			pool: self.clone(),
			pool_index: self.allocate_index(),
		}
	}

	fn release(&self, index: ChartPoolIndex) {
		self.free_list.borrow_mut().push(index)
	}

	pub fn bind_group(
		&self,
		block_index: usize,
	) -> Rc<crate::shaders::atlas::bind_groups::BindGroup1> {
		self.blocks.borrow()[block_index].bind_group.clone()
	}

	pub fn instance_data_buffer(&self, block_index: usize) -> Rc<wgpu::Buffer> {
		self.blocks.borrow()[block_index]
			.instance_data_buffer
			.clone()
	}
}

pub struct Chart {
	pool: Rc<ChartPool>,
	pool_index: ChartPoolIndex,
}

impl Drop for Chart {
	fn drop(&mut self) {
		self.pool.release(self.pool_index)
	}
}

impl Chart {
	pub fn instance_data_buffer(&self) -> (Rc<wgpu::Buffer>, BufferAddress) {
		let buffer = self.pool.instance_data_buffer(self.pool_index.block_index);
		let stride = self.pool.descriptor.instance_data_stride();
		let offset = stride * self.pool_index.layer_index as u64;
		(buffer, offset)
	}

	pub fn fill_texture(&self, pixel_data: &[u8]) {
		let pool = &self.pool;
		let blocks = pool.blocks.borrow();
		let block = blocks.get(self.pool_index.block_index).unwrap();
		pool.context.queue().fill_texture_layer(
			&block.texture,
			pixel_data,
			self.pool_index.layer_index,
		);
	}
}

trait QueueExt {
	fn fill_texture_layer(&self, texture: &wgpu::Texture, pixel_data: &[u8], layer_index: u32);
	fn fill_texture(&self, texture: &wgpu::Texture, pixel_data: &[u8]) {
		self.fill_texture_layer(texture, pixel_data, 0)
	}
}

impl QueueExt for wgpu::Queue {
	fn fill_texture_layer(&self, texture: &wgpu::Texture, pixel_data: &[u8], layer_index: u32) {
		let size = texture.size();
		let texture_data = pixel_data.repeat((size.width * size.height) as usize);
		self.write_texture(
			wgpu::ImageCopyTexture {
				mip_level: 0,
				origin: wgpu::Origin3d {
					z: layer_index,
					..Default::default()
				},
				texture,
				aspect: wgpu::TextureAspect::All,
			},
			&texture_data,
			wgpu::ImageDataLayout {
				offset: 0,
				bytes_per_row: Some(pixel_data.len() as u32 * size.width),
				rows_per_image: None,
			},
			Extent3d {
				depth_or_array_layers: 1,
				..size
			},
		)
	}
}

#[cfg(test)]
mod tests {
	use wgpu::util::DeviceExt;

	use super::*;
	use crate::*;

	#[test]
	fn chart_pool() -> anyhow::Result<()> {
		let context = test::WgpuTestContext::new()?;

		let chart_descriptor = TypedChartDescriptor {
			texture: ChartTextureDescriptor {
				size: Extent2d {
					width: 128,
					height: 128,
				},
				..Default::default()
			},
			default_data: shaders::atlas::ChartData {
				chart_to_canvas: glam::Mat4::IDENTITY
			},
		};
		let pool = Rc::new(ChartPool::new(context.clone(), Box::new(chart_descriptor)));

		let chart = pool.allocate();
		chart.fill_texture(bytemuck::cast_slice(&[192u8, 64u8, 0u8, 128u8]));
		assert_eq!(chart.pool_index, ChartPoolIndex::new(0, 0));

		let chart = pool.allocate();
		chart.fill_texture(bytemuck::cast_slice(&[128u8, 0u8, 64u8, 192u8]));
		assert_eq!(chart.pool_index, ChartPoolIndex::new(1, 0));

		let chart = pool.allocate();
		chart.fill_texture(bytemuck::cast_slice(&[0u8, 64u8, 128u8, 255u8]));
		assert_eq!(chart.pool_index, ChartPoolIndex::new(1, 1));

		// Populate instance data buffer.
		{
			let mut contents = encase::UniformBuffer::new(Vec::<u8>::new());
			contents.write(&[
				shaders::atlas::ChartData {
					chart_to_canvas: glam::Mat4::from_translation(glam::Vec3::new(-1f32, 0f32, 0f32))
				},
				shaders::atlas::ChartData {
					chart_to_canvas: glam::Mat4::from_translation(glam::Vec3::new(0f32, -1f32, 0f32))
				},
			]).unwrap();
			context.queue().write_buffer(
				&pool.instance_data_buffer(1),
				0,
				&contents.into_inner(),
			);
		}

		let device = context.device();
		let module = shaders::atlas::create_shader_module(device);
		let layout = shaders::atlas::create_pipeline_layout(device);

		let chart_sampler = context.device().create_sampler(&wgpu::SamplerDescriptor {
			..Default::default()
		});

		use shaders::atlas::bind_groups::*;
		let usage_bind_group = BindGroup0::from_bindings(
			device,
			BindGroupLayout0 {
				chart_sampler: &chart_sampler,
			},
		);

		let instance_input_buffer_layout =
			shaders::chart::InstanceInput::vertex_buffer_layout(wgpu::VertexStepMode::Instance);
		let instance_input_buffer =
			context
				.device()
				.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some("instance_input_buffer"),
					contents: bytemuck::cast_slice(&[0u32, 1u32]),
					usage: wgpu::BufferUsages::VERTEX,
				});

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
					buffers: &[instance_input_buffer_layout],
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
			"engine/chart_pool",
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

				render_pass.set_vertex_buffer(0, instance_input_buffer.slice(..));

				pool.bind_group(0).set(&mut render_pass);
				render_pass.draw(0..4, 0..1);

				pool.bind_group(1).set(&mut render_pass);
				render_pass.draw(0..4, 0..2);
			},
		)
	}
}
