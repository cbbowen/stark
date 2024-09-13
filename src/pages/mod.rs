use crate::components::*;
use crate::*;
use leptos::children::Children;
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::A;
use std::sync::Arc;
use thaw::Body1;

#[component]
pub fn ShaderModulesProvider(children: Children) -> impl IntoView {
	let context: Arc<WgpuContext> = use_context().unwrap();
	let resources = Arc::new(render::Resources::new(context.device()));

	use leptos::context::Provider;
	view! { <Provider value=resources>{children()}</Provider> }
}

#[component]
pub fn Home() -> impl IntoView {
	let brush_color = RwSignal::new(glam::Vec3::new(0.5, 0.0, 0.0));
	let brush_size = RwSignal::new(0.2);
	let brush_opacity = RwSignal::new(0.05);
	let brush_softness = RwSignal::new(1.0);

	view! {
		<Title text="Home"/>
		<KeyboardStateProvider>
			<RenderContextProvider initializing_fallback=|| {
				view! { <fallback::Initializing></fallback::Initializing> }
			}>
				<ShaderModulesProvider>

					<Canvas
						brush_color=brush_color
						brush_size=brush_size
						brush_opacity=brush_opacity
						brush_softness=brush_softness
					/>

					<div class="SidePanels">

						<Panel title="Color">
							<ColorPicker color=brush_color/>
						</Panel>

						<Panel title="Brush">
							<BrushSetting name="Size">
								<thaw::Slider
									value=brush_size
									min=0.0
									max=0.25
									step=0.01
								/>
							</BrushSetting>
							<BrushSetting name="Opacity">
								<thaw::Slider
									value=brush_opacity
									min=0.0
									max=1.0
									step=0.05
								/>
							</BrushSetting>
							<BrushSetting name="Softness">
								<thaw::Slider
									value=brush_softness
									min=0.1
									max=4.0
									step=0.2
								/>
							</BrushSetting>
						</Panel>

					</div>

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
