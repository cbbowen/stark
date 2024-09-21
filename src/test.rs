
use itertools::Itertools;

use crate::*;
use std::io::{Read, Write};
use std::ops::Deref;
use std::sync::Arc;
use crate::util::ImageExt;
use zune_image::image::Image;
use zune_image::codecs::ImageFormat;

pub struct WgpuTestContext {
	context: Arc<WgpuContext>,

	copy_transform: render::Shader,
}

impl Deref for WgpuTestContext {
	type Target = Arc<WgpuContext>;
	fn deref(&self) -> &Arc<WgpuContext> {
		&self.context
	}
}

pub struct GoldenOptions {
	pub texture_format: wgpu::TextureFormat,
	pub width: u32,
	pub height: u32,
	pub quantile: f32,
	pub threshold: f32,
}

impl Default for GoldenOptions {
	fn default() -> Self {
		Self {
			texture_format: wgpu::TextureFormat::Rgba8Unorm,
			width: 128,
			height: 128,
			quantile: 0.99,
			threshold: 0.01,
		}
	}
}

impl WgpuTestContext {
	pub fn new() -> Result<Self, WgpuContextError> {
		let context = pollster::block_on(WgpuContext::new())?;
		let device = context.device();

		let copy_transform = render::Shader {
			module: shaders::copy_transform::create_shader_module(device),
			layout: shaders::copy_transform::create_pipeline_layout(device),
		}
		.into();

		let context = Arc::new(context);
		Ok(Self {
			context,
			copy_transform,
		})
	}

	pub fn create_image_texture(&self, path: &str) -> anyhow::Result<wgpu::Texture> {
		let image = Image::open(path)?;
		let (data, width, height, format) = image.into_texture_data();

		Ok(render::texture()
			.width(width)
			.height(height)
			.format(format)
			.usage(wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::TEXTURE_BINDING)
			.view_formats(&[format.remove_srgb_suffix()])
			.with_data((self.queue(), &data))
			.create(self.device()))
	}

	pub fn copy_texture_to_scaled_texture(
		&self,
		source: &wgpu::Texture,
		destination: &wgpu::Texture,
	) {
		let source_view = source.create_view(&Default::default());
		let destination_view = destination.create_view(&Default::default());

		let device = self.device();
		let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			address_mode_w: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Linear,
			mipmap_filter: wgpu::FilterMode::Linear,
			..Default::default()
		});

		let transform_buffer = render::BindingBuffer::init_sized(&glam::Mat2::IDENTITY).create(device);

		use shaders::copy_transform::*;
		let module = &self.copy_transform.module;
		let pipeline = render::render_pipeline()
			.layout(&self.copy_transform.layout)
			.vertex(wgpu::VertexState {
				module,
				entry_point: ENTRY_VS_MAIN,
				compilation_options: Default::default(),
				buffers: &[],
			})
			.fragment(fragment_state(
				module,
				&fs_main_entry([Some(wgpu::ColorTargetState {
					format: destination.format(),
					blend: Some(wgpu::BlendState::REPLACE),
					write_mask: wgpu::ColorWrites::ALL,
				})]),
			))
			.create(device);
		let bind_group = bind_groups::BindGroup0::from_bindings(
			device,
			bind_groups::BindGroupLayout0 {
				transform: transform_buffer.as_entire_buffer_binding(),
				source_texture: &source_view,
				source_sampler: &sampler,
			},
		);

		let mut command_encoder = device.create_command_encoder(&Default::default());
		{
			let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				color_attachments: &[Some(wgpu::RenderPassColorAttachment {
					view: &destination_view,
					resolve_target: None,
					ops: wgpu::Operations {
						load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
						store: wgpu::StoreOp::Store,
					},
				})],
				..Default::default()
			});
			render_pass.set_pipeline(&pipeline);
			bind_group.set(&mut render_pass);
			render_pass.draw(0..4, 0..1);
		}
		self.queue().submit([command_encoder.finish()]);
	}

	pub fn render_golden_commands(
		&self,
		name: &str,
		options: GoldenOptions,
		action: impl FnOnce(wgpu::TextureView, &mut wgpu::CommandEncoder),
	) -> anyhow::Result<()> {
		let mut command_encoder = self.device().create_command_encoder(&Default::default());
		self.render_golden(name, options, |texture_view| {
			action(texture_view, &mut command_encoder);
			self.queue().submit([command_encoder.finish()]);
		})
	}

	pub fn render_golden(
		&self,
		name: &str,
		options: GoldenOptions,
		action: impl FnOnce(wgpu::TextureView),
	) -> anyhow::Result<()> {
		let format = options.texture_format.add_srgb_suffix();
		let view_format = options.texture_format;
		let width = options.width;
		let height = options.height;
		let device = self.device();
		let texture = render::texture()
			.label("drawing_texture")
			.width(width)
			.height(height)
			.format(format)
			.usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC)
			.view_formats(&[view_format])
			.create(device);
		let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
			format: Some(view_format),
			..Default::default()
		});
		action(texture_view);
		self.golden_texture(name, options, &texture, 0)
	}

	pub fn golden_texture(
		&self,
		name: &str,
		options: GoldenOptions,
		texture: &wgpu::Texture,
		layer_index: u32
	) -> anyhow::Result<()> {
		let data = pollster::block_on(self.get_texture_layer_data(texture, layer_index))?;
		let mut image = Image::from_texture_data(&data, texture.width(), texture.height(), texture.format())?;

		let mut path = std::env::current_dir()?;
		path.extend(["test", "output", name]);
		path.set_extension("png");
		if let Some(parent) = path.parent() {
			std::fs::create_dir_all(parent)?;
		}

		if let Ok(mut file) = std::fs::File::create_new(&path) {
			let data = image.write_to_vec(ImageFormat::PNG)?;
			file.write_all(&data);
			return Ok(());
		}

		let mut golden = Image::open(&path)?;
		assert_eq!(image.dimensions(), golden.dimensions());
		let mut differences = golden.convert_to_f32_subpixels()
			.into_iter()
			.zip_eq(image.convert_to_f32_subpixels())
			.map(|(a, b)| (a - b).abs())
			.collect::<Vec<_>>();
		let quantile_index = (options.quantile * differences.len() as f32).floor() as usize;
		assert!(*differences.select_nth_unstable_by(quantile_index, |l, r| l.total_cmp(r)).1 <= options.threshold);

		Ok(())
	}
}
