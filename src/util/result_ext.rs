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
