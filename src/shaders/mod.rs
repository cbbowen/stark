extern crate wgsl_to_wgpu_macro;

pub mod canvas {
	wgsl_to_wgpu_macro::shader_module!("src/shaders", "canvas.wgsl");
}

pub mod drawing {
	wgsl_to_wgpu_macro::shader_module!("src/shaders", "drawing.wgsl");
}

pub mod copy_scaled {
	wgsl_to_wgpu_macro::shader_module!("src/shaders", "copy_scaled.wgsl");
}
