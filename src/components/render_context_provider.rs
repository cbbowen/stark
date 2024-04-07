use super::*;
use crate::util::*;
use leptos::*;

/// Unconditionally provides a `render::Context` context to its descendants. All `RenderCanvas`'s
/// should have this as an ancestor.
#[component]
pub fn RenderContextProvider(children: ChildrenFn) -> impl IntoView {
	let context = leptos::create_local_resource(|| (), |_| crate::render::Context::new());
	let children = move || {
		let children = children.clone();
		context.get().as_ref().map(move |context| {
			let context = context.clone();
			let children = children.clone();
			context.map(move |context| {
				view! { <Provider value=context>{children()}</Provider> }
			})
		})
	};
	let children = create_cache(children);

	view! {
		<Suspense fallback=|| {
			view! { <Waiting/> }
		}>
			<ErrorBoundary fallback=|errors| {
				view! { <ErrorList errors/> }
			}>
				{children}
			</ErrorBoundary>
		</Suspense>
	}
}
