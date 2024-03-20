use std::ops::Deref;

use futures::stream::FlatMap;
use leptos::*;

mod distinct;
pub use distinct::Distinct;

mod distinct_signal;
pub use distinct_signal::*;

mod distinct_trigger;
pub use distinct_trigger::*;

mod map_signal;
pub use map_signal::*;

mod result_ext;
pub use result_ext::*;

mod once;
pub use once::*;

/// It is useful to think of signals as having two channels:
///
/// 1. A value that can be fetched.
/// 2. An event that the value may have changed.
///
/// This caches only the value, forwarding all notifications from the underlying signal. This differs from `leptos::create_memo` which additionally does not notify if the new value is equal to the previous one. In some cases, that is desirable, but it requires the type to implement `PartialEq` which is not always possible. In others, e.g. `Trigger`, it is actively undesirable. Fortunately, Leptos provides a lower level primitive that makes it trivial to separate the two.
pub fn create_cache<T>(f: impl Fn() -> T + 'static) -> Memo<T> {
	create_owning_memo(move |_| (f(), true))
}

pub trait SignalGetExt: SignalGet {
	/// Caches the value this signal. See `create_cache`.
	fn cache_get(self) -> Memo<Self::Value>
	where
		Self: 'static;

	/// Returns a new signal with value `f(self.value())`.
	fn map_get<T, F: Fn(Self::Value) -> T>(
		self,
		f: F,
	) -> MapSignal<Self, GenericMapValue<Self::Value, F>>
	where
		Self: Sized;

	/// Returns a new signal with value `f(self.value())`.
	fn flat_map_get<T, F: Fn(Self::Value) -> T>(
		self,
		f: F,
	) -> FlatMapSignal<Self, GenericMapValue<Self::Value, F>>
	where
		Self: Sized,
		T: SignalGet;
}

impl<S: SignalGet> SignalGetExt for S {
	fn cache_get(self) -> Memo<Self::Value>
	where
		Self: 'static,
	{
		create_cache(move || self.get())
	}

	fn map_get<T, F: Fn(Self::Value) -> T>(
		self,
		f: F,
	) -> MapSignal<Self, GenericMapValue<S::Value, F>>
	where
		Self: Sized,
	{
		MapSignal(self, GenericMapValue::new(f))
	}

	fn flat_map_get<T, F: Fn(Self::Value) -> T>(
		self,
		f: F,
	) -> FlatMapSignal<Self, GenericMapValue<Self::Value, F>>
	where
		Self: Sized,
		T: SignalGet,
	{
		FlatMapSignal(self, GenericMapValue::new(f))
	}
}

pub trait SignalWithExt: SignalWith {
	/// Caches the value this signal. See `create_cache`.
	///
	/// This is also useful for converting from a `SignalWith` to a `SignalGet`, serving a similar purpose to `Iter::cloned`.
	fn cache_with(self) -> Memo<Self::Value>
	where
		Self: 'static,
		Self::Value: Clone;

	/// Returns a new signal with value `f(self.value())`.
	fn map_with<T>(self, f: impl for<'a> Fn(&'a Self::Value) -> &'a T)
		-> impl SignalWith<Value = T>;
}

impl<S: SignalWith> SignalWithExt for S {
	fn cache_with(self) -> Memo<Self::Value>
	where
		Self: 'static,
		Self::Value: Clone,
	{
		create_cache(move || self.with(|v| v.clone()))
	}

	fn map_with<T>(
		self,
		f: impl for<'a> Fn(&'a Self::Value) -> &'a T,
	) -> impl SignalWith<Value = T> {
		MapSignal(self, GenericMapRef::new(f))
	}
}

pub trait ElementExt<T: html::ElementDescriptor + 'static> {
	fn mount_trigger(self) -> Trigger;
}

impl<T: html::ElementDescriptor + 'static> ElementExt<T> for HtmlElement<T> {
	fn mount_trigger(self) -> Trigger {
		let trigger = create_trigger();
		let trigger_clone = trigger.clone();
		let _ = self.on_mount(move |_| {
			trigger_clone.try_notify();
		});
		trigger
	}
}

/// Like `create_memo` but with two important differences:
/// 1. Only produces the first non-`None` value returned by `f`. After this, `f` is not called again, and it will not notify its dependents.
/// 2. Does not require the signal value to implement `PartialEq`.
///
/// This is useful for things that are only expected to change once, like the element of a `NodeRef` and whether or not it is mounted.
fn create_once_signal<T: Clone + 'static>(f: impl Fn() -> Option<T> + 'static) -> Memo<Option<T>> {
	create_owning_memo(move |p| {
		if let Some(Some(p)) = p {
			(Some(p), false)
		} else {
			let v = f();
			let changed = v.is_some();
			(v, changed)
		}
	})
}

pub trait NodeRefExt<T: html::ElementDescriptor> {
	/// Runs the provided closure when the `HtmlElement` connected o the  `NodeRef` is first mounted to the DOM.
	fn on_mount(self, f: impl FnOnce(HtmlElement<T>) + 'static);

	/// Gets the underlying `RwSignal` for the element.
	fn element(&self) -> &RwSignal<Option<HtmlElement<T>>>;

	/// Creates a signal that provides the `HtmlElement` once it is mounted to the DOM.
	fn mounted_element(self) -> Memo<Option<HtmlElement<T>>>;

	fn is_mounted(self) -> impl SignalGet<Value = bool>;
}

impl<T: html::ElementDescriptor + Clone + 'static> NodeRefExt<T> for NodeRef<T> {
	fn on_mount(self, f: impl FnOnce(HtmlElement<T>) + 'static) {
		self.on_load(move |el| {
			if el.is_mounted() {
				f(el);
			} else {
				let _ = el.on_mount(f);
			}
		})
	}

	fn is_mounted(self) -> impl SignalGet<Value = bool> {
		self.mounted_element().map_get(|o| o.is_some())
	}

	fn mounted_element(self) -> Memo<Option<HtmlElement<T>>> {
		let element = create_once_signal(self.clone());
		create_once_signal(move || {
			element().and_then(|e| {
				if e.is_mounted() {
					Some(e)
				} else {
					e.mount_trigger().try_track();
					None
				}
			})
		})
	}

	fn element(&self) -> &RwSignal<Option<HtmlElement<T>>> {
		// SAFETY: `NodeRef` is `repr(transparent)`.
		unsafe { std::mem::transmute(&self) }
	}
}

struct EquatableNodeRef<T: html::ElementDescriptor + 'static>(NodeRef<T>);

impl<T: html::ElementDescriptor> Clone for EquatableNodeRef<T> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

impl<T: html::ElementDescriptor> Copy for EquatableNodeRef<T> {}

impl<T: html::ElementDescriptor> Deref for EquatableNodeRef<T> {
	type Target = NodeRef<T>;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T: html::ElementDescriptor + Clone + 'static> PartialEq for EquatableNodeRef<T> {
	fn eq(&self, other: &Self) -> bool {
		self.0.element() == other.0.element()
	}
}

impl<T: html::ElementDescriptor + Clone + 'static> Eq for EquatableNodeRef<T> {}

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
