
use crate::shaders;
use super::Shader;
use super::pipeline_builder::*;
use std::rc::Rc;

/// Resources that only need to be loaded once for a given device.
#[derive(Debug, Clone)]
pub struct Resources {
	pub canvas: Rc<Shader>,
	pub drawing_action_pipeline_factory: Rc<PipelineFactory>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct DrawingActionUniform {
	position: [f32; 2],
}

impl UniformBindingType for DrawingActionUniform {
	fn name() -> &'static str {
		"DrawingActionUniform"
	}

	fn type_definitions() -> TypeDefinitions {
		[r#"
struct DrawingActionUniform {
	position: vec2<f32>,
};
		 "#
		.to_string()]
		.into_iter()
		.collect()
	}
}

impl Resources {
	pub fn new(device: &wgpu::Device) -> Self {
		Resources {
			canvas: Shader {
				module: shaders::canvas::create_shader_module(device),
				layout: shaders::canvas::create_pipeline_layout(device),
			}.into(),

			drawing_action_pipeline_factory: PipelineFactory::new(
				device,
				"drawing",
				include_str!("drawing.wgsl"),
				[BindGroupLayout::new(
					device,
					"texture",
					[BindGroupLayoutEntry::new(
						"action",
						wgpu::ShaderStages::VERTEX,
						&UniformBuildBindingType::<DrawingActionUniform>::new(),
					)],
				)],
			)
			.into(),
		}
	}
}
