use crate::{util::*, WgpuContext};
use leptos::children::ChildrenFn;
use leptos::context::Provider;
use leptos::prelude::*;

/// Unconditionally provides a `render::Context` context to its descendants. All `RenderCanvas`'s
/// should have this as an ancestor.
#[component]
pub fn RenderContextProvider(
	#[prop(optional, into)] initializing_fallback: ViewFnOnce,
	children: ChildrenFn,
) -> impl IntoView
{
	let resource = async { YoloValue::new(WgpuContext::new().await.unwrap()) };
	let resource = LocalResource::new(yolo_fn_once_to_fn(move || resource));

	view! {
		<Suspense fallback=initializing_fallback>
			{move || {
				let children = children.clone();
				Suspend::new(async move {
					let resource = resource.await;
					let children = children.clone();
					view! { <Provider value=resource>{children()}</Provider> }
				})
			}}
		</Suspense>
	}
}
