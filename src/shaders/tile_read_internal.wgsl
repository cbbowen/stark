@group(0) @binding(0)
var<uniform> _unused: u32;

include!("tile_read.wgsl") {}

@vertex
fn vs_main(
	@builtin(vertex_index) vertex_index: u32,
	@builtin(instance_index) instance_index: u32,
	_instance_input: InstanceInput,
) {}

@fragment
fn fs_main() {}
