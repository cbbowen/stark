use bon::bon;
use std::{fmt::Debug, num::NonZero};

pub trait RenderShaderInterface: Debug {
	const VERTEX_ENTRY: &str;
	const FRAGMENT_ENTRY: &str;
	const NUM_VERTEX_INPUTS: usize;
	const NUM_TARGETS: usize;
	type OverrideConstants: Default;

	// type VertexEntry: Debug;
	// fn vertex_entry(
	// 	step_modes: [wgpu::VertexStepMode; Self::NUM_VERTEX_INPUTS],
	// 	overrides: &Self::OverrideConstants,
	// ) -> Self::VertexEntry
	// ;
	// fn vertex_state<'a>(
	// 	module: &'a wgpu::ShaderModule,
	// 	entry: &'a Self::VertexEntry,
	// ) -> wgpu::VertexState<'a>;

	type FragmentEntry: Debug;
	fn fragment_entry(
		targets: [Option<wgpu::ColorTargetState>; Self::NUM_TARGETS],
		overrides: &Self::OverrideConstants,
	) -> Self::FragmentEntry;
	fn fragment_state<'a>(
		module: &'a wgpu::ShaderModule,
		entry: &'a Self::FragmentEntry,
	) -> wgpu::FragmentState<'a>;

	fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule;
	fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout;
}

#[derive(Debug)]
pub struct RenderShader<Interface: RenderShaderInterface + 'static> {
	module: wgpu::ShaderModule,
	layout: wgpu::PipelineLayout,
	_interface: std::marker::PhantomData<Interface>,
}

#[bon]
impl<Interface: RenderShaderInterface + 'static> RenderShader<Interface> {
	pub fn new(device: &wgpu::Device) -> Self {
		Self {
			module: Interface::create_shader_module(device),
			layout: Interface::create_pipeline_layout(device),
			_interface: Default::default(),
		}
	}

	pub fn module(&self) -> &wgpu::ShaderModule {
		&self.module
	}

	pub fn layout(&self) -> &wgpu::PipelineLayout {
		&self.layout
	}

	#[builder(finish_fn = create)]
	pub fn pipeline<'a>(
		&self,
		#[builder(finish_fn)] device: &wgpu::Device,
		label: Option<&'static str>,
		vertex_buffer_layouts: &[wgpu::VertexBufferLayout<'a>; Interface::NUM_VERTEX_INPUTS],
		targets: [Option<wgpu::ColorTargetState>; Interface::NUM_TARGETS],
		#[builder(default)] overrides: Interface::OverrideConstants,
		depth_stencil: Option<wgpu::DepthStencilState>,
		#[builder(default)] multisample: wgpu::MultisampleState,
		multiview: Option<NonZero<u32>>,
		cache: Option<&wgpu::PipelineCache>,
	) -> wgpu::RenderPipeline
	where
		[(); Interface::NUM_VERTEX_INPUTS]: Sized,
		[(); Interface::NUM_TARGETS]: Sized,
	{
		let module = self.module();
		let layout = self.layout();
		let fragment_entry = Interface::fragment_entry(targets, &overrides);
		let fragment = Interface::fragment_state(module, &fragment_entry);
		let vertex = wgpu::VertexState {
			module,
			entry_point: Interface::VERTEX_ENTRY,
			compilation_options: wgpu::PipelineCompilationOptions {
				constants: fragment.compilation_options.constants,
				..Default::default()
			},
			buffers: vertex_buffer_layouts,
		};

		let layout = Some(layout);
		let fragment = Some(fragment);
		device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label,
			layout,
			vertex,
			fragment,
			primitive: wgpu::PrimitiveState {
				topology: wgpu::PrimitiveTopology::TriangleStrip,
				strip_index_format: None,
				front_face: wgpu::FrontFace::Ccw,
				cull_mode: None,
				polygon_mode: wgpu::PolygonMode::Fill,
				unclipped_depth: false,
				conservative: false,
			},
			depth_stencil,
			multisample,
			multiview,
			cache,
		})
	}
}
