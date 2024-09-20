use std::{future::Future, sync::Arc};

use crate::util::DeviceExt as _;

#[derive(Clone, Debug, thiserror::Error)]
pub enum WgpuContextError {
	#[error("request adapter error")]
	RequestAdapterError,

	#[error("request device error {0}")]
	RequestDeviceError(String),
}

static_assertions::assert_impl_all!(WgpuContextError: std::error::Error, Send, Sync);

impl From<wgpu::RequestDeviceError> for WgpuContextError {
	fn from(value: wgpu::RequestDeviceError) -> Self {
		WgpuContextError::RequestDeviceError(format!("{}", value))
	}
}

#[derive(Debug)]
pub struct WgpuContext {
	instance: wgpu::Instance,
	adapter: wgpu::Adapter,
	device: Arc<wgpu::Device>,
	queue: wgpu::Queue,
}

impl WgpuContext {
	#[tracing::instrument(err)]
	pub async fn new() -> Result<Self, WgpuContextError> {
		let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
			flags: wgpu::InstanceFlags::from_build_config().with_env(),
			..Default::default()
		});
		tracing::info!(?instance);

		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions::default())
			.await
			.ok_or(WgpuContextError::RequestAdapterError)?;
		tracing::info!(?adapter);

		let (device, queue) = adapter
			.request_device(
				&wgpu::DeviceDescriptor {
					required_features: wgpu::Features::default()
						| wgpu::Features::INDIRECT_FIRST_INSTANCE,
					..Default::default()
				},
				None,
			)
			.await?;
		tracing::info!(?device);
		let device = Arc::new(device);

		Ok(Self {
			instance,
			adapter,
			device,
			queue,
		})
	}

	pub fn instance(&self) -> &wgpu::Instance {
		&self.instance
	}

	pub fn adapter(&self) -> &wgpu::Adapter {
		&self.adapter
	}

	pub fn device(&self) -> &Arc<wgpu::Device> {
		&self.device
	}

	pub fn queue(&self) -> &wgpu::Queue {
		&self.queue
	}

	pub fn get_buffer_data(
		&self,
		buffer: std::sync::Arc<wgpu::Buffer>,
	) -> impl Future<Output = anyhow::Result<Vec<u8>>> {
		self.device.clone().get_buffer_data(buffer)
	}

	pub fn get_texture_layer_data(
		&self,
		texture: &wgpu::Texture,
		layer_index: u32,
	) -> impl Future<Output = anyhow::Result<Vec<u8>>> {
		let aspect = wgpu::TextureAspect::All;
		let (block_width, block_height) = texture.format().block_dimensions();
		let bytes_per_row =
			texture.format().block_copy_size(Some(aspect)).unwrap() * (texture.width() / block_width);
		let rows_per_image = texture.height() / block_height;
		let row_stride = wgpu::util::align_to(bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);

		let device = self.device().clone();
		let buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label: None,
			size: (row_stride * texture.height()) as wgpu::BufferAddress,
			usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});
		let mut encoder = device.create_command_encoder(&Default::default());
		let mip_level = 0;
		let layer_size = wgpu::Extent3d {
			depth_or_array_layers: 1u32,
			..texture.size()
		};
		encoder.copy_texture_to_buffer(
			wgpu::ImageCopyTexture {
				texture,
				mip_level,
				origin: wgpu::Origin3d {
					x: 0,
					y: 0,
					z: layer_index,
				},
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
			layer_size.mip_level_size(mip_level, texture.dimension()),
		);
		self.queue().submit([encoder.finish()]);

		let buffer = Arc::new(buffer);
		async move {
			Ok(device
				.get_buffer_data(buffer)
				.await?
				.chunks_exact(row_stride as usize)
				.flat_map(|row| &row[..bytes_per_row as usize])
				.copied()
				.collect())
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::*;

	#[test]
	fn clear() -> anyhow::Result<()> {
		let context = test::WgpuTestContext::new()?;
		context.render_golden_commands(
			"wgpu_context/clear",
			test::GoldenOptions::default(),
			|view, encoder| {
				encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
					color_attachments: &[Some(wgpu::RenderPassColorAttachment {
						view: &view,
						resolve_target: None,
						ops: wgpu::Operations {
							load: wgpu::LoadOp::Clear(wgpu::Color {
								r: 0.1,
								g: 0.2,
								b: 0.5,
								a: 1.0,
							}),
							store: wgpu::StoreOp::Store,
						},
					})],
					..Default::default()
				});
			},
		)
	}

	#[test]
	fn create_image_texture() -> anyhow::Result<()> {
		let context = test::WgpuTestContext::new()?;
		let texture = context.create_image_texture("test/input/cs-gray-7f7f7f.png")?;
		context.golden_texture(
			"wgpu_context/create_image_texture",
			test::GoldenOptions::default(),
			&texture,
			0,
		)
	}

	#[test]
	fn copy_texture_to_scaled_texture() -> anyhow::Result<()> {
		let context = test::WgpuTestContext::new()?;
		let source_texture = context.create_image_texture("test/input/cs-gray-7f7f7f.png")?;
		let destination_texture = context.device().create_texture(&wgpu::TextureDescriptor {
			label: None,
			size: wgpu::Extent3d {
				width: 128,
				height: 128,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: source_texture.format(),
			usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
			view_formats: &[],
		});
		context.copy_texture_to_scaled_texture(&source_texture, &destination_texture);
		context.golden_texture(
			"wgpu_context/copy_texture_to_scaled_texture",
			test::GoldenOptions::default(),
			&destination_texture,
			0,
		)
	}
}
