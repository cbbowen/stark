use leptos::*;
use std::{cell::RefCell, collections::HashSet, rc::Rc};

#[derive(Default)]
struct InternalKeyboardState {
	pressed: HashSet<String>,
}

#[derive(Clone, Default)]
pub struct KeyboardState(Rc<RefCell<InternalKeyboardState>>);

impl KeyboardState {
	pub fn all_pressed(&self) -> HashSet<String> {
		self.0.as_ref().borrow().pressed.clone()
	}

	pub fn is_pressed(&self, key: &str) -> bool {
		self.0.as_ref().borrow().pressed.contains(key)
	}

	fn set_down(&self, key: String) -> bool {
		self.0.as_ref().borrow_mut().pressed.insert(key)
	}

	fn set_up(&self, key: &str) -> bool {
		self.0.as_ref().borrow_mut().pressed.remove(key)
	}
}

#[component]
pub fn KeyboardStateProvider(children: ChildrenFn) -> impl IntoView {
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

	let keydown_handle = leptos::window_event_listener(leptos::ev::keydown, keydown);
	let keyup_handle = leptos::window_event_listener(leptos::ev::keyup, keyup);
	leptos::on_cleanup(move || {
		keydown_handle.remove();
		keyup_handle.remove();
	});

	provide_context(state);
	view! { <div>{children}</div> }
}
