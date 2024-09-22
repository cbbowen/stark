@group(0) @binding(0)
var<uniform> _unused: u32;

include!("tile_read.wgsl") {}

@vertex
fn vs_main(
	@builtin(vertex_index) vertex_index: u32,
	@builtin(instance_index) instance_index: u32,
	_instance_input: InstanceInput,
) -> @builtin(position) vec4<f32> {
	return vec4(0f);
}

@fragment
fn fs_main() {}
