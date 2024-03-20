use super::DistinctSignal;
use leptos::*;

/// Similar to `leptos::Trigger` but with a value type where every instance is distinct. This prevents `leptos::Memo` from hiding the triggers.
pub type DistinctTrigger = DistinctSignal<Trigger>;

pub fn create_distinct_trigger() -> DistinctTrigger {
	DistinctSignal::new(create_trigger())
}

impl Default for DistinctTrigger {
	fn default() -> Self {
		create_distinct_trigger()
	}
}
