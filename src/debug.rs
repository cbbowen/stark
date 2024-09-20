use itertools::Itertools;
use std::{future::Future, io::Write, u16, u8};
use thiserror::Error;

use crate::WgpuContext;

#[derive(Copy, Clone, Debug, Error)]
#[error("`debug` feature not enabled")]
pub struct DebugNotEnabled;

fn image_from_u8_subpixels(
	subpixels: Vec<u8>,
	width: u32,
	height: u32,
	components: u8,
) -> Option<image::DynamicImage> {
	use image::DynamicImage::*;
	use image::*;
	Some(match components {
		1 => ImageLuma8(ImageBuffer::from_vec(width, height, subpixels)?),
		2 => ImageLumaA8(ImageBuffer::from_vec(width, height, subpixels)?),
		3 => ImageRgb8(ImageBuffer::from_vec(width, height, subpixels)?),
		4 => ImageRgba8(ImageBuffer::from_vec(width, height, subpixels)?),
		_ => None?,
	})
}

fn image_from_u16_subpixels(
	subpixels: Vec<u16>,
	width: u32,
	height: u32,
	components: u8,
) -> Option<image::DynamicImage> {
	use image::DynamicImage::*;
	use image::*;
	Some(match components {
		1 => ImageLuma16(ImageBuffer::from_vec(width, height, subpixels)?),
		2 => ImageLumaA16(ImageBuffer::from_vec(width, height, subpixels)?),
		3 => ImageRgb16(ImageBuffer::from_vec(width, height, subpixels)?),
		4 => ImageRgba16(ImageBuffer::from_vec(width, height, subpixels)?),
		_ => None?,
	})
}

fn image_from_f32_subpixels(
	subpixels: Vec<f32>,
	width: u32,
	height: u32,
	components: u8,
) -> Option<image::DynamicImage> {
	use image::DynamicImage::*;
	use image::*;
	Some(match components {
		1 => None?,
		2 => None?,
		3 => ImageRgb32F(ImageBuffer::from_vec(width, height, subpixels)?),
		4 => ImageRgba32F(ImageBuffer::from_vec(width, height, subpixels)?),
		_ => None?,
	})
}

// TODO: Move this into test.rs?
#[cfg(feature = "debug")]
pub fn png_color_components(components: u8) -> Option<png::ColorType> {
	use png::ColorType::*;
	Some(match components {
		1 => Grayscale,
		2 => GrayscaleAlpha,
		3 => Rgb,
		4 => Rgba,
		_ => None?
	})
}

#[derive(Copy, Clone, Debug)]
pub enum SubpixelFormat {
	U8,
	U16,
	// U32,
	// F16,
	F32,
}

#[derive(Debug, Error)]
pub enum SubpixelFormatError {
	#[error("unknown subpixel format for texture format {0:?}")]
	Unknown(wgpu::TextureFormat),
}

impl SubpixelFormat {
	pub fn of_texture(format: wgpu::TextureFormat) -> Result<Self, SubpixelFormatError> {
		use wgpu::TextureFormat::*;
		use SubpixelFormat::*;
		use SubpixelFormatError::*;
		let bytes_per_component =
			format.target_pixel_byte_cost().unwrap_or(0) / format.components() as u32;
		Ok(match format {
			// R16Float | Rg16Float | Rgba16Float => F16,
			R32Float | Rg32Float | Rgba32Float => F32,
			_ => match bytes_per_component {
				1 => U8,
				2 => U16,
				// 4 => U32,
				_ => Err(Unknown(format))?,
			},
		})
	}

	pub fn preferred_image_format(self, components: u8) -> Self {
		use SubpixelFormat::*;
		match (self, components) {
			(F32, 1) | (F32, 2) => U16,
			// PNG doesn't support these:
			(F32, _) => U16,
			_ => self
		}
	}

	// TODO: Move this into test.rs?
	#[cfg(feature = "debug")]
	pub fn png_bit_depth(self) -> Option<png::BitDepth> {
		use SubpixelFormat::*;
		use png::BitDepth::*;
		Some(match self {
			U8 => Eight,
			U16 => Sixteen,
			_ => None?
		})
	}
}

#[derive(Debug, Error)]
pub enum TextureToImageError {
	#[error("{0}")]
	Format(#[from] SubpixelFormatError),

	#[error("no conversion from {from:?} to {to:?}")]
	Conversion { from: SubpixelFormat, to: SubpixelFormat },

	#[error("unable to create image from {width}x{height} texture with format {format:?} from [u8; {len}]")]
	Size {
		len: usize,
		width: u32,
		height: u32,
		format: wgpu::TextureFormat,
	},
}

pub fn convert_subpixels(input_format: SubpixelFormat, output_format: SubpixelFormat) {

}

pub fn image_from_raw(
	data: &[u8],
	width: u32,
	height: u32,
	format: wgpu::TextureFormat,
	image_subpixel_format: Option<SubpixelFormat>,
) -> Result<image::DynamicImage, TextureToImageError> {
	use TextureToImageError::*;
	use SubpixelFormat::*;

	let texture_subpixel_format = SubpixelFormat::of_texture(format)?;
	let components = format.components();
	let image_subpixel_format = image_subpixel_format.unwrap_or(texture_subpixel_format.preferred_image_format(components));

	let result = match texture_subpixel_format {
		U8 => {
			let data = data.into_iter().copied();
			match image_subpixel_format {
				U8 => image_from_u8_subpixels(data.collect(), width, height, components),
				U16 => image_from_u16_subpixels(data.map(|v| v as u16 * 257).collect(), width, height, components),
				F32 => image_from_f32_subpixels(data.map(|v| v as f32 / u8::MAX as f32).collect(), width, height, components),
			}
		},
		U16 => {
			let data = data.into_iter().copied().map(|v| v as u16);
			match image_subpixel_format {
				U8 => image_from_u8_subpixels(data.map(|v| (v / 257) as u8).collect(), width, height, components),
				U16 => image_from_u16_subpixels(data.collect(), width, height, components),
				F32 => image_from_f32_subpixels(data.map(|v| v as f32 / u16::MAX as f32).collect(), width, height, components),
			}
		},
		F32 => {
			let data = data.into_iter().copied().map(|v| v as f32);
			match image_subpixel_format {
				U8 => image_from_u8_subpixels(data.map(|v| (v * u8::MAX as f32) as u8).collect(), width, height, components),
				U16 => image_from_u16_subpixels(data.map(|v| (v * u16::MAX as f32) as u16).collect(), width, height, components),
				F32 => image_from_f32_subpixels(data.collect(), width, height, components),
			}
		},
	};
	result.ok_or(Size {
		len: data.len(),
		width,
		height,
		format,
	})
}

pub fn encode_png(
	data: &[u8],
	width: u32,
	height: u32,
	format: wgpu::TextureFormat,
	image_subpixel_format: Option<SubpixelFormat>,
	mut out: impl Write + std::io::Seek,
) -> anyhow::Result<()> {
	Ok(image_from_raw(data, width, height, format, image_subpixel_format)?.write_to(&mut out, image::ImageFormat::Png)?)
}

pub fn encode_data_url(data: &[u8], mediatype: Option<&str>) -> String {
	#[cfg(feature = "debug")]
	{
		use base64::engine::*;
		let mediatype = mediatype.unwrap_or("");
		let data = general_purpose::URL_SAFE.encode(data);
		return format!("data:{mediatype};base64,{data}");
	}
	#[allow(unreachable_code)]
	"data:text/plain,debug-disabled".to_owned()
}

pub fn encode_texture_layer_as_url(
	context: &WgpuContext,
	texture: &wgpu::Texture,
	layer_index: u32,
) -> impl Future<Output = anyhow::Result<String>> {
	let data = context.get_texture_layer_data(texture, layer_index);
	let width = texture.width();
	let height = texture.height();
	let format = texture.format();
	async move {
		let data = data.await?;
		let mut png_data = std::io::Cursor::new(Vec::new());
		encode_png(&data, width, height, format, None, &mut png_data)?;
		let png_data = png_data.into_inner();
		Ok(encode_data_url(&png_data, Some("image/png")))
	}
}
