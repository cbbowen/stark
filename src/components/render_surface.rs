use crate::render;
use crate::render::wgpu_wrapper;
use crate::util::*;
use leptos::*;
use leptos_use::{use_debounce_fn, use_throttle_fn_with_arg};
use std::{fmt::Debug, ops::Deref};
use tracing::error;
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, thiserror::Error)]
enum RenderSurfaceError {
	#[error("unsupported platform")]
	UnsupportedPlatform,

	#[error("unsupported surface")]
	UnsupportedSurface,

	#[error("failed to create surface")]
	CreateSurfaceError(#[from] wgpu::CreateSurfaceError),
}

static_assertions::assert_impl_all!(RenderSurfaceError: std::error::Error, Send, Sync);
static_assertions::assert_impl_all!(Result<(), RenderSurfaceError>: leptos::IntoView);

#[tracing::instrument(err)]
fn create_surface(
	context: render::Context,
	element: web_sys::HtmlCanvasElement,
) -> Result<render::WgpuSurface, RenderSurfaceError> {
	use RenderSurfaceError::*;

	#[allow(unused_variables, unused_mut)]
	let mut surface = Err(UnsupportedPlatform);

	#[cfg(all(target_arch = "wasm32", not(target_os = "emscripten")))]
	{
		surface = Ok(context
			.instance
			.create_surface(wgpu::SurfaceTarget::Canvas(element))?);
	}

	#[allow(unreachable_code)]
	let surface = surface?;
	if !context.adapter.is_surface_supported(&surface) {
		return Err(UnsupportedSurface.into());
	}
	let surface = wgpu_wrapper(surface);

	Ok(surface)
}

/// Argument tuple to `ConfigureCallback`.
pub type ConfigureArgs = (render::WgpuSurface, u32, u32);

/// Callback type which determines the surface configuration.
pub type ConfigureCallback = Callback<ConfigureArgs, Option<wgpu::SurfaceConfiguration>>;

#[component]
pub fn RenderSurface(
	#[prop(into)] render: MaybeSignal<render::RenderCallback>,
	#[prop(optional, into)] configure: Option<ConfigureCallback>,
	#[prop(optional, into)] configured: Option<Callback<wgpu::SurfaceConfiguration>>,
	#[prop(default = 250.0, into)] min_configure_interval: f64,
	#[prop(default = 30.0, into)] min_render_interval: f64,
) -> impl IntoView {
	let context: render::Context = expect_context();

	let element_node_ref = leptos::create_node_ref();
	let element = element_node_ref.mounted_element();
	let element = element.map(move |e| <HtmlElement<html::Canvas> as Deref>::deref(e).clone());
	let surface = {
		let context = context.clone();
		element.map(move |e| create_surface(context, e.clone()))
	};

	// Default to the default surface configuration.
	let configure = {
		let adapter = context.adapter.clone();
		configure.unwrap_or(
			(move |args: ConfigureArgs| {
				let (surface, width, height) = args;
				surface.get_default_config(&*adapter, width, height)
			})
			.into(),
		)
	};

	// Requirements for when to configure and render:
	// 1. After configuring, another configure must not occur for `configure_interval`.
	// 2. After rendering, another render must not occur for `render_interval`.
	// 3. Rendering must occur _immediately_ after every configure to avoid flashing.
	// 4. Configuring must occur at some point after resizing.
	// 5. Rendering must occur at some point after the renderable changes.
	//
	// Requirements (2) and (3) are what make the implementation tricky. Together, they essentially
	// imply that we must do the reconfigure inside the render effect. That, in turn, means we can't
	// simply put the configure function behind a throttle.

	let (needs_reconfigure, set_needs_reconfigure, clear_needs_reconfigure) = {
		let (get, write) = create_signal(true);
		let set = use_debounce_fn(move || write.set(true), min_configure_interval);
		let clear = move || write.set_untracked(false);
		(get, set, clear)
	};

	let (size, write_resize) = create_signal::<Option<(u32, u32)>>(None);
	create_render_effect(move |_| {
		if size.get().is_some() {
			set_needs_reconfigure();
		}
	});
	create_render_effect(move |_| {
		if let Some(element) = element.get() {
			write_resize.set(Some((
				element.client_width() as u32,
				element.client_height() as u32,
			)));
		}
	});

	let try_reconfigure = {
		let device = context.device.clone();
		move |args: ConfigureArgs| -> bool {
			let surface = args.0.clone();
			let Some(configuration) = configure.call(args.clone()) else {
				tracing::warn!(?args, "Failed to configure surface");
				return false;
			};
			surface.configure(&*device, &configuration);
			clear_needs_reconfigure();
			if let Some(configured) = &configured {
				configured.call(configuration);
			}
			true
		}
	};

	// This must not attempt to track signals because it will only be called conditionally. Anything
	// that should be tracked should instead be an argument.
	let try_render = move |args: (
		Option<Result<render::WgpuSurface, RenderSurfaceError>>,
		render::RenderCallback,
		bool,
	)| {
		let (surface, render, needs_reconfigure) = args;
		let Some(Ok(surface)) = surface else {
			return;
		};

		let Some((width, height)) = size.get_untracked() else {
			error!("no size");
			return;
		};

		let surface_texture = if needs_reconfigure {
			None
		} else {
			match surface.get_current_texture() {
				Ok(surface_texture) => Some(surface_texture),
				Err(wgpu::SurfaceError::Lost) => {
					tracing::warn!("lost surface");
					None
				}
				Err(err) => {
					tracing::error!(?err, "failed to get output texture");
					return;
				}
			}
		};

		let surface_texture = match surface_texture {
			Some(surface_texture) => surface_texture,
			None => {
				if !try_reconfigure((surface.clone(), width, height)) {
					error!("failed to reconfigure");
					return;
				}
				match surface.get_current_texture() {
					Ok(surface_texture) => surface_texture,
					Err(err) => {
						tracing::error!(?err, "failed to get output texture");
						return;
					}
				}
			}
		};

		let view = surface_texture
			.texture
			.create_view(&wgpu::TextureViewDescriptor::default());
		render.call(view);
		surface_texture.present();
	};
	let try_render = use_throttle_fn_with_arg(try_render, min_render_interval);
	let try_render = move || try_render((surface.get(), render.get(), needs_reconfigure.get()));

	// Render as an effect.
	create_effect(move |_| try_render());

	// On resize, try to render. Note that this will additionally reconfigure if the surface is lost.
	leptos_use::use_resize_observer(element_node_ref, move |entries, _| {
		let Some(entry) = entries.last() else {
			return;
		};
		let size = entry.device_pixel_content_box_size().get(0);
		if let Ok(size) = size.dyn_into::<web_sys::ResizeObserverSize>() {
			write_resize.set(Some((size.inline_size() as u32, size.block_size() as u32)));
		}
	});

	view! { <canvas class="RenderSurface" node_ref=element_node_ref></canvas> }
}
