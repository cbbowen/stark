// use leptos::*;
// use wgpu::*;
use std::rc::Rc;

struct RcAsPtr {}

impl<T> bykey::KeyFunc<Rc<T>> for RcAsPtr {
	type Key<'a> = *const T where T: 'a;
	fn key(value: &Rc<T>) -> *const T {
		Rc::as_ptr(value)
	}
}

// Wrap `wgpu` types to make them `Clone` and `Eq`.
pub type WgpuInstance = bykey::ByKey<Rc<wgpu::Instance>, RcAsPtr>;
pub type WgpuAdapter = bykey::ByKey<Rc<wgpu::Adapter>, RcAsPtr>;
pub type WgpuDevice = bykey::ByKey<Rc<wgpu::Device>, RcAsPtr>;
pub type WgpuQueue = bykey::ByKey<Rc<wgpu::Queue>, RcAsPtr>;

pub trait RenderableInputs {
	fn instance(&self) -> WgpuInstance;
	fn adapter(&self) -> WgpuAdapter;
	fn device(&self) -> WgpuDevice;
	fn queue(&self) -> WgpuQueue;
	fn surface_configuration(&self) -> wgpu::SurfaceConfiguration;
}

pub type BoundRenderable = Box<dyn Fn(wgpu::TextureView)>;
// TODO: We may need yet another level of indirection returning, `Box<dyn Fn() -> BoundRenderable` so that we can make the `RenderCanvas` react to changes in the renderable. I don't see a good way to do it otherwise because throttling the actual render means we won't necessarily call the `BoundRenderable` after every change.
pub type Renderable = Box<dyn Fn(&dyn RenderableInputs) -> BoundRenderable>;

#[cfg(test)]
mod test {
	#[test]
	fn test_minimal_update() {
		use leptos::prelude::*;
		let _ = leptos::create_runtime();
		let (source, set_source) = leptos::create_signal((false, false));
		let projection = leptos::create_memo(move |_| source().0);

		let updates = std::rc::Rc::new(std::cell::RefCell::new(0));
		let derived = {
			let updates = updates.clone();
			leptos::create_memo(move |_| {
				*updates.borrow_mut() += 1;
				projection.get()
			})
		};

		// Test initialization.
		assert_eq!(updates.borrow().clone(), 0);
		assert_eq!(derived.get_untracked(), false);
		assert_eq!(updates.borrow().clone(), 1);
		assert_eq!(derived.get_untracked(), false);
		assert_eq!(updates.borrow().clone(), 1);

		// Test updating part of the source that isn't projected.
		set_source((false, true));
		assert_eq!(updates.borrow().clone(), 1);
		assert_eq!(derived.get_untracked(), false);
		assert_eq!(updates.borrow().clone(), 1);

		// Test updating the part of the source that is projected.
		set_source((true, false));
		assert_eq!(updates.borrow().clone(), 1);
		assert_eq!(derived.get_untracked(), true);
		assert_eq!(updates.borrow().clone(), 2);
	}
}
