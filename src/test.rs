use wgpu::util::DeviceExt;
use wgpu::Extent3d;

use crate::*;
use std::io::Read;
use std::ops::Deref;
use std::rc::Rc;

pub struct WgpuTestContext {
	context: Rc<WgpuContext>,

	copy_scaled_pipeline_factory: render::PipelineFactory,
}

impl Deref for WgpuTestContext {
	type Target = Rc<WgpuContext>;
	fn deref(&self) -> &Rc<WgpuContext> {
		&self.context
	}
}

pub struct GoldenOptions {
	pub texture_format: wgpu::TextureFormat,
	pub width: u32,
	pub height: u32,
	pub quantile: f32,
	pub threshold: i16,
}

impl Default for GoldenOptions {
	fn default() -> Self {
		Self {
			texture_format: wgpu::TextureFormat::Rgba8Unorm,
			width: 128,
			height: 128,
			quantile: 0.99,
			threshold: 1,
		}
	}
}

impl WgpuTestContext {
	pub fn barrier(&self) {
		self
			.device()
			.poll(wgpu::Maintain::wait())
			.panic_on_timeout()
	}

	pub fn new() -> Result<Self, WgpuContextError> {
		let context = pollster::block_on(WgpuContext::new())?;

		let copy_scaled_pipeline_factory =
			render::PipelineFactoryBuilder::new("copy_scaled", include_str!("copy_scaled.wgsl"))
				.add_group(
					render::BindGroupLayoutBuilder::new()
						.add_entry(render::BindGroupLayoutEntryBuilder::new(
							"source_texture",
							wgpu::ShaderStages::FRAGMENT,
							Rc::new(render::Texture2f2BuildBindingType::default()),
						))
						.add_entry(render::BindGroupLayoutEntryBuilder::new(
							"source_sampler",
							wgpu::ShaderStages::FRAGMENT,
							Rc::new(render::SamplerBuildBindingType::default()),
						)),
				)
				.build(context.device());

		let context = Rc::new(context);
		Ok(Self {
			context,
			copy_scaled_pipeline_factory,
		})
	}

	pub fn create_image_texture(&self, path: &str) -> anyhow::Result<wgpu::Texture> {
		let mut buffer = Vec::new();
		std::fs::File::open(path)?.read_to_end(&mut buffer)?;
		let buffer = image::load_from_memory(&buffer)?.to_rgba8();

		let format = wgpu::TextureFormat::Rgba8UnormSrgb;
		Ok(self.device().create_texture_with_data(
			self.queue(),
			&wgpu::TextureDescriptor {
				label: None,
				size: wgpu::Extent3d {
					width: buffer.width(),
					height: buffer.height(),
					depth_or_array_layers: 1,
				},
				mip_level_count: 1,
				sample_count: 1,
				dimension: wgpu::TextureDimension::D2,
				format,
				usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::TEXTURE_BINDING,
				view_formats: &[format.remove_srgb_suffix()],
			},
			wgpu::util::TextureDataOrder::default(),
			&buffer,
		))
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
			min_filter: wgpu::FilterMode::Nearest,
			mipmap_filter: wgpu::FilterMode::Nearest,
			..Default::default()
		});

		let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: None,
			layout: Some(self.copy_scaled_pipeline_factory.layout()),
			vertex: wgpu::VertexState {
				module: self.copy_scaled_pipeline_factory.module(),
				entry_point: "vs_main",
				compilation_options: Default::default(),
				buffers: &[],
			},
			fragment: Some(wgpu::FragmentState {
				module: self.copy_scaled_pipeline_factory.module(),
				entry_point: "fs_main",
				compilation_options: Default::default(),
				targets: &[Some(wgpu::ColorTargetState {
					format: destination.format(),
					blend: Some(wgpu::BlendState::REPLACE),
					write_mask: wgpu::ColorWrites::ALL,
				})],
			}),
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
		let bind_group = self.copy_scaled_pipeline_factory.bind_group_factories()[0].create(device, &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&source_view),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(&sampler),
				},
			]);

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
			render_pass.set_bind_group(0, &bind_group, &[]);
			render_pass.draw(0..4, 0..1);
		}
		self.queue().submit([command_encoder.finish()]);
	}

	fn get_buffer_data(&self, buffer: &wgpu::Buffer) -> Vec<u8> {
		let slice = buffer.slice(..);
		slice.map_async(wgpu::MapMode::Read, |_| ());
		self.barrier();
		slice.get_mapped_range().to_vec()
	}

	fn get_texture_data(&self, texture: &wgpu::Texture) -> Vec<u8> {
		let aspect = wgpu::TextureAspect::All;
		let (block_width, block_height) = texture.format().block_dimensions();
		let bytes_per_row =
			texture.format().block_copy_size(Some(aspect)).unwrap() * (texture.width() / block_width);
		let rows_per_image = texture.height() / block_height;
		let row_stride = wgpu::util::align_to(bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);

		let device = self.device();
		let buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label: None,
			size: (row_stride * texture.height()) as wgpu::BufferAddress,
			usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});
		let mut encoder = device.create_command_encoder(&Default::default());
		let mip_level = 0;
		encoder.copy_texture_to_buffer(
			wgpu::ImageCopyTexture {
				texture,
				mip_level,
				origin: wgpu::Origin3d::ZERO,
				aspect,
			},
			wgpu::ImageCopyBuffer {
				buffer: &buffer,
				layout: wgpu::ImageDataLayout {
					offset: 0,
					bytes_per_row: Some(row_stride),
					rows_per_image: Some(rows_per_image),
				},
			},
			texture
				.size()
				.mip_level_size(mip_level, texture.dimension()),
		);
		self.queue().submit([encoder.finish()]);
		self
			.get_buffer_data(&buffer)
			.chunks_exact(row_stride as usize)
			.flat_map(|row| &row[..bytes_per_row as usize])
			.copied()
			.collect()
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
		let texture = device.create_texture(&wgpu::TextureDescriptor {
			size: wgpu::Extent3d {
				width,
				height,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format,
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
			label: Some("drawing_texture"),
			view_formats: &[view_format],
		});
		let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
			format: Some(view_format),
			..Default::default()
		});
		action(texture_view);
		self.golden_texture(name, options, &texture)
	}

	pub fn golden_texture(
		&self,
		name: &str,
		options: GoldenOptions,
		texture: &wgpu::Texture,
	) -> anyhow::Result<()> {
		let data = self.get_texture_data(texture);
		let width = texture.width();
		let height = texture.height();

		let mut path = std::env::current_dir()?;
		path.extend(["test", "output", name]);
		path.set_extension("png");
		if let Some(parent) = path.parent() {
			std::fs::create_dir_all(parent)?;
		}

		if let Ok(file) = std::fs::File::create_new(path.as_path()) {
			let file = std::io::BufWriter::new(file);
			let mut encoder = png::Encoder::new(file, width, height);
			encoder.set_color(png::ColorType::Rgba);
			encoder.set_depth(png::BitDepth::Eight);
			encoder.set_srgb(png::SrgbRenderingIntent::AbsoluteColorimetric);
			encoder.set_compression(png::Compression::Best);
			let mut writer = encoder.write_header().unwrap();
			writer.write_image_data(&data)?;
			Ok(())
		} else {
			let golden_decoder = png::Decoder::new(std::fs::File::open(path)?);
			let mut golden_reader = golden_decoder.read_info()?;
			let mut golden_data = vec![0; golden_reader.output_buffer_size()];
			let golden_info = golden_reader.next_frame(&mut golden_data)?;
			assert_eq!(golden_info.width, width);
			assert_eq!(golden_info.height, height);
			assert_eq!(golden_info.color_type, png::ColorType::Rgba);
			assert_eq!(golden_info.bit_depth, png::BitDepth::Eight);
			assert_eq!(golden_data.len(), data.len());

			let mut differences = golden_data
				.iter()
				.zip(data.iter())
				.map(|(a, b)| (*a as i16 - *b as i16).abs())
				.collect::<Vec<_>>();
			let quantile_index = (options.quantile * differences.len() as f32).floor() as usize;
			assert!(*differences.select_nth_unstable(quantile_index).1 <= options.threshold);

			Ok(())
		}
	}
}
