use wgsl_to_wgpu_macro::shader_module;

shader_module!(pub mod "canvas.wgsl" in "src/shaders");
shader_module!(pub mod "drawing.wgsl" in "src/shaders");
shader_module!(pub mod "copy_scaled.wgsl" in "src/shaders");

pub mod chart {
	super::shader_module!(mod "chart.wgsl" in "src/shaders");
	pub use chart::bind_groups::BindGroup0 as BindGroup;
	pub use chart::bind_groups::BindGroupLayout0 as BindGroupLayout;
}
