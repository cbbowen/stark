use crate::components::*;
use crate::*;
use leptos::children::Children;
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::A;
use std::sync::Arc;
use thaw::{Card, CardHeader, CardPreview};

#[component]
pub fn ShaderModulesProvider(children: Children) -> impl IntoView {
	let context: Arc<WgpuContext> = use_context().unwrap();
	let resources = Arc::new(render::Resources::new(context.device()));

	use leptos::context::Provider;
	view! { <Provider value=resources>{children()}</Provider> }
}

#[component]
pub fn Home() -> impl IntoView {
	let drawing_color = RwSignal::new(glam::Vec3::new(0.5, 0.0, 0.0));
	let brush_size = RwSignal::new(0.5);

	view! {
		<Title text="Home"/>
		<KeyboardStateProvider>
			<RenderContextProvider initializing_fallback=|| {
				view! { <fallback::Initializing></fallback::Initializing> }
			}>
				<ShaderModulesProvider>

					<Canvas drawing_color=drawing_color brush_size=brush_size/>

					<Card class="ColorPickerCard">
						<CardHeader>
							<thaw::Body1>
								<b>"Color Picker"</b>
							</thaw::Body1>
						</CardHeader>
						<CardPreview>
							<ColorPicker color=drawing_color/>
						</CardPreview>
					</Card>

					<Card class="BrushSizeCard">
						<CardHeader>
							<thaw::Body1>
								<b>"Brush Size"</b>
							</thaw::Body1>
						</CardHeader>
						<CardPreview>
							<thaw::Slider value=brush_size min=0.01 max=1.0 step=0.05/>
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
			<div>{format!("Not found: {path}")}</div>
			<A href="/">Return home</A>
		</div>
	}
}
