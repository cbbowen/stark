use leptos::prelude::*;
use std::rc::Rc;
use std::sync::Arc;
use send_wrapper::SendWrapper;
use std::ops::{Deref, DerefMut};

// Panics if the result is actually called multiple times.
pub fn yolo_fn_once_to_fn<Out>(
	f: impl FnOnce() -> Out + 'static,
) -> impl Clone + Send + Fn() -> Out + 'static {
	let f = Arc::new(SendWrapper::new(std::cell::Cell::new(Some(f))));
	move || {
		let f = f.deref();
		let f = f.deref();
		let f = f.take().unwrap();
		f()
	}
}

pub struct YoloValue<T>(Arc<T>);

impl<T> Clone for YoloValue<T> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

impl<T: Send + 'static> YoloValue<T> {
	pub fn new(value: T) -> Self {
		Self(Arc::new(value))
	}

	pub fn get(&self) -> Arc<T> {
		self.0.clone()
	}
}

pub fn use_yolo_context<T: Send + 'static>() -> Arc<T> {
	let context: YoloValue<T> = use_context().unwrap();
	context.get()
}