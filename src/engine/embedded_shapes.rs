// Long term, we probably don't want to embed the large assets.

use image::*;

static RAW_00507_PNG: &[u8] = include_bytes!("../../public/assets/shapes/00507.png");

pub struct Shape {
	pub width: u32,
	pub height: u32,
	pub values: Vec<f32>,
}

pub fn get_shape_00507() -> Shape {
	let reader = ImageReader::new(std::io::Cursor::new(RAW_00507_PNG)).with_guessed_format().unwrap();
	let image = reader.decode().unwrap();
	let values = image.pixels().map(|p| p.2.to_luma().0[0] as f32 / 255f32).collect();

	Shape{
		width: image.width(),
		height: image.height(),
		values,
	}
}