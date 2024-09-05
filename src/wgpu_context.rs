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
	device: wgpu::Device,
	queue: wgpu::Queue,
}

impl WgpuContext {
	#[tracing::instrument(err)]
	pub async fn new() -> Result<Self, WgpuContextError> {
		let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
			flags: wgpu::InstanceFlags::from_build_config().with_env(),
			..Default::default()
		});

		let adapter = instance
			.request_adapter(&Default::default())
			.await
			.ok_or(WgpuContextError::RequestAdapterError)?;

		let (device, queue) = adapter
			.request_device(
				&wgpu::DeviceDescriptor {
					required_features: wgpu::Features::default() | wgpu::Features::INDIRECT_FIRST_INSTANCE |
						wgpu::Features::MULTIVIEW,
					..Default::default()
				},
				None,
			)
			.await?;

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

	pub fn device(&self) -> &wgpu::Device {
		&self.device
	}

	pub fn queue(&self) -> &wgpu::Queue {
		&self.queue
	}
}

#[cfg(test)]
mod tests {
	use std::borrow::Borrow;

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
		let texture =
			context.create_image_texture("test/output/wgpu_context/create_image_texture.png")?;
		context.golden_texture(
			"wgpu_context/create_image_texture",
			test::GoldenOptions::default(),
			&texture,
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
		)
	}
}
