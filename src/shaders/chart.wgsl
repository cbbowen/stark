// usage
@group(0) @binding(0)
var chart_sampler: sampler;
@group(0) @binding(1)
var<uniform> canvas_to_view: mat4x4<f32>;

// block
struct ChartData {
	chart_to_canvas: mat4x4<f32>,
};
@group(1) @binding(0)
var chart_texture: texture_2d_array<f32>;  // [layer_index]
@group(1) @binding(1)
// `storage` only because it is runtime-sized.
// see https://google.github.io/tour-of-wgsl/types/arrays/runtime-sized-arrays/
var<storage> chart_data: array<ChartData>;  // [layer_index]

// chart
struct InstanceInput {
	@location(0) layer_index: u32,
};

// There's no actual logic in this file. It's purpose is to specify the layout of the shared binding group.
// TODO: Ideally, we would have a way to automatically insert this in the WGSL files that need it. A purely textual solution isn't great, though because these will change very frequently and so should generally be the last binding group, not the first.

@vertex
fn vs_main(
	@builtin(vertex_index) vertex_index: u32,
	@builtin(instance_index) instance_index: u32,
	_instance_input: InstanceInput,
) {}

@fragment
fn fs_main() {}