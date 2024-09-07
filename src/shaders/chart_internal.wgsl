include!("chart.wgsl") {}

@vertex
fn vs_main(
	@builtin(vertex_index) vertex_index: u32,
	@builtin(instance_index) instance_index: u32,
	_instance_input: InstanceInput,
) {}

@fragment
fn fs_main() {}
