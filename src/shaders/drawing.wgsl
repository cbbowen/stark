struct DrawingAction {
   position: vec2<f32>,
	pressure: f32,
	seed: vec2<f32>,
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
	let size = sqrt(action.pressure);
	let pos = (2.0 * action.position - 1.0) + size * 0.5 * vec2<f32>(x, y);
	out.clip_position = vec4<f32>(pos.x, pos.y, 0.0, 1.0);
	out.tex_coords = vec2<f32>(x, y);
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let c = in.tex_coords + 0.02 * dither(in.clip_position.xy + action.seed);
	let softness = 0.5;
	let opacity = sqrt(action.pressure) * 0.05;
	let alpha = opacity * pow(max(0.0, 1.0 - dot(c, c)), softness);
	return vec4<f32>(0.75, vec2(0.03, 0.15) * sin(c * 1.57079632679), alpha);
}

fn dither(co: vec2<f32>) -> vec2<f32> {
	let a = sin(dot(co.xy, vec2(12.9898, 78.233)));
	let b = (co.xy + vec2(43758.5453, 29443.5016));
	return fract(a * b) - 0.5;
}
