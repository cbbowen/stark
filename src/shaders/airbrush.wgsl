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

include!("tile_write.wgsl") {}

struct VertexInput {
	@builtin(vertex_index) vertex_index: u32,
	@location(0) position: vec2<f32>,
	@location(1) u_bounds: vec2<f32>,
	@location(2) opacity: f32,
	@location(3) rate: f32,
};

struct VertexOutput {
	@builtin(position) position: vec4<f32>,
	@location(0) @interpolate(linear) u_bounds: vec2<f32>,
	@location(1) @interpolate(linear) vw: vec2<f32>,
	@location(2) @interpolate(linear) rate: f32,
	
	// @location(3) @interpolate(flat) theta: f32,
};

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
	 let canvas_position = in.position;
	 let layer_tile_data = tile_data[layer_index];
	 let chart_position = (canvas_position - layer_tile_data.chart_to_canvas_translation) / layer_tile_data.chart_to_canvas_scale;
    out.position = vec4(vec2(2.0, -2.0) * (chart_position - 0.5), 0.0, 1.0);
    out.u_bounds = in.u_bounds;
    out.vw = vec2(f32(in.vertex_index & 1), in.opacity);
	 out.rate = in.rate;
	 // out.theta = f32(in.vertex_index);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	 // Useful for debugging toplogy.
	 // let theta = 6.28 * 0.5 * (in.u_bounds.y + in.u_bounds.x);
	 // let theta = in.theta;
	 // return vec4(0.75, in.vw.y * 0.5 * vec2(sin(theta), cos(theta)), in.u_bounds.y - in.u_bounds.x);

    let shape_transmission = in.rate * (textureSample(shape_texture, shape_sampler, vec3(in.u_bounds.y, in.vw)).x -
	                                     textureSample(shape_texture, shape_sampler, vec3(in.u_bounds.x, in.vw)).x);

    let alpha = -expm1(shape_transmission) * (1 + dither1(in.position.xy + action.seed) / 256.0);

    let color = action.color + dither3(in.position.xy + action.seed) / 256;
    return vec4(color, clamp(alpha, 0.0, 1.0));
}

fn expm1(x: f32) -> f32 {
    return exp(x) - 1;
	// return x * (1 + x / 2 * (1 + x / 3 * (1 + x / 4)));
}
