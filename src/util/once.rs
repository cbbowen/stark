// use std::future::Future;

use leptos::prelude::*;

/// Similar to `leptos::Memo` but only changes once similarly to `std::cell::OnceCell`.
///
/// There are two important differences between `OnceMemo` and `leptos::Memo`:
/// 1. `OnceMemo` only produces the first non-`None` value returned by `f`. After this, `f` is not
///    called again, and it will not notify its dependents.
/// 2. `OnceMemo` does not require the signal value to implement `PartialEq`.
///
/// This is useful for things that are only expected to change once, like the element of a `NodeRef`
/// and whether or not it is mounted.
#[derive(Debug)]
pub struct OnceMemo<T: 'static> {
	inner: Memo<Option<T>>,
}

impl<T> Clone for OnceMemo<T> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
		}
	}
}

impl<T> Copy for OnceMemo<T> {}

impl<T> OnceMemo<T> {
	pub fn new(f: impl Fn() -> Option<T> + 'static) -> Self {
		let inner = create_owning_memo(move |p| {
			if let Some(Some(p)) = p {
				(Some(p), false)
			} else {
				let v = f();
				let changed = v.is_some();
				(v, changed)
			}
		});
		Self { inner }
	}

	/// Creates a new signal that has the same value as applying the given mapping function to the
	/// value of this signal.
	///
	/// This is essentially equivalent to `create_derived(|| self.get().map(f))` except that `f` need
	/// only be `FnOnce`.
	pub fn map<U: 'static, F: FnOnce(&T) -> U + 'static>(self, f: F) -> OnceMemo<U> {
		let f = std::cell::Cell::new(Some(f));
		OnceMemo::new(move || self.with(|ot| ot.as_ref().map(|t| f.take().unwrap()(t))))
	}
}

impl<T: Clone> SignalGetUntracked for OnceMemo<T> {
	type Value = Option<T>;

	fn get_untracked(&self) -> Self::Value {
		self.inner.get_untracked()
	}

	fn try_get_untracked(&self) -> Option<Self::Value> {
		self.inner.try_get_untracked()
	}
}

impl<T: Clone> SignalGet for OnceMemo<T> {
	type Value = Option<T>;

	fn get(&self) -> Self::Value {
		self.get_untracked().or_else(|| self.inner.get())
	}

	fn try_get(&self) -> Option<Self::Value> {
		let result = self.try_get_untracked()?;
		if result.is_some() {
			return Some(result);
		}
		return self.inner.try_get();
	}
}

impl<T> SignalWithUntracked for OnceMemo<T> {
	type Value = Option<T>;

	fn with_untracked<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> O {
		self.inner.with_untracked(f)
	}

	fn try_with_untracked<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> Option<O> {
		self.inner.try_with_untracked(f)
	}
}

impl<T> SignalWith for OnceMemo<T> {
	type Value = Option<T>;

	fn with<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> O {
		self.with_untracked(move |r| {
			if r.is_some() {
				f(r)
			} else {
				self.inner.with(f)
			}
		})
	}

	fn try_with<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> Option<O> {
		self
			.try_with_untracked(move |r| {
				if r.is_some() {
					Some(f(r))
				} else {
					self.inner.try_with(f)
				}
			})
			.and_then(|r| r)
	}
}

impl<T> SignalDispose for OnceMemo<T> {
	fn dispose(self) {
		self.inner.dispose()
	}
}
