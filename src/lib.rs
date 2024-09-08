#![feature(error_generic_member_access)]

pub(crate) mod util;

mod components;
mod engine;
mod geom;
mod pages;
mod render;
pub mod shaders;

mod wgpu_context;
pub use wgpu_context::*;

#[cfg(test)]
pub mod test;

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::*;
use leptos_router::components::*;

#[component]
pub fn App() -> impl IntoView {
	provide_meta_context();

	view! {
		<Html/>

		<Title formatter=|page| format!("Stark - {page}")/>

		// Inject metadata in the <head> tag.
		<Meta charset="UTF-8"/>
		<Meta name="viewport" content="width=device-width, initial-scale=1.0"/>

		<pages::Home/>
		// <Router>
		// 	<Routes>
		// 		// TODO: Figure out best to handle routes. When deployed on Github pages, this will be under /stark, but when testing locally with trunk, it won't.
		// 		<Route path="/stark" view=pages::Home/>
		// 		<Route path="/*" view=|| view! { <Redirect path="/stark"/> }/>
		// 	</Routes>
		// </Router>
	}
}
