/// A type for which no instances are equal.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Hash)]
pub struct Distinct<T>(pub T);

impl<T> Distinct<T> {
	pub fn from_ref(v: &T) -> &Self {
		// SAFETY: `Distinct` is `repr(transparent)`.
		unsafe { std::mem::transmute(v) }
	}
}

impl<T> PartialEq for Distinct<T> {
	fn eq(&self, _other: &Self) -> bool {
		false
	}
}

impl<T> Eq for Distinct<T> {}

impl<T> std::ops::Deref for Distinct<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
