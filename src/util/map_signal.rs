use leptos::*;

pub trait MapValue<T> {
	type Value;
	fn map_value(&self, from: T) -> Self::Value;
}

impl<T, U> MapValue<T> for fn(T) -> U {
	type Value = U;
	fn map_value(&self, from: T) -> Self::Value {
		self(from)
	}
}

pub struct GenericMapValue<T, F> {
	f: F,
	_phantom: std::marker::PhantomData<*const T>,
}

impl<T, F: Clone> Clone for GenericMapValue<T, F> {
	fn clone(&self) -> Self {
		Self::new(self.f.clone())
	}
}

impl<T, F> GenericMapValue<T, F> {
	pub fn new(f: F) -> Self {
		GenericMapValue {
			f,
			_phantom: std::marker::PhantomData,
		}
	}
}

impl<T, F> From<F> for GenericMapValue<T, F> {
	fn from(value: F) -> Self {
		GenericMapValue::new(value)
	}
}

impl<T, U, F: Fn(T) -> U> MapValue<T> for GenericMapValue<T, F> {
	type Value = U;
	fn map_value(&self, from: T) -> Self::Value {
		(self.f)(from)
	}
}

pub trait MapRef<T> {
	type Value;
	fn map_ref<'a>(&self, from: &'a T) -> &'a Self::Value
	where
		T: 'a;
}

impl<T, U> MapRef<T> for for<'a> fn(&'a T) -> &'a U {
	type Value = U;
	fn map_ref<'a>(&self, from: &'a T) -> &'a Self::Value
	where
		T: 'a,
	{
		self(from)
	}
}

pub struct GenericMapRef<T, F> {
	f: F,
	_phantom: std::marker::PhantomData<*const T>,
}

impl<T, F: Clone> Clone for GenericMapRef<T, F> {
	fn clone(&self) -> Self {
		Self::new(self.f.clone())
	}
}

impl<T, F> GenericMapRef<T, F> {
	pub fn new(f: F) -> Self {
		GenericMapRef {
			f,
			_phantom: std::marker::PhantomData,
		}
	}
}

impl<T, F> From<F> for GenericMapRef<T, F> {
	fn from(value: F) -> Self {
		GenericMapRef::new(value)
	}
}

impl<T, U, F> MapRef<T> for GenericMapRef<T, F>
where
	F: for<'a> Fn(&'a T) -> &'a U,
{
	type Value = U;
	fn map_ref<'a>(&self, from: &'a T) -> &'a Self::Value
	where
		T: 'a,
	{
		(self.f)(from)
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapSignal<S, M>(pub S, pub M);

// impl<S, M> std::ops::Deref for MapSignal<S, M> {
// 	type Target = S;

// 	fn deref(&self) -> &S {
// 		&self.0
// 	}
// }

impl<S: SignalGet, M: MapValue<S::Value>> SignalGet for MapSignal<S, M> {
	type Value = M::Value;

	fn get(&self) -> Self::Value {
		self.1.map_value(self.0.get())
	}

	fn try_get(&self) -> Option<Self::Value> {
		self.0.try_get().map(|v| self.1.map_value(v))
	}
}

impl<S: SignalGetUntracked, M: MapValue<S::Value>> SignalGetUntracked for MapSignal<S, M> {
	type Value = M::Value;

	fn get_untracked(&self) -> Self::Value {
		self.1.map_value(self.0.get_untracked())
	}

	fn try_get_untracked(&self) -> Option<Self::Value> {
		self.0.try_get_untracked().map(|v| self.1.map_value(v))
	}
}

impl<S: SignalWith, M: MapRef<S::Value>> SignalWith for MapSignal<S, M> {
	type Value = M::Value;

	fn with<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> O {
		self.0.with(|v| f(self.1.map_ref(v)))
	}

	fn try_with<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> Option<O> {
		self.0.try_with(|v| f(self.1.map_ref(v)))
	}
}

impl<S: SignalWithUntracked, M: MapRef<S::Value>> SignalWithUntracked for MapSignal<S, M> {
	type Value = M::Value;

	fn with_untracked<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> O {
		self.0.with_untracked(|v| f(self.1.map_ref(v)))
	}

	fn try_with_untracked<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> Option<O> {
		self.0.try_with_untracked(|v| f(self.1.map_ref(v)))
	}
}

impl<S: SignalGet, M: MapValue<S::Value>> FnOnce<()> for MapSignal<S, M> {
	type Output = <Self as SignalGet>::Value;

	extern "rust-call" fn call_once(mut self, args: ()) -> Self::Output {
		self.call_mut(args)
	}
}

impl<S: SignalGet, M: MapValue<S::Value>> FnMut<()> for MapSignal<S, M> {
	extern "rust-call" fn call_mut(&mut self, args: ()) -> Self::Output {
		self.call(args)
	}
}

impl<S: SignalGet, M: MapValue<S::Value>> Fn<()> for MapSignal<S, M> {
	extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
		self.get()
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlatMapSignal<S, M>(pub S, pub M);

impl<S: SignalGet, M: MapValue<S::Value, Value: SignalGet>> SignalGet for FlatMapSignal<S, M> {
	type Value = <M::Value as SignalGet>::Value;

	fn get(&self) -> Self::Value {
		self.1.map_value(self.0.get()).get()
	}

	fn try_get(&self) -> Option<Self::Value> {
		self
			.0
			.try_get()
			.map(|v| self.1.map_value(v))
			.and_then(|v| v.try_get())
	}
}

impl<S: SignalGet, M: MapValue<S::Value, Value: SignalGet>> FnOnce<()> for FlatMapSignal<S, M> {
	type Output = <Self as SignalGet>::Value;

	extern "rust-call" fn call_once(mut self, args: ()) -> Self::Output {
		self.call_mut(args)
	}
}

impl<S: SignalGet, M: MapValue<S::Value, Value: SignalGet>> FnMut<()> for FlatMapSignal<S, M> {
	extern "rust-call" fn call_mut(&mut self, args: ()) -> Self::Output {
		self.call(args)
	}
}

impl<S: SignalGet, M: MapValue<S::Value, Value: SignalGet>> Fn<()> for FlatMapSignal<S, M> {
	extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
		self.get()
	}
}
