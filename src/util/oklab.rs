use glam::*;

pub fn oklab_to_rgb(lab: Vec3) -> Vec3 {
	return linear_srgb_to_rgb(oklab_to_linear_srgb(lab));
}

fn oklab_to_linear_srgb(lab: Vec3) -> Vec3 {
	#[cfg_attr(rustfmt, rustfmt_skip)]
	static A: Mat3 = Mat3::from_cols_array(&[
		1.0, 1.0, 1.0,
		0.3963377774, -0.1055613458, -0.0894841775,
		0.2158037573, -0.0638541728, -1.2914855480]);
	#[cfg_attr(rustfmt, rustfmt_skip)]
  	static B: Mat3 = Mat3::from_cols_array(&[
		4.0767416621, -1.2684380046, -0.0041960863, 
		-3.3077115913, 2.6097574011, -0.7034186147, 
		0.2309699292, -0.3413193965, 1.7076147010]);
	let v = A * lab;
	return B * (v * v * v);
}

fn linear_srgb_to_rgb(srgb: Vec3) -> Vec3 {
	return vec3(srgb_gamma(srgb.x), srgb_gamma(srgb.y), srgb_gamma(srgb.z));
}

fn srgb_gamma(x: f32) -> f32 {
	if x >= 0.0031308 {
		return 1.055 * x.powf(1.0 / 2.4) - 0.055;
	}
	return 12.92 * x;
}
