use wgsl_to_wgpu_macro::shader;

// `cargo expand --lib shaders::horizontal_scan`

shader!(pub mod "atlas.wgsl" in "src/shaders");
shader!(pub mod "canvas.wgsl" in "src/shaders");
shader!(pub mod "copy_transform.wgsl" in "src/shaders");
shader!(pub mod "color_picker.wgsl" in "src/shaders");

shader!(pub mod "airbrush.wgsl" in "src/shaders");

shader!(mod "tile_read_internal.wgsl" in "src/shaders");
shader!(mod "tile_write_internal.wgsl" in "src/shaders");

shader!(pub mod "log_transform.wgsl" in "src/shaders" where filterable: false);
shader!(pub mod "horizontal_scan.wgsl" in "src/shaders" where filterable: false);

pub use tile_read_internal::TileData;

pub mod tile_read {
	pub use super::tile_read_internal::bind_groups::BindGroup1 as BindGroup;
	pub use super::tile_read_internal::bind_groups::BindGroupLayout1 as BindGroupLayout;
	pub use super::tile_read_internal::InstanceInput;
}

pub mod tile_write {
	pub use super::tile_write_internal::bind_groups::BindGroup1 as BindGroup;
	pub use super::tile_write_internal::bind_groups::BindGroupLayout1 as BindGroupLayout;
}
