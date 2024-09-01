
use crate::shaders;
use super::Shader;
use super::pipeline_builder::*;
use std::rc::Rc;

/// Resources that only need to be loaded once for a given device.
#[derive(Debug, Clone)]
pub struct Resources {
	pub canvas: Rc<Shader>,
	pub drawing: Rc<Shader>,
}

impl Resources {
	pub fn new(device: &wgpu::Device) -> Self {
		Resources {
			canvas: Shader {
				module: shaders::canvas::create_shader_module(device),
				layout: shaders::canvas::create_pipeline_layout(device),
			}.into(),

			drawing: Shader {
				module: shaders::drawing::create_shader_module(device),
				layout: shaders::drawing::create_pipeline_layout(device),
			}.into(),
		}
	}
}
