use crate::components::*;
use crate::*;
use leptos::prelude::*;
use leptos_meta::*;
use leptos::children::Children;
use thaw::{Card, CardHeader, CardPreview};
use leptos_router::components::A;
use std::rc::Rc;
use crate::util::*;

#[component]
pub fn ShaderModulesProvider(children: Children) -> impl IntoView {
	let context: Rc<WgpuContext> = use_yolo_context();
	let resources = YoloValue::new(render::Resources::new(context.device()));
	
	use leptos::context::Provider;
	view! {
		<Provider value=resources>
			{children()}
		</Provider>
	}
}

#[component]
pub fn Home() -> impl IntoView {
	let drawing_color = RwSignal::new(glam::Vec3::new(0.5, 0.0, 0.0));

	view! {
		<Title text="Home"/>
		<KeyboardStateProvider>
			<RenderContextProvider
				initializing_fallback=|| {
					view! { <fallback::Initializing /> }
				}
				error_fallback=|errors| {
					let errors = errors.get();
					view! { <fallback::ErrorList errors></fallback::ErrorList> }
				}>
				<ShaderModulesProvider>
					<Canvas drawing_color=drawing_color/>
					<Card class="ColorPickerCard">
						<CardHeader>
							"Color Picker"
						</CardHeader>
						<CardPreview>
							<ColorPicker color=drawing_color/>
						</CardPreview>
					</Card>
				</ShaderModulesProvider>
			</RenderContextProvider>
		</KeyboardStateProvider>
	}
}

#[component]
pub fn NotFound() -> impl IntoView {
	let path = leptos_router::hooks::use_location().pathname.get();

	view! {
		<Title text="Not found"/>
		<div class="NotFound">
			<div>{ format!("Not found: {path}") }</div>
			<A href="/">Return home</A>
		</div>
	}
}
