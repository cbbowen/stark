@group(0) @binding(0)
var<uniform> chart_to_canvas: mat4x4<f32>;

// @group(1) @binding(0)
// var<uniform> canvas_to_view: mat4x4<f32>;

struct VertexInput {
	@builtin(vertex_index) vertex_index: u32,
};

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(
	in: VertexInput,
) -> VertexOutput {
	var out: VertexOutput;
	let x = f32(in.vertex_index & 1u);
	let y = 0.5 * f32(in.vertex_index & 2u);
	// out.clip_position = canvas_to_view * chart_to_canvas * vec4<f32>(x, y, 0.0, 1.0);
	out.clip_position = chart_to_canvas * vec4<f32>(x, y, 0.0, 1.0);
	out.tex_coords = vec2<f32>(x, y);
	return out;
}

@group(0) @binding(1)
var chart_texture: texture_2d<f32>;
@group(0) @binding(2)
var chart_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	return textureSample(chart_texture, chart_sampler, in.tex_coords);
}