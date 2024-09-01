@group(0) @binding(0)
var<uniform> chart_to_canvas: mat4x4<f32>;
@group(0) @binding(1)
var chart_texture: texture_2d<f32>;
@group(0) @binding(2)
var chart_sampler: sampler;

@group(1) @binding(0)
var<uniform> canvas_to_view: mat4x4<f32>;

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
	out.clip_position = vec4<f32>(x, y, 0.0, 1.0) * chart_to_canvas * canvas_to_view;
	out.tex_coords = vec2<f32>(x, y);
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let oklab = textureSample(chart_texture, chart_sampler, in.tex_coords);
	return vec4(oklab_to_linear_srgb(oklab.xyz), oklab.w);
}

fn oklab_to_linear_srgb(c: vec3<f32>) -> vec3<f32> {
   let A = mat3x3<f32>(1.0, 0.3963377774, 0.2158037573,
                       1.0, -0.1055613458, -0.0638541728,
                       1.0, -0.0894841775, -1.2914855480);
  	let B = mat3x3<f32>(4.0767416621, -3.3077115913, 0.2309699292,
                       -1.2684380046, 2.6097574011, -0.3413193965,
                       -0.0041960863, -0.7034186147, 1.7076147010);
  	let d = c * A;
  	return (d * d * d) * B;
}
