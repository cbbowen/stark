use leptos::{store_value, Callable, StoredValue};
use std::fmt;

use super::OptionExt;

pub struct TryCallback<In: 'static, Out: 'static = ()>(StoredValue<Box<dyn Fn(In) -> Out>>);

impl<In, Out> TryCallback<In, Out> {
	fn try_call(&self, input: In) -> Option<Out> {
		self.0.try_with_value(|f| f(input))
	}
}

impl<In> fmt::Debug for TryCallback<In> {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
		fmt.write_str("TryCallback")
	}
}

impl<In, Out> Clone for TryCallback<In, Out> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<In, Out> Copy for TryCallback<In, Out> {}

impl<In, Out> TryCallback<In, Out> {
	pub fn new<F>(f: F) -> TryCallback<In, Out>
	where
		F: Fn(In) -> Out + 'static,
	{
		Self(store_value(Box::new(f)))
	}
}

impl<In: 'static, Out: Default + 'static> Callable<In, Out> for TryCallback<In, Out> {
	fn call(&self, input: In) -> Out {
		self
			.try_call(input)
			.unwrap_or_default_and_log("callback already disposed")
	}
}

impl<In, Out, F: Fn(In) -> Out + 'static> From<F> for TryCallback<In, Out> {
	fn from(value: F) -> Self {
		Self::new(value)
	}
}
