include!("oklab.wgsl") {}
include!("dither.wgsl") {}

@group(0) @binding(0)
var<uniform> lightness: f32;

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(
	@builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
	var out: VertexOutput;
	let x = f32(vertex_index & 1u) - 0.5;
	let y = 0.5 * f32(vertex_index & 2u) - 0.5;
	// let x = 0.5 * f32((vertex_index + 1) & 2u) - 0.5;
	// let y = 0.5 * f32(vertex_index & 2u) - 0.5;
	out.tex_coords = vec2<f32>(x, y);
	// TODO: Pass this in as a transformation matrix.
	out.clip_position = vec4<f32>(3.8 * out.tex_coords + vec2(-0.09, 0.24), 0.0, 1.0);
	// out.clip_position = vec4<f32>(2.0 * out.tex_coords, 0.0, 1.0);
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let srgb = constrained_oklab_to_linear_srgb(lightness, in.tex_coords);
	// let srgb = oklab_to_linear_srgb(vec3(lightness, in.tex_coords));
	// if !valid_color(srgb) {
	// 	return vec4(0.0);
	// }
	let rgb = linear_srgb_to_rgb(srgb) + dither3(in.clip_position.xy) / 128.0;
	return vec4(rgb, 1.0);
}

fn valid_color(rgb: vec3<f32>) -> bool {
	let components = (rgb >= vec3(0.0)) & (rgb < vec3(1.0));
	return components.x & components.y & components.z;
}

fn step(condition: bool, size: f32) -> f32 {
	if condition {
		return size;
	}
	return -size;
}

override proof: bool = true;

// Scales chroma to produce a valid sRGB color.
fn constrained_oklab_to_linear_srgb(L: f32, ab: vec2<f32>) -> vec3<f32> {
	var s = 0.5;
	var r = oklab_to_linear_srgb(vec3(L, s * ab));
	var step_size = 0.5;

	for (var i = 0; i < 8; i = i + 1) {
		step_size = step_size * 0.5;
		s = s + step(valid_color(r), step_size);
		r = oklab_to_linear_srgb(vec3(L, s * ab));
	}

	// Final step with the same step size. This allows us to reach 1.0.
	s = s + step(valid_color(r), step_size);
	var proof_factor = 0.0;
	if proof {
		proof_factor = pow(1.0 - s, 0.25);
	}
	return mix(oklab_to_linear_srgb(vec3(L, s * ab)), 0.25 * vec3(1.0 - L), proof_factor);
}
