use super::Shader;
use crate::shaders;

/// Resources that only need to be loaded once for a given device.
#[derive(Debug)]
pub struct Resources {
	pub canvas: Shader,
	pub airbrush: Shader,
	pub color_picker: Shader,
	pub copy_transform: Shader,
	pub log_transform: Shader,
	pub horizontal_scan: Shader,
}

impl Resources {
	pub fn new(device: &wgpu::Device) -> Self {
		Resources {
			canvas: Shader {
				module: shaders::canvas::create_shader_module(device),
				layout: shaders::canvas::create_pipeline_layout(device),
			},

			airbrush: Shader {
				module: shaders::airbrush::create_shader_module(device),
				layout: shaders::airbrush::create_pipeline_layout(device),
			},

			color_picker: Shader {
				module: shaders::color_picker::create_shader_module(device),
				layout: shaders::color_picker::create_pipeline_layout(device),
			},

			copy_transform: Shader {
				module: shaders::copy_transform::create_shader_module(device),
				layout: shaders::copy_transform::create_pipeline_layout(device),
			},

			horizontal_scan: Shader {
				module: shaders::horizontal_scan::create_shader_module(device),
				layout: shaders::horizontal_scan::create_pipeline_layout(device),
			},

			log_transform: Shader {
				module: shaders::log_transform::create_shader_module(device),
				layout: shaders::log_transform::create_pipeline_layout(device),
			},
		}
	}
}
