use crate::components::*;
use leptos::*;
use leptos_meta::*;
use crate::*;

#[component]
pub fn ShaderModulesProvider(children: Children) -> impl IntoView
{
	run_as_child(move || {
		let context: std::rc::Rc<WgpuContext> = expect_context();
		provide_context(render::Resources::new(context.device()));
		children()
	})
}

#[component]
pub fn Home() -> impl IntoView {
	view! {
		<Title text="Home"/>
		<RenderContextProvider
			initializing_fallback=|| {
				view! { <fallback::Initializing /> }
			}
			error_fallback=|errors| {
				view! { <fallback::ErrorList errors></fallback::ErrorList> }
			}>
			<ShaderModulesProvider>
				<div class="Home">
					<Canvas/>
					<ColorPicker/>
				</div>
			</ShaderModulesProvider>
		</RenderContextProvider>
	}
}

#[component]
pub fn NotFound() -> impl IntoView {
	let path = use_location().pathname.get();

	view! {
		<Title text="Not found"/>
		<div class="NotFound">
			<div>{ format!("Not found: {path}") }</div>
			<A href="/">Return home</A>
		</div>
	}
}
