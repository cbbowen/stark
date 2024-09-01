@group(0) @binding(0)
var source_texture: texture_2d<f32>;
@group(0) @binding(1)
var source_sampler: sampler;

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(
	@builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
	var out: VertexOutput;
	let x = f32(vertex_index & 1u);
	let y = 0.5 * f32(vertex_index & 2u);
	out.clip_position = vec4<f32>(2.0 * x - 1.0, 2.0 * y - 1.0, 0.0, 1.0);
	out.tex_coords = vec2<f32>(x, y);
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	return textureSample(source_texture, source_sampler, in.tex_coords);
}