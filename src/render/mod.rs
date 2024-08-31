mod resources;
pub use resources::*;

mod pipeline_builder;
pub use pipeline_builder::*;

#[derive(Debug)]
pub struct Shader {
	pub module: wgpu::ShaderModule,
	pub layout: wgpu::PipelineLayout,
}