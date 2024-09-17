use leptos::prelude::*;

#[component]
pub fn BrushSetting(#[prop(into)] name: String, children: Children) -> impl IntoView {
	view! {
		<div class="BrushSetting">
			<span class="BrushSettingName">{name}</span>
			{children()}
		</div>
	}
}
