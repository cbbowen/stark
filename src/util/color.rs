use glam::{Mat3, vec3};

pub trait ColorSpace: Sized + Copy + Default {
	fn from_linear_srgb(srgb: LinearSrgb) -> Self;

	fn to_linear_srgb(&self) -> LinearSrgb;

	fn from_color<C: ColorSpace>(color: C) -> Self {
		Self::from_linear_srgb(color.into_color())
	}

	fn into_color<C: ColorSpace>(self) -> C {
		C::from_color(self)
	}

	fn to_css(&self) -> String {
		let rgb: Rgb = self.into_color();
		rgb.to_css()
	}
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LinearSrgb { pub r: f32, pub g: f32, pub b: f32 }

impl ColorSpace for LinearSrgb {
	fn from_linear_srgb(srgb: LinearSrgb) -> Self {
		srgb
	}

	fn to_linear_srgb(&self) -> LinearSrgb {
		*self
	}

	fn from_color<C: ColorSpace>(color: C) -> Self {
		color.to_linear_srgb()
	}
}

fn srgb_inverse_transfer(x: f32) -> f32 {
	// This condition differs slightly from IEC2003, but I trust Björn Ottosson more.
	if x >= 0.0031308 {
		1.055 * x.powf(1.0 / 2.4) - 0.055
	} else {
		12.92 * x
	}
}

fn srgb_transfer(x: f32) -> f32 {
	// This condition differs slightly from IEC2003, but I trust Björn Ottosson more.
	if x >= 0.04045 {
		((x + 0.055) / 1.055).powf(2.4)
	} else {
		x / 12.92
	}
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Rgb { pub r: f32, pub g: f32, pub b: f32 }

impl ColorSpace for Rgb {
	fn from_linear_srgb(srgb: LinearSrgb) -> Self {
		Self {
			r: srgb_inverse_transfer(srgb.r),
			g: srgb_inverse_transfer(srgb.g),
			b: srgb_inverse_transfer(srgb.b),
		}
	}

	fn to_linear_srgb(&self) -> LinearSrgb {
		LinearSrgb {
			r: srgb_transfer(self.r),
			g: srgb_transfer(self.g),
			b: srgb_transfer(self.b),
		}
	}

	fn to_css(&self) -> String {
		format!(
			"rgb({} {} {})",
			(self.r.clamp(0.0, 1.0) * 255.5) as u8,
			(self.g.clamp(0.0, 1.0) * 255.5) as u8,
			(self.b.clamp(0.0, 1.0) * 255.5) as u8,
		)
	}
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Oklab { pub l: f32, pub a: f32, pub b: f32 }

impl ColorSpace for Oklab {
	fn from_linear_srgb(srgb: LinearSrgb) -> Self {
		#[cfg_attr(rustfmt, rustfmt_skip)]
		static A: Mat3 = Mat3::from_cols_array(&[
			0.4122214708, 0.2119034982, 0.0883024619,
			0.5363325363, 0.6806995451, 0.2817188376,
			0.0514459929, 0.1073969566, 0.6299787005]);
		#[cfg_attr(rustfmt, rustfmt_skip)]
		static B: Mat3 = Mat3::from_cols_array(&[
			 0.2104542553,  1.9779984951,  0.0259040371, 
			 0.7936177850, -2.4285922050,  0.7827717662, 
			-0.0040720468,  0.4505937099, -0.8086757660]);
		let lab = vec3(srgb.r, srgb.g, srgb.b);
		let v = A * lab;
		let lab = B * v.map(|x| x.cbrt());
		Self { 
			l: lab.x,
			a: lab.y,
			b: lab.z,
		}
	}

	fn to_linear_srgb(&self) -> LinearSrgb {
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
		let lab = vec3(self.l, self.a, self.b);
		let v = A * lab;
		let srgb = B * (v * v * v);
		LinearSrgb { 
			r: srgb.x,
			g: srgb.y,
			b: srgb.z,
		}
	}

	fn to_css(&self) -> String {
		format!(
			"oklab({} {} {})",
			self.l.clamp(0.0, 1.0),
			self.a.clamp(-0.4, 0.4),
			self.b.clamp(-0.4, 0.4),
		)
	}
}