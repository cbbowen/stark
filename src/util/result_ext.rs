pub trait ResultExt<T, E> {
	fn ok_or_log(self) -> Option<T>
	where
		E: std::fmt::Display;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
	fn ok_or_log(self) -> Option<T>
	where
		E: std::fmt::Display,
	{
		self.inspect_err(|err| tracing::error!("{}", err)).ok()
	}
}
