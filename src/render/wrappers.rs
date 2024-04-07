use std::rc::Rc;

pub struct RcAsPtr {}

impl<T> bykey::KeyFunc<Rc<T>> for RcAsPtr {
	type Key<'a> = *const T where T: 'a;
	fn key(value: &Rc<T>) -> *const T {
		Rc::as_ptr(value)
	}
}

/// Wrapper for `wgpu` types to make them `Clone` and `Eq`.
pub type WgpuWrapper<T> = bykey::ByKey<Rc<T>, RcAsPtr>;

pub fn wgpu_wrapper<T>(value: T) -> WgpuWrapper<T> {
	Rc::new(value).into()
}

pub type WgpuInstance = WgpuWrapper<wgpu::Instance>;
pub type WgpuAdapter = WgpuWrapper<wgpu::Adapter>;
pub type WgpuDevice = WgpuWrapper<wgpu::Device>;
pub type WgpuQueue = WgpuWrapper<wgpu::Queue>;
pub type WgpuSurface = WgpuWrapper<wgpu::Surface<'static>>;
