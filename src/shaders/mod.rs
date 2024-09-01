extern crate wgsl_to_wgpu_macro;

wgsl_to_wgpu_macro::shader_module!("src/shaders", "canvas.wgsl");
wgsl_to_wgpu_macro::shader_module!("src/shaders", "drawing.wgsl");
wgsl_to_wgpu_macro::shader_module!("src/shaders", "copy_scaled.wgsl");
