use leptos::*;

#[component]
pub fn Waiting() -> impl IntoView {
	view! { "Loading..." }
}

#[component]
pub fn ErrorList(#[prop(into)] errors: Signal<Errors>) -> impl IntoView {
	view! {
		<ul>
			{move || {
				errors.with(move |errors|
					errors
					.iter()
					.map(|(_, e)| view! { <li>{e.to_string()}</li> })
					.collect_view())
			}}
		</ul>
	}
}
