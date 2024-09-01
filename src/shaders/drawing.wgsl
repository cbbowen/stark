struct DrawingAction {
   position: vec2<f32>,
};
@group(0) @binding(0)
var<uniform> action: DrawingAction;

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
	let x = 2.0 * f32(in.vertex_index & 1u) - 1.0;
	let y = f32(in.vertex_index & 2u) - 1.0;
	let pos = (2.0 * action.position - 1.0) + 0.5 * vec2<f32>(x, y);
	out.clip_position = vec4<f32>(pos.x, pos.y, 0.0, 1.0);
	out.tex_coords = vec2<f32>(x, y);
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let alpha = 0.05 * max(0.0, 1.0 - dot(in.tex_coords, in.tex_coords));
	return vec4<f32>(0.75, vec2(0.1, 0.35) * sin(in.tex_coords / 1.57079632679), alpha);
}
