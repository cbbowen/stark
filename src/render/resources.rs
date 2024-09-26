use std::sync::Arc;

use crate::shaders::*;

/// Resources that only need to be loaded once for a given device.
pub struct Resources {
	pub canvas: canvas::Shader,
	pub airbrush: airbrush::Shader,
	pub color_picker: color_picker::Shader,
	pub copy_transform: copy_transform::Shader,

	pub depth_to_layers: depth_to_layers::Shader,
	pub layers_to_depth: layers_to_depth::Shader,
	pub log_transform: log_transform::Shader,
	pub horizontal_scan: horizontal_scan::Shader,
}

impl Resources {
	pub fn new(device: &Arc<wgpu::Device>) -> Self {
		Resources {
			canvas: canvas::Shader::new(device.clone()),
			airbrush: airbrush::Shader::new(device.clone()),
			color_picker: color_picker::Shader::new(device.clone()),
			copy_transform: copy_transform::Shader::new(device.clone()),

			depth_to_layers: depth_to_layers::Shader::new(device.clone()),
			layers_to_depth: layers_to_depth::Shader::new(device.clone()),
			log_transform: log_transform::Shader::new(device.clone()),
			horizontal_scan: horizontal_scan::Shader::new(device.clone()),
		}
	}
}
