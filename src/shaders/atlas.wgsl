// usage
@group(0) @binding(0)
var chart_sampler: sampler;

// block
include!("tile_read.wgsl") {}

struct VertexOutput {
	@builtin(position) position: vec4<f32>,
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
	let layer_tile_data = tile_data[layer_index];
	out.position = vec4(layer_tile_data.chart_to_canvas_scale * vec2(x, y) + layer_tile_data.chart_to_canvas_translation, 0.0, 1.0);
	out.tex_coords = vec2<f32>(x, y);
	out.layer_index = layer_index;
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	return textureSample(tile_texture, chart_sampler, in.tex_coords, in.layer_index);
}