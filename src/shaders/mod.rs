use wgsl_to_wgpu_macro::shader;

shader!(pub mod "canvas.wgsl" in "src/shaders");
shader!(pub mod "drawing.wgsl" in "src/shaders");
shader!(pub mod "copy_scaled.wgsl" in "src/shaders");

pub mod chart {
	super::shader!(mod "chart.wgsl" in "src/shaders");
	pub use chart::bind_groups::BindGroup0 as BindGroup;
	pub use chart::bind_groups::BindGroupLayout0 as BindGroupLayout;
}
