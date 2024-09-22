use super::ComputeShader;
use crate::shaders::interface::RenderShader;
use crate::shaders::*;

/// Resources that only need to be loaded once for a given device.
#[derive(Debug)]
pub struct Resources {
	pub canvas: canvas::Shader,
	pub airbrush: airbrush::Shader,
	pub color_picker: color_picker::Shader,
	pub copy_transform: copy_transform::Shader,
	pub log_transform: ComputeShader,
	pub horizontal_scan: ComputeShader,
}

impl Resources {
	pub fn new(device: &wgpu::Device) -> Self {
		Resources {
			canvas: RenderShader::new(device),
			airbrush: RenderShader::new(device),
			color_picker: RenderShader::new(device),
			copy_transform: RenderShader::new(device),

			horizontal_scan: ComputeShader {
				module: horizontal_scan::create_shader_module(device),
				layout: horizontal_scan::create_pipeline_layout(device),
			},

			log_transform: ComputeShader {
				module: log_transform::create_shader_module(device),
				layout: log_transform::create_pipeline_layout(device),
			},
		}
	}
}
