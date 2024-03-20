use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapDistinct;

impl<T> MapValue<T> for MapDistinct {
	type Value = Distinct<T>;
	fn map_value(&self, from: T) -> Self::Value {
		Distinct(from)
	}
}

impl<T> MapRef<T> for MapDistinct {
	type Value = Distinct<T>;
	fn map_ref<'a>(&self, from: &'a T) -> &'a Self::Value
	where
		T: 'a,
	{
		Distinct::from_ref(from)
	}
}

pub type DistinctSignal<S> = MapSignal<S, MapDistinct>;

impl<S> DistinctSignal<S> {
	pub fn new(s: S) -> Self {
		MapSignal(s, MapDistinct)
	}
}
