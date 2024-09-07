use leptos::{
	store_value, Callable, Memo, SignalDispose, SignalGet, SignalGetUntracked, SignalWith,
	SignalWithUntracked, StoredValue,
};
use std::fmt;

use super::{create_derived, OptionExt};

/// This is essentially identical to `leptos::Callback` but additionally supports `try_call`.
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

pub struct CallbackSignal<In: 'static, Out: 'static = ()>(Memo<TryCallback<In, Out>>);

impl<In, Out> fmt::Debug for CallbackSignal<In, Out> {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
		fmt.write_str("CallbackSignal")
	}
}

impl<In, Out> Clone for CallbackSignal<In, Out> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

impl<In, Out> Copy for CallbackSignal<In, Out> {}

impl<In: 'static, Out: 'static> CallbackSignal<In, Out> {
	pub fn new<G, F>(g: G) -> Self
	where
		G: Fn() -> F + 'static,
		F: Fn(In) -> Out + 'static,
	{
		Self(create_derived(move || TryCallback::new(g())))
	}

	pub fn try_call(&self, input: In) -> Option<Out> {
		self.try_with(move |f| f.try_call(input)).and_then(|o| o)
	}

	pub fn try_call_untracked(&self, input: In) -> Option<Out> {
		self
			.try_with_untracked(move |f| f.try_call(input))
			.and_then(|o| o)
	}
}

impl<In, Out, F: Fn(In) -> Out + 'static> From<F> for CallbackSignal<In, Out> {
	fn from(value: F) -> Self {
		let value = std::cell::Cell::new(Some(value));
		Self::new(move || value.take().unwrap())
	}
}

impl<In, Out> SignalGet for CallbackSignal<In, Out> {
	type Value = TryCallback<In, Out>;

	fn get(&self) -> Self::Value {
		self.0.get()
	}

	fn try_get(&self) -> Option<Self::Value> {
		self.0.try_get()
	}
}

impl<In, Out> SignalGetUntracked for CallbackSignal<In, Out> {
	type Value = TryCallback<In, Out>;

	fn get_untracked(&self) -> Self::Value {
		self.0.get_untracked()
	}

	fn try_get_untracked(&self) -> Option<Self::Value> {
		self.0.try_get_untracked()
	}
}

impl<In, Out> leptos::SignalWith for CallbackSignal<In, Out> {
	type Value = TryCallback<In, Out>;

	fn with<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> O {
		self.0.with(f)
	}

	fn try_with<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> Option<O> {
		self.0.try_with(f)
	}
}

impl<In, Out> leptos::SignalWithUntracked for CallbackSignal<In, Out> {
	type Value = TryCallback<In, Out>;

	fn with_untracked<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> O {
		self.0.with_untracked(f)
	}

	fn try_with_untracked<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> Option<O> {
		self.0.try_with_untracked(f)
	}
}

impl<In, Out: Default> Callable<In, Out> for CallbackSignal<In, Out> {
	fn call(&self, input: In) -> Out {
		self
			.try_call(input)
			.unwrap_or_default_and_log("callback already disposed")
	}
}

impl<In, Out> SignalDispose for CallbackSignal<In, Out> {
	fn dispose(self) {
		self.0.dispose()
	}
}
