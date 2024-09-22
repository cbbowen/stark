use wgsl_to_wgpu_macro::shader;

// `cargo expand --lib shaders::internal::airbrush`

shader!(pub mod "atlas.wgsl" in "src/shaders");
shader!(pub mod "canvas.wgsl" in "src/shaders");
shader!(pub mod "copy_transform.wgsl" in "src/shaders");
shader!(pub mod "color_picker.wgsl" in "src/shaders");

shader!(pub mod "airbrush.wgsl" in "src/shaders");

shader!(pub mod "log_transform.wgsl" in "src/shaders" where filterable: false);
shader!(pub mod "horizontal_scan.wgsl" in "src/shaders" where filterable: false);

shader!(pub mod "tile_read_internal.wgsl" in "src/shaders");
shader!(pub mod "tile_write_internal.wgsl" in "src/shaders");
