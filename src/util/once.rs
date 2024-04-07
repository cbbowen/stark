// use std::future::Future;

use leptos::prelude::*;

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

/// Similar to `leptos::Memo` but only changes once similarly to `std::cell::OnceCell`.
///
/// There are two important differences between `OnceMemo` and `leptos::Memo`:
/// 1. `OnceMemo` only produces the first non-`None` value returned by `f`. After this, `f` is not
///    called again, and it will not notify its dependents.
/// 2. `OnceMemo` does not require the signal value to implement `PartialEq`.
///
/// This is useful for things that are only expected to change once, like the element of a `NodeRef`
/// and whether or not it is mounted.
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

	pub fn map<U: 'static, F: FnOnce(&T) -> U + 'static>(self, f: F) -> OnceMemo<U> {
		let f = std::cell::Cell::new(Some(f));
		OnceMemo::new(move || self.with(|ot| ot.as_ref().map(|t| f.take().unwrap()(t))))
	}

	pub fn flat_map_once<U: 'static, F: Fn(&T) -> Option<U> + 'static>(self, f: F) -> OnceMemo<U> {
		OnceMemo::new(move || self.with(|ot| ot.as_ref().and_then(&f)))
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

// /// Similar to `leptos::RwSignal` but only changes once similarly to `std::cell::OnceCell`.
// pub struct OnceSignal<T: 'static> {
// 	inner: RwSignal<Option<T>>,
// }

// impl<T> OnceSignal<T> {
// 	pub fn new() -> Self {
// 		Self {
// 			inner: RwSignal::new(None),
// 		}
// 	}

// 	pub fn try_init(&self, f: impl FnOnce() -> T) -> bool {
// 		self
// 			.inner
// 			.try_update_untracked(move |r| {
// 				r.is_none()
// 					.then(move || {
// 						*r = Some(f());
// 					})
// 					.is_some()
// 			})
// 			.unwrap_or(false)
// 	}

// 	pub fn try_set(&self, value: T) -> bool {
// 		self.try_init(move || value)
// 	}

// 	pub fn map<U: 'static, F: FnOnce(&T) -> U + 'static>(self, f: F) -> OnceMemo<U> {
// 		let f = std::cell::Cell::new(Some(f));
// 		OnceMemo::new(move || {
// 			self.with(|ot| ot.as_ref().map(|t| f.take().unwrap()(t)))
// 		})
// 	}
// }

// impl<T: Clone> SignalGetUntracked for OnceSignal<T> {
// 	type Value = Option<T>;

// 	fn get_untracked(&self) -> Self::Value {
// 		self.inner.get_untracked()
// 	}

// 	fn try_get_untracked(&self) -> Option<Self::Value> {
// 		self.inner.try_get_untracked()
// 	}
// }

// impl<T: Clone> SignalGet for OnceSignal<T> {
// 	type Value = Option<T>;

// 	fn get(&self) -> Self::Value {
// 		self.get_untracked().or_else(|| self.inner.get())
// 	}

// 	fn try_get(&self) -> Option<Self::Value> {
// 		let result = self.try_get_untracked()?;
// 		if result.is_some() {
// 			return Some(result);
// 		}
// 		return self.inner.try_get();
// 	}
// }

// impl<T> SignalWithUntracked for OnceSignal<T> {
// 	type Value = Option<T>;

// 	fn with_untracked<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> O {
// 		self.inner.with_untracked(f)
// 	}

// 	fn try_with_untracked<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> Option<O> {
// 		self.inner.try_with_untracked(f)
// 	}
// }

// impl<T> SignalWith for OnceSignal<T> {
// 	type Value = Option<T>;

// 	fn with<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> O {
// 		self.with_untracked(move |r| {
// 			if r.is_some() {
// 				f(r)
// 			} else {
// 				self.inner.with(f)
// 			}
// 		})
// 	}

// 	fn try_with<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> Option<O> {
// 		self
// 			.try_with_untracked(move |r| {
// 				if r.is_some() {
// 					Some(f(r))
// 				} else {
// 					self.inner.try_with(f)
// 				}
// 			})
// 			.and_then(|r| r)
// 	}
// }

// /// Similar to `leptos::Trigger` but only triggers once.
// #[derive(Clone)]
// pub struct OnceTrigger {
// 	trigger: leptos::Trigger,
// 	flag: std::rc::Rc<std::cell::OnceCell<()>>,
// }

// impl OnceTrigger {
// 	pub fn new() -> Self {
// 		OnceTrigger {
// 			trigger: leptos::create_trigger(),
// 			flag: std::rc::Rc::new(std::cell::OnceCell::new()),
// 		}
// 	}

// 	pub fn try_notify(&self) -> bool {
// 		self.flag.get().is_none() && self.trigger.try_notify()
// 	}

// 	pub fn notify(&self) {
// 		self.try_notify();
// 	}

// 	pub fn try_track(&self) -> bool {
// 		self.trigger.try_track()
// 	}

// 	pub fn track(&self) {
// 		self.try_track();
// 	}
// }

// impl SignalGetUntracked for OnceTrigger {
// 	type Value = bool;

// 	fn get_untracked(&self) -> Self::Value {
// 		self.flag.get().is_some()
// 	}

// 	fn try_get_untracked(&self) -> Option<Self::Value> {
// 		Some(self.get_untracked())
// 	}
// }

// impl SignalGet for OnceTrigger {
// 	type Value = bool;

// 	fn get(&self) -> Self::Value {
// 		let result = self.get_untracked();
// 		if !result {
// 			self.try_track();
// 		}
// 		result
// 	}

// 	fn try_get(&self) -> Option<Self::Value> {
// 		Some(self.get())
// 	}
// }

// pub struct AsyncOnceSignal<T> {
// 	cell: async_once_cell::OnceCell<T>,
// 	trigger: OnceTrigger,
// }

// impl<T> AsyncOnceSignal<T> {
// 	pub fn new() -> Self {
// 		Self {
// 			cell: async_once_cell::OnceCell::new(),
// 			trigger: OnceTrigger::new(),
// 		}
// 	}

// 	pub async fn get_or_try_init_untracked<E>(
// 		&self,
// 		init: impl Future<Output = Result<T, E>>,
// 	) -> Result<&T, E> {
// 		let result = self.cell.get_or_try_init(init).await?;
// 		self.trigger.try_notify();
// 		Ok(result)
// 	}

// 	pub async fn get_or_try_init<E>(
// 		&self,
// 		init: impl Future<Output = Result<T, E>>,
// 	) -> Result<&T, E> {
// 		let result = self.get_or_try_init_untracked(init).await;
// 		if result.is_err() {
// 			self.trigger.try_track();
// 		}
// 		result
// 	}

// 	pub async fn get_or_init(&self, init: impl Future<Output = T>) -> &T {
// 		self
// 			.get_or_try_init_untracked::<std::convert::Infallible>(async { Ok(init.await) })
// 			.await
// 			.unwrap()
// 	}
// }

// impl<'a, T> leptos::SignalWithUntracked for &'a AsyncOnceSignal<T> {
// 	type Value = Option<&'a T>;

// 	fn try_with_untracked<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> Option<O> {
// 		Some(self.with_untracked(f))
// 	}

// 	fn with_untracked<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> O {
// 		f(&self.cell.get())
// 	}
// }

// impl<'a, T> leptos::SignalWith for &'a AsyncOnceSignal<T> {
// 	type Value = Option<&'a T>;

// 	fn try_with<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> Option<O> {
// 		Some(self.with(f))
// 	}

// 	fn with<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> O {
// 		self.trigger.try_track();
// 		self.with_untracked(f)
// 	}
// }

// impl<T: Clone> leptos::SignalGet for AsyncOnceSignal<T> {
// 	type Value = Option<T>;

// 	fn try_get(&self) -> Option<Self::Value> {
// 		self.try_with(|o| o.cloned())
// 	}

// 	fn get(&self) -> Self::Value {
// 		self.with(|o| o.cloned())
// 	}
// }

// impl<T: Clone> leptos::SignalGetUntracked for AsyncOnceSignal<T> {
// 	type Value = Option<T>;

// 	fn try_get_untracked(&self) -> Option<Self::Value> {
// 		self.try_with_untracked(|o| o.cloned())
// 	}

// 	fn get_untracked(&self) -> Self::Value {
// 		self.with_untracked(|o| o.cloned())
// 	}
// }

// // impl<'a, T> leptos::SignalGet for &'a AsyncOnceSignal<T> {
// // 	type Value = Option<&'a T>;

// // 	fn try_get(&self) -> Option<Self::Value> {
// // 		let result = self.cell.get();
// // 		if result.is_none() && !self.trigger.try_track() {
// // 			None
// // 		} else {
// // 			Some(result)
// // 		}
// // 	}

// // 	fn get(&self) -> Self::Value {
// // 		let result = self.cell.get();
// // 		if result.is_none() {
// // 			self.trigger.track()
// // 		}
// // 		result
// // 	}
// // }

// // impl<'a, T> leptos::SignalGetUntracked for &'a AsyncOnceSignal<T> {
// // 	type Value = Option<&'a T>;

// // 	fn try_get_untracked(&self) -> Option<Self::Value> {
// // 		Some(self.get_untracked())
// // 	}

// // 	fn get_untracked(&self) -> Self::Value {
// // 		self.cell.get()
// // 	}
// // }

// #[cfg(test)]
// mod test {
// 	use super::*;

// 	#[test]
// 	fn test_once_trigger() {
// 		let trigger = OnceTrigger::new();
// 		assert_eq!(trigger.get_untracked(), false);
// 		trigger.notify();
// 		assert_eq!(trigger.get(), true);
// 	}

// 	#[test]
// 	fn test_async_once_signal() {
// 		use pollster::FutureExt;
// 		let signal = AsyncOnceSignal::new();
// 		let future = signal.get_or_init(async { () });
// 		assert!(signal.get_untracked().is_none());
// 		future.block_on();
// 		assert!(signal.get_untracked().is_some());
// 	}
// }
