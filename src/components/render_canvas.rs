use crate::components::{ErrorList, Waiting};
use crate::render;
use crate::render::wgpu_wrapper;
use crate::util::*;
use leptos::*;
use leptos_use::{use_throttle_fn, use_throttle_fn_with_arg};
use std::{fmt::Debug, ops::Deref, rc::Rc};
use tracing::{error, info, trace};
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, thiserror::Error)]
pub enum RenderSurfaceError {
	#[error("unsupported platform")]
	UnsupportedPlatform,

	#[error("unsupported surface")]
	UnsupportedSurface,

	#[error("failed to create surface")]
	CreateSurfaceError(#[from] wgpu::CreateSurfaceError),
}

static_assertions::assert_impl_all!(RenderSurfaceError: std::error::Error, Send, Sync);
static_assertions::assert_impl_all!(Result<(), RenderSurfaceError>: leptos::IntoView);

fn create_surface_configuration(
	context: &render::Context,
	surface: &render::WgpuSurface,
	width: u32,
	height: u32,
) -> Result<wgpu::SurfaceConfiguration, RenderSurfaceError> {
	use RenderSurfaceError::*;
	let config = surface
		.get_default_config(&*context.adapter, width, height)
		.ok_or(UnsupportedSurface)?;
	Ok(wgpu::SurfaceConfiguration {
		format: wgpu::TextureFormat::Rgba16Float,
		..config
	})
}

#[derive(Debug, Clone)]
struct RenderSurface {
	context: render::Context,
	surface: render::WgpuSurface,
	surface_configuration: leptos::RwSignal<wgpu::SurfaceConfiguration>,
	// TODO: There's no reason for this to be a separate `Rc` from `surface`.
	resize: Rc<std::cell::Cell<Option<(u32, u32)>>>,
}

impl RenderSurface {
	#[tracing::instrument(err)]
	fn new(
		context: render::Context,
		canvas: web_sys::HtmlCanvasElement,
	) -> Result<Self, RenderSurfaceError> {
		use RenderSurfaceError::*;

		#[allow(unused_variables, unused_mut)]
		let mut surface = Err(UnsupportedPlatform);

		let width = canvas.client_width() as u32;
		let height = canvas.client_height() as u32;

		#[cfg(all(target_arch = "wasm32", not(target_os = "emscripten")))]
		{
			surface = Ok(context
				.instance
				.create_surface(wgpu::SurfaceTarget::Canvas(canvas))?);
		}

		#[allow(unreachable_code)]
		let surface = surface?;
		if !context.adapter.is_surface_supported(&surface) {
			return Err(UnsupportedSurface.into());
		}
		let surface = wgpu_wrapper(surface);

		let surface_configuration = create_surface_configuration(&context, &surface, width, height)?;
		surface.configure(&*context.device, &surface_configuration);

		Ok(RenderSurface {
			context,
			surface,
			surface_configuration: leptos::create_rw_signal(surface_configuration),
			resize: Rc::new(std::cell::Cell::new(None)),
		})
	}

	pub fn resized(&self, width: u32, height: u32) {
		self.resize.set(Some((width, height)));
	}

	#[tracing::instrument(err)]
	pub fn configure(&self, width: u32, height: u32) -> Result<(), RenderSurfaceError> {
		let surface_configuration =
			create_surface_configuration(&self.context, &self.surface, width, height)?;
		self
			.surface
			.configure(&*self.context.device, &surface_configuration);
		self.surface_configuration.set(surface_configuration);
		Ok(())
	}

	#[tracing::instrument(err, skip(source))]
	pub fn render(&self, source: impl FnOnce(wgpu::TextureView)) -> Result<(), wgpu::SurfaceError> {
		// tracing::trace!("RenderSurface::render");

		if let Some((width, height)) = self.resize.take() {
			trace!(width, height, "reconfiguring");
			self.configure(width, height).ok_or_log();
		}

		let output = self.surface.get_current_texture()?;

		let view = output
			.texture
			.create_view(&wgpu::TextureViewDescriptor::default());

		source(view);
		output.present();

		Ok(())
	}
}

impl PartialEq for RenderSurface {
	fn eq(&self, other: &Self) -> bool {
		self.surface == other.surface
	}
}

// fn render_canvas_with_context(context: render::Context, source: render::Source, surface_configuration_changed: impl Fn(wgpu::SurfaceConfiguration)) -> impl IntoView {
// 	tracing::warn!("RenderCanvas::render_canvas");
// 	let canvas_ref = leptos::create_node_ref();
// 	let canvas_element = canvas_ref.mounted_element();
// 	let surface = {
// 		tracing::warn!("surface = canvas_element.map(...)");
// 		let context = context.clone();
// 		canvas_element.map(move |canvas| {
// 			let canvas: web_sys::HtmlCanvasElement =
// 				<HtmlElement<html::Canvas> as Deref>::deref(canvas).clone();
// 			RenderSurface::new(context, canvas)
// 		})
// 	};
// 	surface.get().map(|r| r.ok_or_log());

// 	let try_render = {
// 		tracing::warn!("building try_render");
// 		// This function is not reactive.
// 		let surface = surface.clone();
// 		let render_once = move || {
// 			tracing::warn!("try_render");
// 			let Some(Ok(surface)) = surface.get_untracked() else {
// 				return;
// 			};
// 			match surface.render(|view| source.call(view)) {
// 				Err(wgpu::SurfaceError::Lost) => {
// 					if let Some(canvas) = canvas_element.get_untracked() {
// 						surface
// 							.configure(canvas.width(), canvas.height())
// 							.ok_or_log();
// 					}
// 				}
// 				other_result => {
// 					other_result.ok_or_log();
// 				}
// 			}
// 		};
// 		use_throttle_fn(render_once, 1000.0)
// 	};

// 	// Render as an effect. Note that `try_render` calls `renderable.get()`, so this will be
// 	// re-run whenever the `renderable` changes.
// 	{
// 		let try_render = try_render.clone();
// 		// `create_render_effect` would also work here.
// 		leptos::create_effect(move |_| try_render());
// 	}

// 	// On resize, try to render. Note that this will additionally reconfigure if the surface is lost.
// 	{
// 		let try_render = try_render.clone();
// 		leptos_use::use_resize_observer(canvas_ref, move |entries, _| {
// 			let Some(Ok(surface)) = surface.get_untracked() else {
// 				return;
// 			};
// 			let Some(entry) = entries.last() else {
// 				return;
// 			};
// 			let size = entry.device_pixel_content_box_size().get(0);
// 			if let Ok(size) = size.dyn_into::<web_sys::ResizeObserverSize>() {
// 				surface.resized(size.inline_size() as u32, size.block_size() as u32);
// 				// try_render();
// 			}
// 		});
// 	}

// 	view! { <canvas id="render_canvas" node_ref=canvas_ref></canvas> }
// }

#[component]
pub fn RenderCanvas(
	#[prop(into)] render: render::Source,
	#[prop(optional)] configured: Option<Callback<wgpu::SurfaceConfiguration>>,
) -> impl IntoView {
	tracing::warn!("RenderCanvas");
	let context: render::Context = expect_context();

	let canvas_ref = leptos::create_node_ref();
	let canvas_element = canvas_ref.mounted_element();
	let surface = {
		tracing::warn!("surface = canvas_element.map(...)");
		let context = context.clone();
		canvas_element.map(move |canvas| {
			let canvas: web_sys::HtmlCanvasElement =
				<HtmlElement<html::Canvas> as Deref>::deref(canvas).clone();
			RenderSurface::new(context, canvas)
		})
	};

	let try_render = {
		tracing::warn!("building try_render");
		let surface = surface.clone();
		let render_once = move || {
			tracing::warn!("try_render");
			let Some(Ok(surface)) = surface.get_untracked() else {
				return;
			};
			match surface.render(|view| render.call(view)) {
				Err(wgpu::SurfaceError::Lost) => {
					if let Some(canvas) = canvas_element.get_untracked() {
						surface
							.configure(canvas.width(), canvas.height())
							.ok_or_log();
					}
				}
				other_result => {
					other_result.ok_or_log();
				}
			}
		};
		use_throttle_fn(render_once, 1000.0)
	};

	create_effect(move |_| try_render());

	// TODO: Make `render` reactive?
	// TODO: Call `configured`.

	view! { <canvas id="render_canvas" node_ref=canvas_ref></canvas> }
}
