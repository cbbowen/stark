// Long term, we probably don't want to embed the large assets.

use crate::util::*;
use zune_core::colorspace::ColorSpace;
use zune_image::image::*;

static RAW_00507_PNG: &[u8] = include_bytes!("../../public/assets/shapes/00507.png");

pub struct Shape {
	pub width: u32,
	pub height: u32,
	pub values: Vec<f32>,
}

pub fn get_shape_00507() -> Shape {
	let mut image = Image::read(RAW_00507_PNG, Default::default()).unwrap();
	image.convert_color(ColorSpace::Luma).unwrap();
	let (width, height) = image.dimensions();

	Shape {
		width: width as u32,
		height: height as u32,
		values: image.convert_to_f32_subpixels(),
	}
}
