use wgsl_to_wgpu_macro::shader;

// `cargo expand --lib shaders::chart`

shader!(pub mod "atlas.wgsl" in "src/shaders");
shader!(pub mod "canvas.wgsl" in "src/shaders");
shader!(pub mod "copy_scaled.wgsl" in "src/shaders");
shader!(pub mod "color_picker.wgsl" in "src/shaders");

shader!(pub mod "airbrush.wgsl" in "src/shaders");

pub mod chart {
	super::shader!(mod "chart_internal.wgsl" in "src/shaders");
	pub use chart_internal::bind_groups::BindGroup1 as BindGroup;
	pub use chart_internal::bind_groups::BindGroupLayout1 as BindGroupLayout;
	pub use chart_internal::{InstanceInput, TileData as ChartData};
}
