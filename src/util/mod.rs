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

mod once;
pub use once::*;

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

pub fn nonzero_size_of<T>() -> Option<std::num::NonZero<u64>> {
	std::num::NonZero::new(std::mem::size_of::<T>() as u64)
}