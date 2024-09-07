use std::fmt::Debug;

pub trait ResultExt<T, E> {
	fn ok_or_log(self) -> Option<T>
	where
		E: Debug;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
	fn ok_or_log(self) -> Option<T>
	where
		E: Debug,
	{
		self.inspect_err(|err| tracing::error!("{err:?}")).ok()
	}
}

pub trait OptionExt<T> {
	fn unwrap_or_default_and_log(self, error: &str) -> T
	where
		T: Default;
}

impl<T> OptionExt<T> for Option<T> {
	fn unwrap_or_default_and_log(self, error: &str) -> T
	where
		T: Default,
	{
		self.unwrap_or_else(|| {
			tracing::error!("{error}");
			T::default()
		})
	}
}
