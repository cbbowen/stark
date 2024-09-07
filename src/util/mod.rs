use std::borrow::Borrow;

use leptos::*;

// mod distinct;
// pub use distinct::Distinct;

// mod distinct_signal;
// pub use distinct_signal::*;

// mod distinct_trigger;
// pub use distinct_trigger::*;

// mod map_signal;
// pub use map_signal::*;

mod result_ext;
pub use result_ext::*;

mod try_callback;
pub use try_callback::*;

mod once;
pub use once::*;
use wasm_bindgen::JsCast;
use wgpu::Extent3d;

/// It is useful to think of signals as having two channels:
///
/// 1. A value that can be fetched.
/// 2. An "event" that the value may have changed.
///
/// `create_derived` caches only the value, forwarding all notifications from the underlying signal.
/// This differs from `leptos::create_memo` which additionally does not notify if the new value is
/// equal to the previous one. In some cases, that is desirable, but it requires the type to
/// implement `PartialEq` which is not always possible. In others, e.g. `Trigger`, it is actively
/// undesirable. Fortunately, Leptos provides a lower-level primitive that makes it trivial to
/// separate the two.
pub fn create_derived<T>(f: impl Fn() -> T + 'static) -> Memo<T> {
	create_owning_memo(move |_| (f(), true))
}

pub trait ElementExt<T: html::ElementDescriptor + 'static> {
	fn mount_trigger(self) -> Trigger;
}

impl<T: html::ElementDescriptor + 'static> ElementExt<T> for HtmlElement<T> {
	fn mount_trigger(self) -> Trigger {
		let trigger = create_trigger();
		let _ = self.on_mount(move |_| {
			trigger.try_notify();
		});
		trigger
	}
}

pub trait NodeRefExt<T: html::ElementDescriptor> {
	/// Creates a signal that provides the `HtmlElement` once it is mounted to the DOM.
	fn mounted_element(self) -> OnceMemo<HtmlElement<T>>;
}

impl<T: html::ElementDescriptor + Clone + 'static> NodeRefExt<T> for NodeRef<T> {
	fn mounted_element(self) -> OnceMemo<HtmlElement<T>> {
		let element = OnceMemo::new(move || self.get());
		OnceMemo::new(move || {
			element.get().and_then(|e| {
				if e.is_mounted() {
					Some(e)
				} else {
					e.mount_trigger().try_track();
					None
				}
			})
		})
	}
}

#[derive(thiserror::Error, Debug)]
#[error("javascript error")]
pub struct JsError(String);

impl From<wasm_bindgen::JsValue> for JsError {
	fn from(value: wasm_bindgen::JsValue) -> Self {
		JsError(format!("{:?}", value))
	}
}

pub fn set_timeout_and_clean_up(
	cb: impl FnOnce() + 'static,
	duration: std::time::Duration,
) -> Result<(), JsError> {
	let handle = set_timeout_with_handle(cb, duration)?;
	Ok(on_cleanup(move || handle.clear()))
}

pub fn set_interval_and_clean_up(
	cb: impl Fn() + 'static,
	duration: std::time::Duration,
) -> Result<(), JsError> {
	let handle = set_interval_with_handle(cb, duration)?;
	Ok(on_cleanup(move || handle.clear()))
}

pub trait PointerCapture {
	fn set_pointer_capture(&self) -> bool;
	fn release_pointer_capture(&self) -> bool;
}

impl PointerCapture for leptos::ev::PointerEvent {
	fn set_pointer_capture(&self) -> bool {
		self
			.current_target()
			.and_then(|target| target.dyn_into::<web_sys::Element>().ok_or_log())
			.and_then(|target| target.set_pointer_capture(self.pointer_id()).ok_or_log())
			.is_some()
	}

	fn release_pointer_capture(&self) -> bool {
		self
			.current_target()
			.and_then(|target| target.dyn_into::<web_sys::Element>().ok_or_log())
			.and_then(|target| {
				target
					.release_pointer_capture(self.pointer_id())
					.ok_or_log()
			})
			.is_some()
	}
}

pub trait CoordinateSource {
	fn size(&self) -> Option<glam::Vec2>;

	fn pixel_position(&self) -> glam::Vec2;

	fn pixel_movement(&self) -> glam::Vec2;

	fn position(&self) -> Option<glam::Vec2> {
		self.size().map(|size| self.pixel_position() / size)
	}

	fn target_position(&self) -> Option<glam::Vec2> {
		self.position().map(|c| glam::vec2(2.0, -2.0) * (c - 0.5))
	}

	fn movement(&self) -> Option<glam::Vec2> {
		self.size().map(|size| self.pixel_movement() / size)
	}

	fn target_movement(&self) -> Option<glam::Vec2> {
		self.movement().map(|c| glam::vec2(2.0, -2.0) * c)
	}
}

impl CoordinateSource for leptos::ev::PointerEvent {
	fn size(&self) -> Option<glam::Vec2> {
		let element = self
			.current_target()
			.and_then(|target| target.dyn_into::<web_sys::Element>().ok_or_log())?;
		Some(glam::vec2(
			element.client_width() as f32,
			element.client_height() as f32,
		))
	}

	fn pixel_position(&self) -> glam::Vec2 {
		glam::vec2(self.offset_x() as f32, self.offset_y() as f32)
	}

	fn pixel_movement(&self) -> glam::Vec2 {
		glam::vec2(self.movement_x() as f32, self.movement_y() as f32)
	}
}

pub trait QueueExt {
	fn fill_texture_layer(&self, texture: &wgpu::Texture, pixel_data: &[u8], layer_index: u32);
	fn fill_texture(&self, texture: &wgpu::Texture, pixel_data: &[u8]) {
		assert_eq!(texture.depth_or_array_layers(), 1);
		self.fill_texture_layer(texture, pixel_data, 0)
	}
}

impl QueueExt for wgpu::Queue {
	fn fill_texture_layer(&self, texture: &wgpu::Texture, pixel_data: &[u8], layer_index: u32) {
		let size = texture.size();
		let texture_data = pixel_data.repeat((size.width * size.height) as usize);
		self.write_texture(
			wgpu::ImageCopyTexture {
				mip_level: 0,
				origin: wgpu::Origin3d {
					z: layer_index,
					..Default::default()
				},
				texture,
				aspect: wgpu::TextureAspect::All,
			},
			&texture_data,
			wgpu::ImageDataLayout {
				offset: 0,
				bytes_per_row: Some(pixel_data.len() as u32 * size.width),
				rows_per_image: None,
			},
			Extent3d {
				depth_or_array_layers: 1,
				..size
			},
		)
	}
}

fn animation_frame_throttle_filter<R>(
) -> impl Fn(std::rc::Rc<dyn Fn() -> R>) -> std::rc::Rc<std::cell::RefCell<Option<R>>> + Clone {
	let is_available = std::rc::Rc::new(std::cell::Cell::new(true));
	let last_return_value: std::rc::Rc<std::cell::RefCell<Option<R>>> = Default::default();

	move |invoke: std::rc::Rc<dyn Fn() -> R>| {
		let last_return_value = last_return_value.clone();
		let is_available = is_available.clone();
		if is_available.take() {
			#[cfg(debug_assertions)]
			let prev = SpecialNonReactiveZone::enter();

			let return_value = invoke();

			#[cfg(debug_assertions)]
			SpecialNonReactiveZone::exit(prev);

			last_return_value.replace(Some(return_value));

			leptos::request_animation_frame(move || is_available.set(true));
		}

		return last_return_value;
	}
}

pub fn use_animation_frame_throttle<F, R>(
	func: F,
) -> impl Fn() -> std::rc::Rc<std::cell::RefCell<Option<R>>> + Clone
where
	F: Fn() -> R + Clone + 'static,
	R: 'static,
{
	leptos_use::utils::create_filter_wrapper(
		std::rc::Rc::new(animation_frame_throttle_filter()),
		func,
	)
}

pub fn use_animation_frame_throttle_with_arg<F, Arg, R>(
	func: F,
) -> impl Fn(Arg) -> std::rc::Rc<std::cell::RefCell<Option<R>>> + Clone
where
	F: Fn(Arg) -> R + Clone + 'static,
	Arg: Clone + 'static,
	R: 'static,
{
	leptos_use::utils::create_filter_wrapper_with_arg(
		std::rc::Rc::new(animation_frame_throttle_filter()),
		func,
	)
}
