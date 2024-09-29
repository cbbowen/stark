use std::sync::Mutex;

use leptos::prelude::*;

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

mod leptos_try;
pub use leptos_try::*;

mod oklab;
pub use oklab::*;

mod piecewise_linear;
pub use piecewise_linear::*;

mod promise;
pub use promise::*;

mod image;
pub use image::ImageExt;

pub mod clothoid;
pub mod input_interpolate;

use leptos::wasm_bindgen;
use leptos::web_sys;
use wasm_bindgen::JsCast;
use wgpu::Extent3d;

use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone, Copy)]
pub struct Unequal<T>(T);

impl<T: std::fmt::Debug> std::fmt::Debug for Unequal<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

impl<T> PartialEq for Unequal<T> {
	fn eq(&self, _other: &Self) -> bool {
		false
	}
}

impl<T> std::ops::Deref for Unequal<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

/// It is useful to think of signals as having two channels:
///
/// 1. A value that can be fetched.
/// 2. An "event" that the value may have changed.
///
/// `create_derived` caches only the value, forwarding all notifications from the underlying signal.
/// This differs from `leptos::create_memo` which additionally does not notify if the new value is
/// equal to the previous one. In some cases, that is desirable, but it requires the type to
/// implement `PartialEq` which is not always possible. In others, e.g. `Trigger`, it is actively
/// undesirable. Leptos used to provide a lower-level primitive that made it trivial to separate
/// the two, but 0.7 introduced a (completely unnecessary) `PartialEq` bound on those.
pub fn create_derived<T: Clone + Send + Sync + 'static>(
	f: impl Fn() -> T + Send + Sync + 'static,
) -> Signal<T> {
	let memo = Memo::new_owning(move |_| (Unequal(f()), true));
	Signal::derive(move || memo.with(|m| m.0.clone()))
}

pub fn create_local_derived<T: Clone + 'static>(
	f: impl Fn() -> T + 'static,
) -> Signal<T, LocalStorage> {
	use send_wrapper::SendWrapper;
	// Ideally, we would just use a `Memo` with `LocalStorage` here, but that isn't implemented yet.
	let f = SendWrapper::new(f);
	let f = move || f();
	let memo = Memo::new_owning(move |_| (Unequal(SendWrapper::new(f())), true));
	Signal::derive_local(move || memo.with(|m| (*m.0).clone()))
}

pub struct LocalCallback<In: 'static, Out: 'static = ()>(
	StoredValue<Box<dyn Fn(In) -> Out>, LocalStorage>,
);

impl<In, Out> Copy for LocalCallback<In, Out> {}
impl<In, Out> Clone for LocalCallback<In, Out> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<In, Out> LocalCallback<In, Out> {
	pub fn new(value: impl Fn(In) -> Out + 'static) -> Self {
		Self(StoredValue::new_local(Box::new(value)))
	}
}

impl<In, Out, F: Fn(In) -> Out + 'static> From<F> for LocalCallback<In, Out> {
	fn from(value: F) -> Self {
		Self::new(value)
	}
}

impl<In, Out> leptos::prelude::Callable<In, Out> for LocalCallback<In, Out> {
	fn run(&self, input: In) -> Out {
		self.0.with_value(|f| f(input))
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
	on_cleanup(move || handle.clear());
	Ok(())
}

pub fn set_interval_and_clean_up(
	cb: impl Fn() + 'static,
	duration: std::time::Duration,
) -> Result<(), JsError> {
	let handle = set_interval_with_handle(cb, duration)?;
	on_cleanup(move || handle.clear());
	Ok(())
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

impl CoordinateSource for leptos::ev::WheelEvent {
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

pub trait DeviceExt {
	fn get_buffer_data(
		self: Arc<Self>,
		buffer: std::sync::Arc<wgpu::Buffer>,
	) -> impl std::future::Future<Output = anyhow::Result<Vec<u8>>>;
}

impl DeviceExt for wgpu::Device {
	fn get_buffer_data(
		self: Arc<Self>,
		buffer: std::sync::Arc<wgpu::Buffer>,
	) -> impl std::future::Future<Output = anyhow::Result<Vec<u8>>> {
		async move {
			let slice = buffer.slice(..);
			let (map_async_future, fulfill) = Promise::new();
			slice.map_async(wgpu::MapMode::Read, fulfill);
			self.poll(wgpu::Maintain::wait());
			map_async_future.await?;
			Ok(slice.get_mapped_range().to_vec())
		}
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
) -> impl Fn(Arc<dyn Fn() -> R>) -> Arc<Mutex<Option<R>>> + Clone {
	let is_available = Rc::new(std::cell::Cell::new(true));
	let last_return_value: Arc<Mutex<Option<R>>> = Default::default();

	move |invoke: Arc<dyn Fn() -> R>| {
		let last_return_value = last_return_value.clone();
		let is_available = is_available.clone();
		if is_available.take() {
			use leptos::reactive_graph::diagnostics::SpecialNonReactiveZone;

			let return_value = {
				#[cfg(debug_assertions)]
				let _guard = SpecialNonReactiveZone::enter();
				invoke()
			};

			*last_return_value.lock().unwrap() = Some(return_value);

			request_animation_frame(move || is_available.set(true));
		}
		last_return_value
	}
}

pub fn use_animation_frame_throttle<F, R>(func: F) -> impl Fn() -> Arc<Mutex<Option<R>>> + Clone
where
	F: Fn() -> R + Clone + 'static,
	R: 'static,
{
	leptos_use::utils::create_filter_wrapper(Arc::new(animation_frame_throttle_filter()), func)
}

pub fn use_animation_frame_throttle_with_arg<F, Arg, R>(
	func: F,
) -> impl Fn(Arg) -> Arc<Mutex<Option<R>>> + Clone
where
	F: Fn(Arg) -> R + Clone + 'static,
	Arg: Clone + 'static,
	R: 'static,
{
	leptos_use::utils::create_filter_wrapper_with_arg(
		Arc::new(animation_frame_throttle_filter()),
		func,
	)
}

pub fn try_color_from_css_string(name: &str) -> Option<glam::Vec4> {
	let color = csscolorparser::parse(name).ok_or_log()?;
	Some(glam::vec4(color.r, color.g, color.b, color.a))
}

pub fn color_from_css_string(name: &str) -> glam::Vec4 {
	try_color_from_css_string(name).unwrap_or(glam::Vec4::ZERO)
}
