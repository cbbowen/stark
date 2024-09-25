use wgsl_to_wgpu_macro::shader;

// `cargo expand --lib shaders::horizontal_scan`

pub mod atlas {
	super::shader!(mod "atlas.wgsl" in "src/shaders");
	pub use atlas::Shader;
	pub use atlas::BindGroup0;
	pub use atlas::FragmentEntry;
	pub use atlas::OverrideConstants;
}

shader!(pub mod "canvas.wgsl" in "src/shaders");
shader!(pub mod "copy_transform.wgsl" in "src/shaders");
shader!(pub mod "color_picker.wgsl" in "src/shaders");

shader!(pub mod "airbrush.wgsl" in "src/shaders");

shader!(pub mod "log_transform.wgsl" in "src/shaders" where filterable: false);
shader!(pub mod "horizontal_scan.wgsl" in "src/shaders" where filterable: false);

// Expose parts of the tile read/write templates.
pub use tile_read::TileData;
pub mod tile_read {
	super::shader!(mod "tile_read_internal.wgsl" in "src/shaders");
	pub type BindGroupLayout = tile_read_internal::BindGroupLayout1;
	pub type BindGroup = tile_read_internal::BindGroup1;
	pub use tile_read_internal::InstanceInput;
	pub use tile_read_internal::TileData;
}
pub mod tile_write {
	super::shader!(mod "tile_write_internal.wgsl" in "src/shaders");
	pub type BindGroupLayout = tile_write_internal::BindGroupLayout1;
	pub type BindGroup = tile_write_internal::BindGroup1;
}
