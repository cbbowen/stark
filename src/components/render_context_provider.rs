use crate::WgpuContext;
use leptos::children::ChildrenFn;
use leptos::context::Provider;
use leptos::prelude::*;
use std::sync::Arc;

/// Unconditionally provides a `render::Context` context to its descendants. All `RenderCanvas`'s
/// should have this as an ancestor.
#[component]
pub fn RenderContextProvider(
	#[prop(optional, into)] initializing_fallback: ViewFnOnce,
	children: ChildrenFn,
) -> impl IntoView {
	let resource =
		LocalResource::new(|| async { Arc::new(WgpuContext::new().await.unwrap()) });

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
