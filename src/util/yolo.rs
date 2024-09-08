use leptos::prelude::*;
use std::rc::Rc;
use std::sync::Arc;
use send_wrapper::SendWrapper;
use std::ops::{Deref, DerefMut};

// #[pin_project]
pub struct SendWrapperFuture<F> {
	// #[pin]
	inner: SendWrapper<F>,
}

impl<F> SendWrapperFuture<F> {
	pub fn new(inner: F) -> Self {
		Self {
			inner: SendWrapper::new(inner),
		}
	}
}

unsafe impl<F> Send for SendWrapperFuture<F> {}
unsafe impl<F> Sync for SendWrapperFuture<F> {}

impl<F: std::future::Future> std::future::Future for SendWrapperFuture<F> {
	type Output = F::Output;
	fn poll(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Self::Output> {
		// let this = self.project();
		// this.inner.as_mut().poll(cx)
		
		// SAFETY: No operations can move out of `inner`.
		let inner: std::pin::Pin<&mut F> = unsafe { self.map_unchecked_mut(|s| s.inner.deref_mut()) };
		inner.poll(cx)
	}
}

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

pub struct YoloValue<T>(Arc<SendWrapper<Rc<T>>>);

impl<T> Clone for YoloValue<T> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

impl<T> YoloValue<T> {
	pub fn new(value: T) -> Self {
		Self(Arc::new(SendWrapper::new(Rc::new(value))))
	}

	pub fn get(&self) -> Rc<T> {
		let value: &Rc<T> = &self.0;
		value.clone()
	}
}

pub fn use_yolo_context<T: 'static>() -> Rc<T> {
	let context: YoloValue<T> = use_context().unwrap();
	context.get()
}