use crate::util::*;
use leptos::*;
use std::future::Future;

#[component]
pub fn AsyncProvider<ContextFuture, Context, ContextError, ErrorFallback, ErrorFallbackResult>(
	context: ContextFuture,
	#[prop(optional, into)] initializing_fallback: ViewFn,
	error_fallback: ErrorFallback,
	children: ChildrenFn,
) -> impl IntoView
where
	Context: Clone + 'static,
	ContextError: Clone + Into<leptos::error::Error> + 'static,
	ContextFuture: Future<Output = Result<Context, ContextError>> + 'static,
	ErrorFallback: Fn(RwSignal<Errors>) -> ErrorFallbackResult + 'static,
	ErrorFallbackResult: IntoView + 'static,
{
	// This is a bit of a hack to work around the lack of support for `FnOnce` resources.
	let context = std::cell::Cell::new(Some(context));
	let context = create_local_resource(|| (), move |_| context.take().unwrap());

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
			}>
				{children}
			</ErrorBoundary>
		</Suspense>
	}
}

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
	let context = async { crate::WgpuContext::new().await.map(std::rc::Rc::new) };
	view! {
		<AsyncProvider context initializing_fallback error_fallback>
			{children.clone()}
		</AsyncProvider>
	}
}
