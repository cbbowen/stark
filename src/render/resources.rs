use crate::geom::Mat4x4fUniform;

use super::pipeline_builder::*;
use std::rc::Rc;

/// Resources that only need to be loaded once for a given device.
#[derive(Debug, Clone)]
pub struct Resources {
	// Pipeline factories.
	pub render_drawing_pipeline_factory: Rc<PipelineFactory>,
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
			render_drawing_pipeline_factory: PipelineFactory::new(
				device,
				"canvas",
				include_str!("canvas.wgsl"),
				[BindGroupLayout::new(
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
					],
				)],
			).into(),

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
