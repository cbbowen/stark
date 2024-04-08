use crate::util::*;
use leptos::*;

/// Unconditionally provides a `render::Context` context to its descendants. All `RenderCanvas`'s
/// should have this as an ancestor.
#[component]
pub fn RenderContextProvider<ErrorFallback, ErrorFallbackResult>(
	#[prop(optional, into)] initializing_fallback: ViewFn,
	error_fallback: ErrorFallback,
	children: ChildrenFn,
) -> impl IntoView
where
	ErrorFallback: Fn(RwSignal<Errors>) -> ErrorFallbackResult + 'static,
	ErrorFallbackResult: IntoView + 'static,
{
	let context = create_local_resource(|| (), |_| crate::render::Context::new());
	let children = create_derived(move || {
		let children = children.clone();
		context.get().as_ref().map(move |context| {
			let context = context.clone();
			let children = children.clone();
			context.map(move |context| {
				view! { <Provider value=context>{children()}</Provider> }
			})
		})
	});

	let error_fallback = Callback::new(error_fallback);

	view! {
		<Suspense fallback=initializing_fallback>
			<ErrorBoundary fallback=move |errors| {
				error_fallback.call(errors)
			}>{children}</ErrorBoundary>
		</Suspense>
	}
}
