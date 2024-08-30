use crate::geom::Mat4x4fUniform;

use super::pipeline_builder::*;
use std::rc::Rc;

/// Resources that only need to be loaded once for a given device.
#[derive(Debug, Clone)]
pub struct Resources {
	// Pipeline factories.
	pub render_drawing_pipeline_factory: Rc<PipelineFactory>,

	// Shader modules.
	pub drawing_shader_module: Rc<wgpu::ShaderModule>,
}

impl Resources {
	pub fn new(device: &wgpu::Device) -> Self {
		let render_drawing_pipeline_factory = PipelineFactory::new(device, "canvas", include_str!("canvas.wgsl"), [
			BindGroupLayout::new(
				device,
				"texture",
				[
					BindGroupLayoutEntry::new(
						"chart_to_canvas",
						wgpu::ShaderStages::VERTEX,
						&UniformBuildBindingType::<Mat4x4fUniform>::new(),
					),
					BindGroupLayoutEntry::new(
						"chart_texture",
						wgpu::ShaderStages::FRAGMENT,
						&Texture2f2BuildBindingType::default(),
					),
					BindGroupLayoutEntry::new(
						"chart_sampler",
						wgpu::ShaderStages::FRAGMENT,
						&SamplerBuildBindingType::default(),
					),
				])
		]).into();
		Resources {
			render_drawing_pipeline_factory,

			// Shader modules.
			drawing_shader_module: Rc::new(
				device.create_shader_module(wgpu::include_wgsl!("drawing.wgsl")),
			),
		}
	}
}
