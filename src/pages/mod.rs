use crate::components::*;
use leptos::*;
use leptos_meta::*;

#[component]
pub fn Home() -> impl IntoView {
	view! {
		<Title text="Home"/>
		<RenderContextProvider
			initializing_fallback=|| {
				view! { <fallback::Initializing></fallback::Initializing> }
			}
			error_fallback=|errors| {
				view! { <fallback::ErrorList errors></fallback::ErrorList> }
			}>
			<div class="Home">
				<Canvas/>
			</div>
		</RenderContextProvider>
	}
}

#[component]
pub fn NotFound() -> impl IntoView {
	view! {
		<Title text="Not found"/>
		<div class="NotFound">"Not found"</div>
	}
}
