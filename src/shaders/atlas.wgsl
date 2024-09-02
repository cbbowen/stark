// usage
@group(0) @binding(0)
var chart_sampler: sampler;

// block
struct ChartData {
	chart_to_canvas: mat4x4<f32>,
};
@group(1) @binding(0)
var chart_texture: texture_2d_array<f32>;  // [layer_index]
@group(1) @binding(1)
var<storage> chart_data: array<ChartData>;  // [layer_index]

// chart
struct InstanceInput {
	@location(0) layer_index: u32,
};

//

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) tex_coords: vec2<f32>,
	@location(1) @interpolate(flat) layer_index: u32,
};

@vertex
fn vs_main(
	@builtin(vertex_index) vertex_index: u32,
	instance_in: InstanceInput,
) -> VertexOutput {
	var out: VertexOutput;
	let x = f32(vertex_index & 1u);
	let y = 0.5 * f32(vertex_index & 2u);
	let layer_index = instance_in.layer_index;
	let chart_datum = chart_data[layer_index];
	out.clip_position = chart_datum.chart_to_canvas * vec4<f32>(x, y, 0.0, 1.0);
	out.tex_coords = vec2<f32>(x, y);
	// TODO: Get this from the instance buffer.
	out.layer_index = layer_index;
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	return textureSample(chart_texture, chart_sampler, in.tex_coords, in.layer_index);
}