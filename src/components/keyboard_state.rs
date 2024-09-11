use leptos::prelude::*;
use leptos::context::provide_context;
use std::{collections::HashSet, sync::Arc, sync::RwLock};

#[derive(Default)]
struct InternalKeyboardState {
	pressed: HashSet<String>,
}

#[derive(Clone, Default)]
pub struct KeyboardState(Arc<RwLock<InternalKeyboardState>>);

impl KeyboardState {
	pub fn all_pressed(&self) -> HashSet<String> {
		self.0.read().unwrap().pressed.clone()
	}

	pub fn is_pressed(&self, key: &str) -> bool {
		self.0.read().unwrap().pressed.contains(key)
	}

	fn set_down(&self, key: String) -> bool {
		self.0.write().unwrap().pressed.insert(key)
	}

	fn set_up(&self, key: &str) -> bool {
		self.0.write().unwrap().pressed.remove(key)
	}
}

#[component]
pub fn KeyboardStateProvider(children: Children) -> impl IntoView {
	let state = KeyboardState::default();
	let keydown = {
		let state = state.clone();
		move |e: leptos::ev::KeyboardEvent| {
			if !e.repeat() && !state.set_down(e.key()) {
				tracing::warn!(key = e.key(), "key already down");
			}
		}
	};
	let keyup = {
		let state = state.clone();
		move |e: leptos::ev::KeyboardEvent| {
			if !state.set_up(&e.key()) {
				tracing::warn!(key = e.key(), "key not down");
			}
		}
	};

	let keydown_handle = window_event_listener(leptos::ev::keydown, keydown);
	let keyup_handle = window_event_listener(leptos::ev::keyup, keyup);
	on_cleanup(move || {
		keydown_handle.remove();
		keyup_handle.remove();
	});

	provide_context(state);
	view! {
		<div tabindex="0" class="Provider">
			{children()}
		</div>
	}
}
