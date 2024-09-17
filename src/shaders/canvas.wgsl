include!("oklab.wgsl") {}

@group(0) @binding(0)
var chart_sampler: sampler;
@group(0) @binding(1)
var<uniform> canvas_to_view: mat4x4<f32>;

include!("tile_read.wgsl") {}

struct VertexInput {
	@builtin(vertex_index) index: u32,
}

struct VertexOutput {
	@location(0) chart_position: vec2<f32>,
	@location(1) @interpolate(flat) layer_index: u32,
	@builtin(position) view_position: vec4<f32>,
};

@vertex
fn vs_main(
	instance: InstanceInput,
	vertex: VertexInput,
) -> VertexOutput {
	let x = f32(vertex.index & 1u);
	let y = 0.5 * f32(vertex.index & 2u);
	let chart_position = vec2<f32>(x, y);

	let layer_tile_data = tile_data[instance.layer_index];
	let canvas_position = layer_tile_data.chart_to_canvas_scale * chart_position + layer_tile_data.chart_to_canvas_translation;
	let view_position = canvas_to_view * vec4(canvas_position, 0.0, 1.0);

	var out: VertexOutput;
	out.layer_index = instance.layer_index;
	out.chart_position = chart_position;
	out.view_position = view_position;
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let oklab = textureSample(tile_texture, chart_sampler, in.chart_position, in.layer_index);
	return vec4(oklab_to_rgb(oklab.xyz), oklab.w);
}
