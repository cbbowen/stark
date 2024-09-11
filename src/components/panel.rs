use leptos::prelude::*;
use thaw::{Card, CardHeader, CardPreview, Body1};

#[component]
pub fn Panel(#[prop(into)] title: String, children: Children) -> impl IntoView {
	view! {
		<Card class="Panel">
			<CardHeader>
				<Body1>
					<b>{title}</b>
				</Body1>
			</CardHeader>
			<CardPreview>
				{children()}
			</CardPreview>
		</Card>
	}	
}