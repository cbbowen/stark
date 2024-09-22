
@group(0) @binding(0)
var<uniform> transform: mat2x2<f32>;
@group(0) @binding(1)
var source_texture: texture_2d<f32>;
@group(0) @binding(2)
var source_sampler: sampler;

struct VertexOutput {
	@builtin(position) destination_position: vec4<f32>,
	@location(0) source_position: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
	let x = f32(vertex_index & 1u);
	let y = 0.5 * f32(vertex_index & 2u);
	let source_position = vec2(x, y);

	let centered_position = vec2(2.0, -2.0) * (source_position - 0.5);
	let destination_position = transform * centered_position;

	var out: VertexOutput;
	out.destination_position = vec4(destination_position, 0.0, 1.0);
	out.source_position = source_position;
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	 return textureSample(source_texture, source_sampler, in.source_position);
}