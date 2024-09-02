use wgsl_to_wgpu_macro::shader;

// `cargo expand --lib shaders::chart`

shader!(pub mod "atlas.wgsl" in "src/shaders");
shader!(pub mod "canvas.wgsl" in "src/shaders");
shader!(pub mod "drawing.wgsl" in "src/shaders");
shader!(pub mod "copy_scaled.wgsl" in "src/shaders");

pub mod chart {
	super::shader!(mod "chart.wgsl" in "src/shaders");
	pub use chart::{ChartData, InstanceInput, bind_groups::*};
}
