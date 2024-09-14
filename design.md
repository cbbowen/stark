# Features

* Beautiful Lab colors, both for picking and blending.
* Full GPU acceleration.
* Continuous brushes.
* (in-progress) Infinite canvas.
* (in-progress) Infinite undo history.
* (maybe) Collaborative editing.

# Dependencies

## WGPU
https://sotrh.github.io/learn-wgpu/beginner/tutorial2-surface
* `wgpu::Instance`
* `wgpu::Surface`
  * Constructed from `web_sys::HtmlCanvasElement`
  * Needs to be `configure`d to the `wgpu::Device`:
    * initially,
	 * after a resize,
	 * and when rendering fails with `wgpu::SurfaceError::Lost`.
* `wgpu::Adapter`
  * `async`hronously constructed from `wgpu::Instance` and (ideally) `wgpu::Surface` (via `request_adapter`)
* `wgpu::Device` and `wgpu::Queue`
  * `async`hronously constructed from `wgpu::Adapter` (via `request_device`)

## Leptos
https://github.com/leptos-rs/leptos
https://book.leptos.dev/web_sys.html
https://docs.rs/leptos/latest/leptos/ev/trait.EventDescriptor.html (`DOMContentLoaded`, `load`, `unload` look useful)
Async: https://book.leptos.dev/async/10_resources.html

Native desktop is lower priority, but note that it doesn't currently work out-of-the-box with Dioxus either.

```rs
let canvas_ref: NodeRef<html::Canvas> = create_node_ref();
canvas_ref.on_load(move |canvas: HtmlElement<html::Canvas>| {
	// ...
});
```


## Alternatives considered

### Dioxus
https://docs.rs/dioxus-hooks/latest/dioxus_hooks/index.html
* `RenderCanvas`
  * On creation (`onmounted`), we get a `web_sys::HtmlCanvasElement` that can be used to construct the surface.
  * On destruction (`dioxus::hooks::use_on_destroy`), we need to remove the surface constructed from the `web_sys::HtmlCanvasElement`.
  * On resize, we need to `configure` the surface to the `wgpu::Device`.
