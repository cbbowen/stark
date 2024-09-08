use crate::{util::*, WgpuContext};
use leptos::children::ChildrenFn;
use leptos::context::Provider;
use leptos::prelude::*;

// #[component]
// pub fn AsyncProvider<ContextFuture, Context, ContextError, ErrorFallback, ErrorFallbackResult>(
// 	context: ContextFuture,
// 	#[prop(optional, into)] initializing_fallback: ViewFnOnce,
// 	error_fallback: ErrorFallback,
// 	children: ChildrenFn,
// ) -> impl IntoView
// where
// 	Context: Clone + Send + Sync + 'static,
// 	ContextError: Clone + Into<leptos::error::Error> + Send + Sync + 'static,
// 	ContextFuture: Future<Output = Result<Context, ContextError>> + Send + Sync + 'static,
// 	ErrorFallback: Fn(ArcRwSignal<Errors>) -> ErrorFallbackResult + Send + Sync + 'static,
// 	ErrorFallbackResult: IntoView + 'static,
// {
// 	// This is a bit of a hack to work around the lack of support for `FnOnce` resources.
// 	let context = std::cell::Cell::new(Some(context));
// 	let context = LocalResource::new(move || context.take().unwrap());

// 	let children = move || {
// 		let children = children.clone();
// 		context.with(move |context| {
// 			context.as_ref().map(move |context| {
// 				let context = context.clone();
// 				let children = children.clone();
// 				context.map(move |context| {
// 					view! { <Provider value=context>{children()}</Provider> }
// 				})
// 			})
// 		})
// 	};
// 	let children = create_derived(children);

// 	let error_fallback = Callback::new(error_fallback);

// 	view! {
// 		<Suspense fallback=move || initializing_fallback.run()>
// 			<ErrorBoundary fallback=move |errors| {
// 				error_fallback.run(errors)
// 			}>
// 				{children}
// 			</ErrorBoundary>
// 		</Suspense>
// 	}
// }

/// Unconditionally provides a `render::Context` context to its descendants. All `RenderCanvas`'s
/// should have this as an ancestor.
#[component]
pub fn RenderContextProvider<ErrorFallback, ErrorFallbackResult>(
	#[prop(optional, into)] initializing_fallback: ViewFnOnce,
	error_fallback: ErrorFallback,
	children: ChildrenFn,
) -> impl IntoView
where
	ErrorFallback: Fn(ArcRwSignal<Errors>) -> ErrorFallbackResult + Send + 'static,
	ErrorFallbackResult: IntoView + Send + 'static,
{
	// let context = async { Arc::new(SendWrapper::new(Rc::new(WgpuContext::new().await.unwrap())))
	// };

	// let resource = async { "Awaited" };
	let resource = async { YoloValue::new(WgpuContext::new().await.unwrap()) };
	let resource = SendWrapperFuture::new(resource);
	let resource = LocalResource::new(yolo_fn_once_to_fn(move || resource));

	view! {
		<Suspense fallback=|| {
			view! { "Fallback" }
		}>
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

	// use std::cell::Cell;
	// let context = async { SendWrapper::new(Rc::new(WgpuContext::new().await.unwrap())) };
	// let context: Box<dyn Future<Output = SendWrapper<Rc<WgpuContext>>> + Send> =
	// Box::new(context); let context = SendWrapper::new(Cell::new(Some(context)));
	// let context = async move {
	// 	let context = context.take();
	// 	let context = context.take().unwrap();
	// 	let context = Box::into_pin(context);
	// 	context.await
	// };

	// use leptos::prelude::Suspend;
	// view! {
	// 	{Suspend::new(async move {
	// 		let context = context.await;
	// 		let context = Arc::new(context);
	// 		view! {
	// 			<Provider value=context>
	// 				"blah"
	// 			</Provider>
	// 		}
	// 	})}
	// }

	// let context = Arc::new(std::sync::Mutex::new(std::cell::Cell::new(Some(context))));
	// let context = LocalResource::new(move || context.lock().unwrap().take().unwrap());
	// let children = move || {
	// 	let children = children.clone();
	// 	let context = context.get()?;
	// 	let context = Arc::new(context);
	// 	Some(view! { <Provider value=context>{children()}</Provider> })
	// };

	// view! {
	// 	<AsyncProvider context initializing_fallback error_fallback>
	// 		{children.clone()}
	// 	</AsyncProvider>
	// }

	// use leptos::suspense::Suspense;
	// view! {
	// 	<Suspense fallback=initializing_fallback>{children}// "Blah"
	// 	// <ErrorBoundary fallback=error_fallback>
	// 	// "Blah"
	// 	// // {children}
	// 	// </ErrorBoundary>
	// 	</Suspense>
	// }
}
