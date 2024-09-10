use crate::util::*;
use crate::*;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys;
use leptos_use::{use_debounce_fn, use_element_size};
use std::borrow::Borrow;
use std::rc::Rc;
use std::sync::Arc;
use std::{fmt::Debug, ops::Deref};
use tracing::{error, trace, warn};

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

pub type WgpuSurface = Rc<wgpu::Surface<'static>>;

#[tracing::instrument(err)]
fn create_surface(
	context: Arc<WgpuContext>,
	element: web_sys::HtmlCanvasElement,
) -> Result<WgpuSurface, RenderSurfaceError> {
	use RenderSurfaceError::*;

	#[allow(unused_assignments, unused_mut)]
	let mut surface = Err(UnsupportedPlatform);

	#[cfg(all(target_arch = "wasm32", not(target_os = "emscripten")))]
	{
		surface = Ok(context
			.instance()
			.create_surface(wgpu::SurfaceTarget::Canvas(element))?);
	}

	#[allow(unreachable_code)]
	let surface = surface?;
	if !context.adapter().is_surface_supported(&surface) {
		return Err(UnsupportedSurface.into());
	}
	let surface = Rc::new(surface);

	Ok(surface)
}

/// Argument tuple to `ConfigureCallback`.
pub type ConfigureArgs = (WgpuSurface, u32, u32);

/// Callback type which determines the surface configuration.
pub type ConfigureCallback = LocalCallback<ConfigureArgs, Option<wgpu::SurfaceConfiguration>>;

pub type ConfiguredCallback = LocalCallback<wgpu::SurfaceConfiguration>;

pub type RenderCallback = LocalCallback<wgpu::TextureView>;

#[component]
pub fn RenderSurface(
	#[prop(optional, into)] node_ref: Option<NodeRef<leptos::html::Canvas>>,
	#[prop(into)] render: MaybeSignal<RenderCallback, LocalStorage>,
	#[prop(optional, into)] configure: Option<ConfigureCallback>,
	#[prop(optional, into)] configured: Option<ConfiguredCallback>,
	#[prop(default = 250.0, into)] min_configure_interval: f64,
) -> impl IntoView {
	let context: Arc<WgpuContext> = use_yolo_context();

	let node_ref = node_ref.unwrap_or_else(NodeRef::new);

	let surface = {
		let context = context.clone();
		create_local_derived(move || {
			let context = context.clone();
			node_ref
				.get()
				.and_then(move |element| create_surface(context, element).ok_or_log())
		})
	};

	// Default to the default surface configuration.
	let configure = configure.map(move |configure| {
		let configure = move |args| configure.run(args);
		let b: Rc<dyn Fn(ConfigureArgs) -> Option<wgpu::SurfaceConfiguration>> = Rc::new(configure);
		b
	});
	let configure = {
		let context = context.clone();
		let default_configure = move |args: ConfigureArgs| {
			let (surface, width, height) = args;
			surface.get_default_config(context.adapter(), width, height)
		};
		configure.unwrap_or_else(|| Rc::new(default_configure))
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
		let (get, write) = signal_local(true);
		let set = use_debounce_fn(move || write.try_set_or_log(true), min_configure_interval);
		let clear = move || *write.write_untracked() = false;
		(get, set, clear)
	};

	let size = RwSignal::new_local(None);
	let write_resize = move |width, height| {
		// TODO: Figure out why we're getting bogus size updates here.
		if width == 0 || height == 0 {
			warn!(width, height, "RenderSurface::write_resize failed");
			return;
		};
		size.set(Some((width, height)));
		// size.set(Some((256, 256)));
	};
	StoredValue::new_local(RenderEffect::new(move |_| {
		if size.get().is_some() {
			set_needs_reconfigure();
		}
	}));
	StoredValue::new_local(RenderEffect::new(move |_| {
		let element = node_ref.get();
		if let Some(element) = &element {
			write_resize(
				element.client_width() as u32,
				element.client_height() as u32,
			);
		}
	}));

	let try_reconfigure = {
		trace!("RenderSurface::try_reconfigure");
		let context = context.clone();
		move |args: ConfigureArgs| -> bool {
			let surface = args.0.clone();
			let Some(configuration) = configure(args.clone()) else {
				warn!(?args, "Failed to configure surface");
				return false;
			};
			surface.configure(context.device(), &configuration);
			clear_needs_reconfigure();
			if let Some(configured) = &configured {
				configured.run(configuration);
			}
			true
		}
	};

	// This must not attempt to track signals because it will only be called conditionally. Anything
	// that should be tracked should instead be an argument.
	let try_render = move |args: (Option<WgpuSurface>, bool)| {
		let span = tracing::trace_span!("RenderSurface::try_render");
		let _enter = span.enter();

		let (surface, needs_reconfigure) = args;
		let Some(surface) = surface else {
			trace!("surface not yet set");
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
					warn!("lost surface");
					None
				}
				Err(err) => {
					error!(?err, "failed to get output texture");
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
						error!(?err, "failed to get output texture");
						return;
					}
				}
			}
		};

		let view = surface_texture
			.texture
			.create_view(&wgpu::TextureViewDescriptor::default());
		render.with_untracked(|f| f.run(view));
		surface_texture.present();
	};
	let try_render = use_animation_frame_throttle_with_arg(try_render);
	let try_render = move || {
		// What is going on here?
		// Intuitively, `render.get()` should be passed to `try_render` below. However, `try_render`
		// may invoke the `configured` callback causing changes the `render` signal. The callback
		// would then be out-of-date which is a problem if it affects pipeline compatibility. This
		// fix is, however, a hack. I need to rethink the API a bit.
		render.with(|_| {});

		try_render((surface.get(), needs_reconfigure.get()))
	};

	// Render as an effect.
	Effect::new(move |_| try_render());

	// On resize, try to render. Note that this will additionally reconfigure if the surface is lost.
	leptos_use::use_resize_observer(node_ref.get(), move |entries, _| {
		let Some(entry) = entries.last() else {
			return;
		};
		let size = entry.device_pixel_content_box_size().get(0);
		if let Ok(size) = size.dyn_into::<web_sys::ResizeObserverSize>() {
			write_resize(size.inline_size() as u32, size.block_size() as u32);
		}
	});

	view! { <canvas class="RenderSurface" node_ref=node_ref></canvas> }
}
