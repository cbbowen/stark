mod resources;
pub use resources::*;

#[derive(Debug)]
pub struct Shader {
	pub module: wgpu::ShaderModule,
	pub layout: wgpu::PipelineLayout,
}