use super::Shader;
use crate::shaders;
use std::sync::Arc;

/// Resources that only need to be loaded once for a given device.
#[derive(Debug)]
pub struct Resources {
	pub canvas: Shader,
	pub drawing: Shader,
	pub color_picker: Shader,
}

impl Resources {
	pub fn new(device: &wgpu::Device) -> Self {
		Resources {
			canvas: Shader {
				module: shaders::canvas::create_shader_module(device),
				layout: shaders::canvas::create_pipeline_layout(device),
			},

			drawing: Shader {
				module: shaders::drawing::create_shader_module(device),
				layout: shaders::drawing::create_pipeline_layout(device),
			},

			color_picker: Shader {
				module: shaders::color_picker::create_shader_module(device),
				layout: shaders::color_picker::create_pipeline_layout(device),
			},
		}
	}
}
