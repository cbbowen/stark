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
use leptos_router::components::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
	provide_meta_context();

	let configuration = leptos::prelude::get_configuration(None).unwrap();
	let options = configuration.leptos_options;

	view! {
		<!DOCTYPE html>
		<html lang="en">
			<head>
				<meta charset="utf-8"/>
				<meta name="viewport" content="width=device-width, initial-scale=1"/>
				<Title formatter=|page| format!("Stark - {page}")/>
				<AutoReload options=options.clone()/>
				<HydrationScripts options/>
				<MetaTags/>
			</head>
			<body>
				<pages::Home />
				// <Router>
				// <Routes>
				// // TODO: Figure out best to handle routes. When deployed on Github pages, this will be under /stark, but when testing locally with trunk, it won't.
				// <Route path="/stark" view=pages::Home/>
				// <Route path="/*" view=|| view! { <Redirect path="/stark"/> }/>
				// </Routes>
				// </Router>
			</body>
		</html>
	}
}
