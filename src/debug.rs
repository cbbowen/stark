use itertools::Itertools;
use std::{future::Future, u8};
use thiserror::Error;

use crate::util::ImageExt;
use crate::WgpuContext;
use zune_image::codecs::ImageFormat;
use zune_image::image::Image;

#[derive(Copy, Clone, Debug, Error)]
#[error("`debug` feature not enabled")]
pub struct DebugNotEnabled;

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
		let image = Image::from_texture_data(&data, width, height, format)?;
		let png_data = image.write_to_vec(ImageFormat::PNG)?;
		Ok(encode_data_url(&png_data, Some("image/png")))
	}
}
