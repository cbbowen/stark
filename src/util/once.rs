use std::future::Future;

use leptos::prelude::*;

/// Similar to `leptos::Trigger` but only triggers once.
pub struct OnceTrigger {
	trigger: leptos::Trigger,
	flag: std::cell::Cell<bool>,
}

impl OnceTrigger {
	pub fn new() -> Self {
		OnceTrigger {
			trigger: leptos::create_trigger(),
			flag: std::cell::Cell::new(false),
		}
	}

	pub fn try_notify(&self) -> bool {
		!self.flag.replace(true) && self.trigger.try_notify()
	}

	pub fn notify(&self) {
		self.try_notify();
	}

	pub fn try_track(&self) -> bool {
		self.trigger.try_track()
	}

	pub fn track(&self) {
		self.try_track();
	}
}

impl SignalGetUntracked for OnceTrigger {
	type Value = bool;

	fn get_untracked(&self) -> Self::Value {
		self.flag.get()
	}

	fn try_get_untracked(&self) -> Option<Self::Value> {
		Some(self.get_untracked())
	}
}

impl SignalGet for OnceTrigger {
	type Value = bool;

	fn get(&self) -> Self::Value {
		let result = self.get_untracked();
		if !result {
			self.try_track();
		}
		result
	}

	fn try_get(&self) -> Option<Self::Value> {
		Some(self.get())
	}
}

pub struct AsyncOnceSignal<T> {
	cell: async_once_cell::OnceCell<T>,
	trigger: OnceTrigger,
}

impl<T> AsyncOnceSignal<T> {
	pub fn new() -> Self {
		Self {
			cell: async_once_cell::OnceCell::new(),
			trigger: OnceTrigger::new(),
		}
	}

	pub async fn get_or_try_init_untracked<E>(
		&self,
		init: impl Future<Output = Result<T, E>>,
	) -> Result<&T, E> {
		let result = self.cell.get_or_try_init(init).await?;
		self.trigger.try_notify();
		Ok(result)
	}

	pub async fn get_or_try_init<E>(
		&self,
		init: impl Future<Output = Result<T, E>>,
	) -> Result<&T, E> {
		let result = self.get_or_try_init_untracked(init).await;
		if result.is_err() {
			self.trigger.try_track();
		}
		result
	}

	pub async fn get_or_init(&self, init: impl Future<Output = T>) -> &T {
		self
			.get_or_try_init_untracked::<std::convert::Infallible>(async { Ok(init.await) })
			.await
			.unwrap()
	}
}

impl<'a, T> leptos::SignalWithUntracked for &'a AsyncOnceSignal<T> {
	type Value = Option<&'a T>;

	fn try_with_untracked<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> Option<O> {
		Some(self.with_untracked(f))
	}

	fn with_untracked<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> O {
		f(&self.cell.get())
	}
}

impl<'a, T> leptos::SignalWith for &'a AsyncOnceSignal<T> {
	type Value = Option<&'a T>;

	fn try_with<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> Option<O> {
		Some(self.with(f))
	}

	fn with<O>(&self, f: impl FnOnce(&Self::Value) -> O) -> O {
		self.trigger.try_track();
		self.with_untracked(f)
	}
}

impl<T: Clone> leptos::SignalGet for AsyncOnceSignal<T> {
	type Value = Option<T>;

	fn try_get(&self) -> Option<Self::Value> {
		self.try_with(|o| o.cloned())
	}

	fn get(&self) -> Self::Value {
		self.with(|o| o.cloned())
	}
}

impl<T: Clone> leptos::SignalGetUntracked for AsyncOnceSignal<T> {
	type Value = Option<T>;

	fn try_get_untracked(&self) -> Option<Self::Value> {
		self.try_with_untracked(|o| o.cloned())
	}

	fn get_untracked(&self) -> Self::Value {
		self.with_untracked(|o| o.cloned())
	}
}

// impl<'a, T> leptos::SignalGet for &'a AsyncOnceSignal<T> {
// 	type Value = Option<&'a T>;

// 	fn try_get(&self) -> Option<Self::Value> {
// 		let result = self.cell.get();
// 		if result.is_none() && !self.trigger.try_track() {
// 			None
// 		} else {
// 			Some(result)
// 		}
// 	}

// 	fn get(&self) -> Self::Value {
// 		let result = self.cell.get();
// 		if result.is_none() {
// 			self.trigger.track()
// 		}
// 		result
// 	}
// }

// impl<'a, T> leptos::SignalGetUntracked for &'a AsyncOnceSignal<T> {
// 	type Value = Option<&'a T>;

// 	fn try_get_untracked(&self) -> Option<Self::Value> {
// 		Some(self.get_untracked())
// 	}

// 	fn get_untracked(&self) -> Self::Value {
// 		self.cell.get()
// 	}
// }

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_once_trigger() {
		let trigger = OnceTrigger::new();
		assert_eq!(trigger.get_untracked(), false);
		trigger.notify();
		assert_eq!(trigger.get(), true);
	}

	#[test]
	fn test_async_once_signal() {
		use pollster::FutureExt;
		let signal = AsyncOnceSignal::new();
		let future = signal.get_or_init(async { () });
		assert!(signal.get_untracked().is_none());
		future.block_on();
		assert!(signal.get_untracked().is_some());
	}
}
