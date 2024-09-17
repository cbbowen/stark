use leptos::prelude::*;
use thiserror::Error;

use super::ResultExt;

#[derive(Error, Debug)]
#[error("signal disposed")]
struct LeptosTryError;

impl LeptosTryError {
	fn new() -> Self {
		// #[cfg(debug_assertions)]
		// panic!("signal disposed");

		#[allow(unreachable_code)]
		Self
	}
}

pub trait SetExt: Set {
	fn try_set_or_log(&self, value: Self::Value) -> Option<Self::Value>;
}

impl<S> SetExt for S
where
	Self: Set,
{
	fn try_set_or_log(&self, value: Self::Value) -> Option<Self::Value> {
		self
			.try_set(value)
			.ok_or_else(LeptosTryError::new)
			.ok_or_log()
	}
}

pub trait GetExt: Get {
	fn try_get_or_log(&self) -> Option<Self::Value>;
}

impl<S> GetExt for S
where
	Self: Get,
{
	fn try_get_or_log(&self) -> Option<Self::Value> {
		self.try_get().ok_or_else(LeptosTryError::new).ok_or_log()
	}
}
