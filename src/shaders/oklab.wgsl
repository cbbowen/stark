const OKLAB_A = mat3x3(1.0, 0.3963377774, 0.2158037573,
							1.0, -0.1055613458, -0.0638541728,
							1.0, -0.0894841775, -1.2914855480);
const OKLAB_B = mat3x3(4.0767416621, -3.3077115913, 0.2309699292,
							-1.2684380046, 2.6097574011, -0.3413193965,
							-0.0041960863, -0.7034186147, 1.7076147010);

fn oklab_to_rgb(lab: vec3<f32>) -> vec3<f32> {
	return linear_srgb_to_rgb(oklab_to_linear_srgb(lab));
}

fn oklab_to_linear_srgb(lab: vec3<f32>) -> vec3<f32> {
  	let v = lab * OKLAB_A;
  	return (v * v * v) * OKLAB_B;
}

fn linear_srgb_to_rgb(srgb: vec3<f32>) -> vec3<f32> {
	return vec3(srgb_gamma(srgb.x), srgb_gamma(srgb.y), srgb_gamma(srgb.z));
}

fn srgb_gamma(x: f32) -> f32 {
	if x >= 0.0031308 {
		return 1.055 * pow(x, 1 / 2.4) - 0.055;
	 }
	 return 12.92 * x;
}
