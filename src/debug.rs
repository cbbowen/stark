use std::{future::Future, io::Write};
use thiserror::Error;

use crate::WgpuContext;

#[derive(Copy, Clone, Debug, Error)]
#[error("`debug` feature not enabled")]
pub struct DebugNotEnabled;

pub fn encode_png(
	data: &[u8],
	width: u32,
	height: u32,
	format: wgpu::TextureFormat,
	out: impl Write,
) -> anyhow::Result<()> {
	#[cfg(feature = "debug")]
	{
		let mut encoder = png::Encoder::new(out, width, height);
		encoder.set_color(match format.components() {
			1 => png::ColorType::Grayscale,
			3 => png::ColorType::Rgb,
			4 => png::ColorType::Rgba,
			_ => png::ColorType::Rgba,
		});
		encoder.set_depth(match format {
			wgpu::TextureFormat::Rgba16Float => png::BitDepth::Sixteen,
			_ => png::BitDepth::Eight,
		});
		encoder.set_srgb(png::SrgbRenderingIntent::AbsoluteColorimetric);
		encoder.set_compression(png::Compression::Best);
		encoder.write_header()?.write_image_data(&data)?;
		return Ok(());
	}
	#[allow(unreachable_code)]
	{
		Err(DebugNotEnabled)?;
		Ok(())
	}
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
		let mut png_data = Vec::new();
		encode_png(&data, width, height, format, &mut png_data)?;
		Ok(encode_data_url(&png_data, Some("image/png")))
	}
}
