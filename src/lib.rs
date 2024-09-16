#![feature(error_generic_member_access)]
#![feature(generic_const_exprs)]
#![feature(maybe_uninit_uninit_array)]
#![feature(maybe_uninit_array_assume_init)]

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

	let theme = RwSignal::new(thaw::Theme::dark());

	view! {
		<thaw::ConfigProvider theme=theme>
			<Router>
				<Routes fallback=pages::NotFound>
					// TODO: Figure out best to handle routes. When deployed on Github pages, this will be under /stark, but when testing locally with trunk, it won't.
					<Route path=path!("/stark") view=pages::Home/>
					<Route path=path!("/*") view=|| view! { <Redirect path="/stark"/> }/>
				</Routes>
			</Router>
		</thaw::ConfigProvider>
	}
}
