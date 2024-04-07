use std::error::Error;

use super::wrappers::*;
use tracing::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Context {
	pub instance: WgpuInstance,
	pub adapter: WgpuAdapter,
	pub device: WgpuDevice,
	pub queue: WgpuQueue,
}

#[derive(Clone, Debug, thiserror::Error, )]
pub enum ContextError {
	#[error("request adapter error")]
	RequestAdapterError,

	#[error("request device error {0}")]
	RequestDeviceError(String),
}

impl From<wgpu::RequestDeviceError> for ContextError {
	fn from(value: wgpu::RequestDeviceError) -> Self {
		ContextError::RequestDeviceError(
			format!("{}", value)
		)
	}
}

static_assertions::assert_impl_all!(ContextError: std::error::Error, Send, Sync);
static_assertions::assert_impl_all!(Result<(), ContextError>: leptos::IntoView);

#[derive(Debug, thiserror::Error)]
#[error("no supported devices")]
struct NoSupportedDevices(String);

impl Context {
	#[tracing::instrument(err)]
	pub async fn new() -> Result<Self, ContextError> {
		use ContextError::*;

		let instance_descriptor = wgpu::InstanceDescriptor {
			backends: wgpu::Backends::all(),
			#[cfg(debug_assertions)]
			flags: wgpu::InstanceFlags::DEBUG | wgpu::InstanceFlags::VALIDATION,
			..Default::default()
		};

		let instance = wgpu::Instance::new(instance_descriptor);

		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions::default())
			.await
			.ok_or(RequestAdapterError)?;

		let device_descriptor = wgpu::DeviceDescriptor {
			label: None,
			required_features: wgpu::Features::SHADER_F16,
			required_limits: if cfg!(target_arch = "wasm32") {
				wgpu::Limits::downlevel_webgl2_defaults()
			} else {
				wgpu::Limits::default()
			},
		};

		let (device, queue) = adapter.request_device(&device_descriptor, None).await?;

		Ok(Self {
			instance: wgpu_wrapper(instance),
			adapter: wgpu_wrapper(adapter),
			device: wgpu_wrapper(device),
			queue: wgpu_wrapper(queue),
		})
	}
}
