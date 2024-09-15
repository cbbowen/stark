include!("dither.wgsl") {}

struct AirbrushAction {
	seed: vec2<f32>,
	color: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> action: AirbrushAction;

@group(0) @binding(1)
var shape_texture: texture_3d<f32>;
@group(0) @binding(2)
var shape_sampler: sampler;

struct VertexInput {
	@builtin(vertex_index) vertex_index: u32,
	@location(0) position: vec2<f32>,
	@location(1) u_bounds: vec2<f32>,
	@location(2) opacity: f32,
	@location(3) rate: f32,
};

struct VertexOutput {
	@builtin(position) position: vec4<f32>,
	@location(0) u_bounds: vec2<f32>,
	@location(1) vw: vec2<f32>,
	@location(2) rate: f32,
};

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4(vec2(2.0, -2.0) * (in.position - 0.5), 0.0, 1.0);
    out.u_bounds = in.u_bounds;
    out.vw = vec2(f32(in.vertex_index & 1), in.opacity);
	 out.rate = in.rate;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	 // Useful for debugging toplogy.
	 //  let theta = 6.28 * 0.5 * (in.u_bounds.y + in.u_bounds.x);
	 //  return vec4(0.75, in.vw.y * 0.5 * vec2(sin(theta), cos(theta)), in.u_bounds.y - in.u_bounds.x);

    let shape_transmission = in.rate * (textureSample(shape_texture, shape_sampler, vec3(in.u_bounds.y, in.vw)).x -
	                                     textureSample(shape_texture, shape_sampler, vec3(in.u_bounds.x, in.vw)).x);

    let alpha = -expm1(shape_transmission) * (1 + 0.125 * dither1(in.position.xy + action.seed));

    let color = action.color + dither3(in.position.xy + action.seed) / 128;
    return vec4(color, clamp(alpha, 0.0, 1.0));
}

fn expm1(x: f32) -> f32 {
    return exp(x) - 1;
	// return x * (1 + x / 2 * (1 + x / 3 * (1 + x / 4)));
}
