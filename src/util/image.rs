use half::f16;
use itertools::Itertools;
use thiserror::Error;
use zune_core::bit_depth::BitDepth;
use zune_core::colorspace::ColorSpace;

#[derive(Debug, Error)]
pub enum Error {
	#[error("unsupported textue format {0:?}")]
	UnsupportedTextureFormat(wgpu::TextureFormat),
}
use Error::*;

pub type Result<T> = std::result::Result<T, Error>;

pub trait ImageExt: Sized {
	fn into_texture_data(self) -> (Box<[u8]>, u32, u32, wgpu::TextureFormat);
	fn from_texture_data(
		data: &[u8],
		width: u32,
		height: u32,
		texture_format: wgpu::TextureFormat,
	) -> Result<Self>;

	fn convert_to_u8_subpixels(&mut self) -> Vec<u8>;
	fn convert_to_u16_subpixels(&mut self) -> Vec<u16>;
	fn convert_to_f32_subpixels(&mut self) -> Vec<f32>;
}

impl ImageExt for zune_image::image::Image {
	fn convert_to_u8_subpixels(&mut self) -> Vec<u8> {
		self.convert_depth(BitDepth::Eight).unwrap();
		let channels = self.frames_ref()[0].channels_ref(self.colorspace(), false);
		let size: usize = channels.iter().map(|c| c.len()).sum();
		let len = size.div_ceil(std::mem::size_of::<u8>());
		let mut data = Vec::from_iter(std::iter::repeat_n(Default::default(), len));
		let len = zune_image::utils::swizzle_channels(channels, &mut data).unwrap();
		debug_assert_eq!(len, data.len());
		data.truncate(len);
		data
	}

	fn convert_to_u16_subpixels(&mut self) -> Vec<u16> {
		self.convert_depth(BitDepth::Sixteen).unwrap();
		let channels = self.frames_ref()[0].channels_ref(self.colorspace(), false);
		let size: usize = channels.iter().map(|c| c.len()).sum();
		let len = size.div_ceil(std::mem::size_of::<u16>());
		let mut data = Vec::from_iter(std::iter::repeat_n(Default::default(), len));
		let len = zune_image::utils::swizzle_channels(channels, &mut data).unwrap();
		debug_assert_eq!(len, data.len());
		data.truncate(len);
		data
	}

	fn convert_to_f32_subpixels(&mut self) -> Vec<f32> {
		self.convert_depth(BitDepth::Float32).unwrap();
		let channels = self.frames_ref()[0].channels_ref(self.colorspace(), false);
		let size: usize = channels.iter().map(|c| c.len()).sum();
		let len = size.div_ceil(std::mem::size_of::<f32>());
		let mut data = Vec::from_iter(std::iter::repeat_n(Default::default(), len));
		let len = zune_image::utils::swizzle_channels(channels, &mut data).unwrap();
		debug_assert_eq!(len, data.len());
		data.truncate(len);
		data
	}

	fn into_texture_data(mut self) -> (Box<[u8]>, u32, u32, wgpu::TextureFormat) {
		let (depth, color, texture_format) = {
			match (self.depth(), self.colorspace()) {
				(BitDepth::Eight, ColorSpace::Luma) => (
					BitDepth::Eight,
					ColorSpace::Luma,
					wgpu::TextureFormat::R8Unorm,
				),
				(BitDepth::Eight, _) => (
					BitDepth::Eight,
					ColorSpace::RGBA,
					wgpu::TextureFormat::Rgba8Unorm,
				),
				(BitDepth::Sixteen, ColorSpace::Luma) => (
					BitDepth::Sixteen,
					ColorSpace::Luma,
					wgpu::TextureFormat::R16Unorm,
				),
				(BitDepth::Sixteen, ColorSpace::RGBA) => (
					BitDepth::Sixteen,
					ColorSpace::RGBA,
					wgpu::TextureFormat::Rgba16Unorm,
				),
				_ => (
					BitDepth::Float32,
					ColorSpace::RGBA,
					wgpu::TextureFormat::Rgba32Float,
				),
			}
		};
		self.convert_color(color).unwrap();
		let data = match depth {
			BitDepth::Eight => {
				bytemuck::cast_slice_box(self.convert_to_u8_subpixels().into_boxed_slice())
			}
			BitDepth::Sixteen => {
				bytemuck::cast_slice_box(self.convert_to_u16_subpixels().into_boxed_slice())
			}
			BitDepth::Float32 => {
				bytemuck::cast_slice_box(self.convert_to_f32_subpixels().into_boxed_slice())
			}
			_ => unreachable!(),
		};
		let (width, height) = self.dimensions();
		(data, width as u32, height as u32, texture_format)
	}

	fn from_texture_data(
		data: &[u8],
		width: u32,
		height: u32,
		texture_format: wgpu::TextureFormat,
	) -> Result<Self> {
		let width = width as usize;
		let height = height as usize;
		Ok(match texture_format.remove_srgb_suffix() {
			wgpu::TextureFormat::R8Unorm => Self::from_u8(data, width, height, ColorSpace::Luma),
			wgpu::TextureFormat::Rgba8Unorm => Self::from_u8(data, width, height, ColorSpace::RGBA),
			wgpu::TextureFormat::R16Unorm => {
				Self::from_u16(bytemuck::cast_slice(data), width, height, ColorSpace::Luma)
			}
			wgpu::TextureFormat::Rgba16Unorm => {
				Self::from_u16(bytemuck::cast_slice(data), width, height, ColorSpace::RGBA)
			}
			wgpu::TextureFormat::R32Float => {
				Self::from_f32(bytemuck::cast_slice(data), width, height, ColorSpace::Luma)
			}
			wgpu::TextureFormat::Rgba32Float => {
				Self::from_f32(bytemuck::cast_slice(data), width, height, ColorSpace::RGBA)
			}
			wgpu::TextureFormat::R16Float => Self::from_f32(
				&bytemuck::cast_slice::<_, f16>(data)
					.into_iter()
					.copied()
					.map(f16::to_f32)
					.collect_vec(),
				width,
				height,
				ColorSpace::Luma,
			),
			wgpu::TextureFormat::Rgba16Float => Self::from_f32(
				&bytemuck::cast_slice::<_, f16>(data)
					.into_iter()
					.copied()
					.map(f16::to_f32)
					.collect_vec(),
				width,
				height,
				ColorSpace::RGBA,
			),
			_ => Err(UnsupportedTextureFormat(texture_format))?,
		})
	}
}
