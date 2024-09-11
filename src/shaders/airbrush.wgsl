include!("dither.wgsl") {}

struct AirbrushAction {
	seed: vec2<f32>,
	color: vec3<f32>,
	pressure: f32,
};
@group(0) @binding(0)
var<uniform> action: AirbrushAction;

struct VertexInput {
	@builtin(vertex_index) vertex_index: u32,
	@location(0) position: vec2<f32>,
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
    let y = 1.0 - f32(in.vertex_index & 2u);
    out.tex_coords = vec2(x, y);
    out.clip_position = vec4(vec2(2.0, -2.0) * (in.position - 0.5), 0.0, 1.0);
   //  out.clip_position = vec4(in.position, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let c = in.tex_coords;
    let softness = 2.0;
    let opacity_noise = dither1(in.clip_position.xy + action.seed) / 8.0;
    let opacity = max(0.0, (sqrt(action.pressure) + opacity_noise) * 0.05);
    let alpha = opacity * pow(max(0.0, 1.0 - dot(c, c)), softness);

    let color = action.color + dither3(in.clip_position.xy + action.seed) / 128;
    return vec4(color, alpha);
}
