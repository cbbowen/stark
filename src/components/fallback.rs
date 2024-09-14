use leptos::prelude::*;

#[component]
pub fn Initializing() -> impl IntoView {
	view! { "Initializing..." }
}

#[component]
pub fn ErrorList(#[prop(into)] errors: ArcSignal<Errors>) -> impl IntoView {
	view! {
		<ul>
			{move || {
				let errors: Vec<_> = errors.with(move |errors|
					errors
					.iter()
					.map(|(_, e)| e.to_string()).collect());
				tracing::warn!(?errors, "ErrorList::view");

				errors
					.into_iter()
					.map(|e| view! { <li>{e}</li> })
					.collect_view()
			}}
		</ul>
	}
}
