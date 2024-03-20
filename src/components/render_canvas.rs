use crate::util::*;
use leptos::*;
use leptos_use::use_throttle_fn_with_arg;
use std::{fmt::Debug, ops::Deref, rc::Rc};
use tracing::{error, info, trace};
use wasm_bindgen::JsCast;

pub struct RenderContext {
	instance: wgpu::Instance,
	adapter: AsyncOnceSignal<Rc<RenderAdapter>>,
}

#[derive(Debug)]
pub struct RenderAdapter {
	adapter: wgpu::Adapter,
	device: wgpu::Device,
	queue: wgpu::Queue,
}

#[derive(Debug, Clone)]
struct RenderSurface {
	surface: Rc<wgpu::Surface<'static>>,
	adapter: Rc<RenderAdapter>,
	// TODO: There's no reason for this to be a separate `Rc` from `surface`.
	resize: Rc<std::cell::Cell<Option<(u32, u32)>>>,
}

#[derive(thiserror::Error, Debug)]
#[error("unsupported platform")]
struct UnsupportedPlatform;

#[derive(thiserror::Error, Debug)]
#[error("no matching adapters")]
struct NoMatchingAdapaters;

#[derive(thiserror::Error, Debug)]
#[error("unsupported surface")]
struct UnsupportedSurface;

#[derive(thiserror::Error, Debug)]
#[error("no supported devices")]
struct NoSupportedDevices(String);

impl RenderContext {
	pub fn new() -> Self {
		let descriptor = wgpu::InstanceDescriptor {
			backends: wgpu::Backends::all(),
			#[cfg(debug_assertions)]
			flags: wgpu::InstanceFlags::DEBUG | wgpu::InstanceFlags::VALIDATION,
			..Default::default()
		};
		trace!(
			{ descriptor = format!("{:?}", descriptor) },
			"creating wgpu instance"
		);

		let instance = wgpu::Instance::new(descriptor);
		info!(
			{ instance = format!("{:?}", instance) },
			"created wgpu instance"
		);

		Self {
			instance,
			adapter: AsyncOnceSignal::new(),
		}
	}

	pub fn instance(&self) -> &wgpu::Instance {
		&self.instance
	}

	pub fn adapter(&self) -> Option<Rc<RenderAdapter>> {
		self.adapter.get().map(|a| a.clone())
	}

	async fn create_adapter<'a, 'b>(
		&'a self,
		compatible_surface: Option<&'a wgpu::Surface<'b>>,
	) -> anyhow::Result<RenderAdapter> {
		let adapter = self
			.instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				compatible_surface,
				..Default::default()
			})
			.await
			.ok_or(NoMatchingAdapaters)?;

		let device_descriptor = wgpu::DeviceDescriptor {
			label: None,
			required_features: wgpu::Features::SHADER_F16, // wgpu::Features::empty(),
			required_limits: if cfg!(target_arch = "wasm32") {
				wgpu::Limits::downlevel_webgl2_defaults()
			} else {
				wgpu::Limits::default()
			},
		};
		trace!(
			{ device_descriptor = format!("{:?}", device_descriptor) },
			"requesting device"
		);

		let (device, queue) = adapter
			.request_device(
				&device_descriptor,
				None, // Trace path
			)
			.await
			.map_err(|e| NoSupportedDevices(format!("{:?}", e)))?;
		info!({device = format!("{:?}", device), queue = format!("{:?}", queue)}, "requested device");

		Ok(RenderAdapter {
			adapter,
			device,
			queue,
		})
	}

	async fn create_surface(
		&self,
		canvas: web_sys::HtmlCanvasElement,
	) -> anyhow::Result<RenderSurface> {
		#[allow(unused_variables, unused_mut)]
		let mut surface: anyhow::Result<wgpu::Surface> = Err(UnsupportedPlatform.into());

		let width = canvas.client_width();
		let height = canvas.client_height();

		#[cfg(all(target_arch = "wasm32", not(target_os = "emscripten")))]
		{
			surface = Ok(self
				.instance
				.create_surface(wgpu::SurfaceTarget::Canvas(canvas))?);
		}

		#[allow(unreachable_code)]
		let surface = surface?;

		let adapter = self
			.adapter
			.get_or_try_init::<anyhow::Error>(async {
				Ok(Rc::new(self.create_adapter(Some(&surface)).await?))
			})
			.await?
			.clone();

		if !adapter.adapter.is_surface_supported(&surface) {
			return Err(UnsupportedSurface.into());
		}

		let surface = std::rc::Rc::new(surface);
		let surface = RenderSurface {
			surface,
			adapter,
			resize: Rc::new(std::cell::Cell::new(None)),
		};

		// We could instead pass this in as `resize` which would configure the surface on the next render. But in the interest of failing fast, we explicitly call `configure` here.
		surface.configure(width as u32, height as u32).ok_or_log();

		Ok(surface)
	}
}

impl RenderAdapter {
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

impl RenderSurface {
	pub fn adapter(&self) -> &wgpu::Adapter {
		self.adapter.adapter()
	}

	pub fn device(&self) -> &wgpu::Device {
		self.adapter.device()
	}

	pub fn queue(&self) -> &wgpu::Queue {
		self.adapter.queue()
	}

	pub fn resized(&self, width: u32, height: u32) {
		self.resize.set(Some((width, height)));
	}

	#[tracing::instrument(err)]
	pub fn configure(&self, width: u32, height: u32) -> anyhow::Result<()> {
		// let config = self
		// 	.surface
		// 	.get_default_config(self.adapter(), width, height)
		// 	.ok_or(UnsupportedSurface)?;

		let caps = self.surface.get_capabilities(self.adapter());
		let config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			format: wgpu::TextureFormat::Rgba16Float,
			width,
			height,
			desired_maximum_frame_latency: 2,
			present_mode: *caps.present_modes.get(0).unwrap(),
			alpha_mode: wgpu::CompositeAlphaMode::Auto,
			view_formats: vec![],
		};

		trace!(?config);

		self.surface.configure(self.adapter.device(), &config);
		Ok(())
	}

	#[tracing::instrument(skip(renderable))]
	pub fn render(&self, renderable: Renderable) -> Result<(), wgpu::SurfaceError> {
		// tracing::trace!("RenderSurface::render");

		if let Some((width, height)) = self.resize.take() {
			trace!(width, height, "reconfiguring");
			self.configure(width, height).ok_or_log();
		}

		let output = self.surface.get_current_texture()?;

		let view = output
			.texture
			.create_view(&wgpu::TextureViewDescriptor::default());

		renderable(view).ok_or_log();
		output.present();

		Ok(())
	}
}

impl PartialEq for RenderSurface {
	fn eq(&self, other: &Self) -> bool {
		self.surface.global_id() == other.surface.global_id()
	}
}

// TODO: This is probably the wrong interface because a `Renderable` may need to construct device-dependent things (e.g. pipelines).
// One way to resolve this would be to expose the `Device` as a `[Once]Signal` in the context. The caller can then set it up appropriately when the device is set.
pub type Renderable = Rc<dyn Fn(wgpu::TextureView) -> anyhow::Result<()>>;

#[component]
pub fn RenderCanvas(#[prop(into)] renderable: Signal<Renderable>) -> impl IntoView {
	let canvas_ref = leptos::create_node_ref();
	let canvas_element = canvas_ref.mounted_element();

	let render_context: Rc<RenderContext> = expect_context();
	let surface = create_local_resource(DistinctSignal::new(canvas_element), move |el| {
		let ri: Rc<RenderContext> = render_context.clone();
		async move {
			if let Distinct(Some(ref el)) = el {
				let el: web_sys::HtmlCanvasElement =
					<HtmlElement<html::Canvas> as Deref>::deref(el).clone();
				ri.create_surface(el).await.ok()
			} else {
				None
			}
		}
	});
	let surface = surface.cache_with().map_get(|oo| oo.and_then(|o| o));

	let try_render = {
		let surface = surface.clone();
		let render_once = move |renderable: Renderable| {
			let surface = surface.clone();
			if let Some(surface) = surface.get() {
				match surface.render(renderable) {
					Err(wgpu::SurfaceError::Lost) => {
						if let Some(canvas) = canvas_element.get() {
							surface
								.configure(canvas.width(), canvas.height())
								.ok_or_log();
						}
					}
					other_result => {
						other_result.ok_or_log();
					}
				}
			}
		};
		let render_once = use_throttle_fn_with_arg(render_once, 30.0);
		let renderable = renderable.clone();
		move || render_once(renderable.clone().get())
	};

	// Render as an effect. Note that `try_render` calls `renderable.get()`, so this will be re-run whenever the `renderable` changes.
	{
		let try_render = try_render.clone();
		// `create_effect` would also work here, but this may allow us to better levarage parallelism.
		leptos::create_render_effect(move |_| try_render());
	}

	// On resize, try to render. Note that this will additionally reconfigure if the surface is lost.
	{
		let try_render = try_render.clone();
		leptos_use::use_resize_observer(canvas_ref, move |entries, _| {
			if let Some(surface) = surface.get_untracked() {
				if let Some(entry) = entries.last() {
					let size = entry.device_pixel_content_box_size().get(0);
					if let Ok(size) = size.dyn_into::<web_sys::ResizeObserverSize>() {
						surface.resized(size.inline_size() as u32, size.block_size() as u32);
						try_render();
					}
				}
			}
		});
	}

	view! { <canvas id="render_canvas" node_ref=canvas_ref></canvas> }
}
