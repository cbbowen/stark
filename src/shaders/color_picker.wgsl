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
	out.tex_coords = vec2<f32>(x, y);
	out.clip_position = vec4<f32>(3.8 * out.tex_coords + vec2(-0.09, 0.24), 0.0, 1.0);
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let srgb = constrained_oklab_to_linear_srgb(lightness, in.tex_coords);
	// let srgb = oklab_to_linear_srgb(vec3(lightness, in.tex_coords));
	// if !valid_color(srgb) {
	// 	return vec4(0.0);
	// }
	let rgb = linear_srgb_to_rgb(srgb);
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
	return oklab_to_linear_srgb(vec3(L, s * ab));
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

fn linear_srgb_to_rgb(srgb: vec3<f32>) -> vec3<f32> {
	return vec3(gamma(srgb.x), gamma(srgb.y), gamma(srgb.z));
}

fn gamma(x: f32) -> f32 {
	if x >= 0.0031308 {
		return 1.055 * pow(x, 1 / 2.4) - 0.055;
	 }
	 return 12.92 * x;
}