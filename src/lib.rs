#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(associated_type_bounds)]

pub(crate) mod util;

mod components;
mod pages;
mod renderables;

use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
	provide_meta_context();
	provide_context(std::rc::Rc::new(
		components::render_canvas::RenderContext::new(),
	));

	view! {
		<Html lang="en" dir="ltr" attr:data-theme="light"/>

		// sets the document title
		<Title text="Welcome to Leptos CSR"/>

		// injects metadata in the <head> of the page
		<Meta charset="UTF-8"/>
		<Meta name="viewport" content="width=device-width, initial-scale=1.0"/>

		<Router>
			<Routes>
				<Route path="/" view=pages::Home/>
				<Route path="/*" view=pages::NotFound/>
			</Routes>
		</Router>
	}
}
