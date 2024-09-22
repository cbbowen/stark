@group(0) @binding(0)
var<uniform> _unused: u32;

include!("tile_write.wgsl") {}

@vertex
fn vs_main() -> @builtin(position) vec4<f32> {
	return vec4(0f);
}

@fragment
fn fs_main() {}
