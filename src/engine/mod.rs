// mod stroke;
// pub use stroke::*;

// mod atlas;
// pub use atlas::*;

use std::num::NonZeroU32;
use std::{cell::RefCell, rc::Rc};

use wgpu::Extent3d;

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
pub struct ChartDescriptor {
	texture: ChartTextureDescriptor,
	instance_data: ChartInstanceDataDescriptor,
}

struct ChartPoolBlock {
	texture: wgpu::Texture,
	texture_view: wgpu::TextureView,
	instance_data_buffer: wgpu::Buffer,
	bind_group: Rc<crate::shaders::atlas::bind_groups::BindGroup1>,
}

impl ChartPoolBlock {
	pub fn new(
		context: &WgpuContext,
		descriptor: &ChartDescriptor,
		// bind_group_layout: &wgpu::BindGroupLayout,
		block_size: NonZeroU32,
	) -> Self {
		let device = context.device();
		let texture = device
			.create_texture(&descriptor.texture.to_texture_descriptor(block_size.get()));

		let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
			dimension: Some(wgpu::TextureViewDimension::D2Array),
			..Default::default()
		});
		let instance_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label: None,
			size: block_size.get() as wgpu::BufferAddress * descriptor.instance_data.array_stride,
			usage: wgpu::BufferUsages::VERTEX
				| wgpu::BufferUsages::COPY_DST
				| wgpu::BufferUsages::COPY_SRC,
			mapped_at_creation: false,
		});
		// let bind_group = device
		// 	.create_bind_group(&wgpu::BindGroupDescriptor {
		// 		label: None,
		// 		layout: bind_group_layout,
		// 		entries: &[wgpu::BindGroupEntry {
		// 			binding: 0,
		// 			// TODO: We could do this with a single bind group by using a `TextureViewArray`.
		// 			resource: wgpu::BindingResource::TextureView(&texture_view),
		// 		}],
		// 	});
		use crate::shaders::atlas::bind_groups::*;
		let bind_group = BindGroup1::from_bindings(device, BindGroupLayout1 {
			chart_texture: &texture_view,
		});
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

pub struct ChartPool {
	context: Rc<WgpuContext>,
	descriptor: ChartDescriptor,
	blocks: RefCell<Vec<ChartPoolBlock>>,
	free_list: RefCell<Vec<ChartPoolIndex>>,
	// bind_group_layout: wgpu::BindGroupLayout,
}

impl ChartPool {
	pub fn new(context: Rc<WgpuContext>, descriptor: ChartDescriptor) -> Self {
		// let bind_group_layout =
		// 	context
		// 		.device()
		// 		.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		// 			label: None,
		// 			entries: &[wgpu::BindGroupLayoutEntry {
		// 				binding: 0,
		// 				visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
		// 				// TODO: We could do this with a single bind group by using an array of
		// 				// textures.
		// 				count: None,
		// 				ty: wgpu::BindingType::Texture {
		// 					sample_type: wgpu::TextureSampleType::Float { filterable: true },
		// 					view_dimension: wgpu::TextureViewDimension::D2Array,
		// 					multisampled: false,
		// 				},
		// 			}],
		// 		});
		Self {
			context,
			descriptor,
			blocks: Default::default(),
			free_list: Default::default(),
			// bind_group_layout,
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
			&self.descriptor,
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

	pub fn bind_group(&self, block_index: usize) -> Rc<crate::shaders::atlas::bind_groups::BindGroup1> {
		self.blocks.borrow()[block_index].bind_group.clone()
	}

	// pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
	// 	&self.bind_group_layout
	// }
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::*;

	#[test]
	fn chart_pool() -> anyhow::Result<()> {
		let context = test::WgpuTestContext::new()?;

		let chart_descriptor = ChartDescriptor {
			texture: ChartTextureDescriptor {
				size: Extent2d {
					width: 128,
					height: 128,
				},
				..Default::default()
			},
			..Default::default()
		};
		let mut pool = Rc::new(ChartPool::new(context.clone(), chart_descriptor));
		let chart = pool.allocate();

		let test_texture = context.create_image_texture("test/input/cs-gray-7f7f7f.png");

		let device = context.device();
		let module = shaders::atlas::create_shader_module(device);
		let layout = shaders::atlas::create_pipeline_layout(device);

		let chart_sampler = context.device().create_sampler(&wgpu::SamplerDescriptor {
			..Default::default()
		});

		use shaders::atlas::bind_groups::*;
		let bind_group0 = BindGroup0::from_bindings(
			device,
			BindGroupLayout0 {
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
					buffers: &[],
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
				let bind_group1 = pool.bind_group(0);
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
				bind_group0.set(&mut render_pass);
				bind_group1.set(&mut render_pass);
				// render_pass.set_vertex_buffer(0, buffer_slice);
				render_pass.draw(0..4, 0..1);
			},
		)
	}
}
