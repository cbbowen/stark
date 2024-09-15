include!("dither.wgsl") {}

struct AirbrushAction {
	seed: vec2<f32>,
	color: vec3<f32>,
	opacity: f32,
	hardness: f32,
};
@group(0) @binding(0)
var<uniform> action: AirbrushAction;

@group(0) @binding(1)
var shape_texture: texture_2d<f32>;
@group(0) @binding(2)
var shape_sampler: sampler;

struct VertexInput {
	@builtin(vertex_index) vertex_index: u32,
	@location(0) position: vec2<f32>,
	@location(1) u_bounds: vec2<f32>,
};

struct VertexOutput {
	@builtin(position) position: vec4<f32>,
	@location(0) u_bounds: vec2<f32>,
	@location(1) v: f32,
};

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4(vec2(2.0, -2.0) * (in.position - 0.5), 0.0, 1.0);
    out.u_bounds = in.u_bounds;
    out.v = f32(in.vertex_index & 1);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	// // Useful for debugging toplogy.
	// let theta = 6.28 * 0.5 * (in.u_bounds.y + in.u_bounds.x);
	// return vec4(0.75, 0.15 * vec2(sin(theta), cos(theta)), in.u_bounds.y - in.u_bounds.x);

    let shape_transmission = action.hardness * (
		textureSample(shape_texture, shape_sampler, vec2(in.u_bounds.y, in.v)).x -
	   textureSample(shape_texture, shape_sampler, vec2(in.u_bounds.x, in.v)).x);
	
	 // TODO: This way of implementing opacity isn't correct for continuous splatting.
	 // We want changing opacity to be equivalent to pointwise scaling the brush shape texture
	 // by the same value, which this is not. I think in general, that's not even something
	 // we can compute exactly here.

    let alpha = action.opacity * (1.0 - exp(shape_transmission)) * (1.0 + 0.0 * dither1(in.position.xy + action.seed));

    let color = action.color + dither3(in.position.xy + action.seed) / 128;
    return vec4(color, clamp(alpha, 0.0, 1.0));
}
