use crate::components::*;
use crate::*;
use leptos::*;
use leptos_meta::*;

#[component]
pub fn ShaderModulesProvider(children: Children) -> impl IntoView {
	run_as_child(move || {
		let context: std::rc::Rc<WgpuContext> = expect_context();
		provide_context(render::Resources::new(context.device()));
		children()
	})
}

#[component]
pub fn Home() -> impl IntoView {
	let drawing_color = leptos::create_rw_signal(glam::Vec3::new(0.5, 0.0, 0.0));

	view! {
		<Title text="Home"/>
		<KeyboardStateProvider>
			<RenderContextProvider
				initializing_fallback=|| {
					view! { <fallback::Initializing /> }
				}
				error_fallback=|errors| {
					view! { <fallback::ErrorList errors></fallback::ErrorList> }
				}>
				<ShaderModulesProvider>
					<div class="Home">
						<Canvas drawing_color=drawing_color/>
						<ColorPicker color=drawing_color/>
					</div>
				</ShaderModulesProvider>
			</RenderContextProvider>
		</KeyboardStateProvider>
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
