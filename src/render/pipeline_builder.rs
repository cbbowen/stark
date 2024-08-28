use std::fmt::{format, Debug};
use std::num::NonZero;
use std::rc::Rc;

use wgpu::BufferBindingType;

/// A WGSL type definition to be added to the shader source file.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TypeDefinition(String);

fn unique_definitions(all_definitions: &[TypeDefinition]) -> Vec<TypeDefinition> {
	let mut definitions = Vec::new();
	let mut definition_set = std::collections::HashSet::new();
	for d in all_definitions {
		if definition_set.insert(d) {
			definitions.push(d.clone());
		}
	}
	definitions
}

pub trait BuildBindingType: Debug {
	fn type_definitions(&self) -> Vec<TypeDefinition> {
		Vec::new()
	}
	fn var_definition(&self, name: &str) -> String;
	fn binding_type(&self) -> wgpu::BindingType;
}

#[derive(Clone, Debug)]
pub struct Texture2f2BuildBindingType {
	multisampled: bool,
	filterable: bool,
}

impl Default for Texture2f2BuildBindingType {
	fn default() -> Self {
		Self {
			multisampled: false,
			filterable: true,
		}
	}
}

impl BuildBindingType for Texture2f2BuildBindingType {
	fn binding_type(&self) -> wgpu::BindingType {
		wgpu::BindingType::Texture {
			sample_type: wgpu::TextureSampleType::Float {
				filterable: self.filterable,
			},
			view_dimension: wgpu::TextureViewDimension::D2,
			multisampled: self.multisampled,
		}
	}

	fn var_definition(&self, name: &str) -> String {
		format!("var {name}: texture_2d<f32>")
	}
}

#[derive(Clone, Debug)]
pub struct SamplerBuildBindingType(wgpu::SamplerBindingType);

impl Default for SamplerBuildBindingType {
	fn default() -> Self {
		Self(wgpu::SamplerBindingType::Filtering)
	}
}

impl BuildBindingType for SamplerBuildBindingType {
	fn binding_type(&self) -> wgpu::BindingType {
		wgpu::BindingType::Sampler(self.0)
	}

	fn var_definition(&self, name: &str) -> String {
		format!("var {name}: sampler")
	}
}

/// Implemented by types which can be used in uniform buffers.
pub trait UniformBindingType: bytemuck::Pod + bytemuck::Zeroable {
	fn name() -> &'static str;
	fn type_definitions() -> Vec<TypeDefinition> {
		Vec::new()
	}
}

impl UniformBindingType for f32 {
	fn name() -> &'static str {
		"f32"
	}
}

pub struct UniformBuildBindingType<T: 'static> {
	_t: std::marker::PhantomData<&'static T>,
}

impl<T: 'static> UniformBuildBindingType<T> {
	pub fn new() -> Self {
		UniformBuildBindingType {
			_t: std::marker::PhantomData,
		}
	}
}

impl<T: UniformBindingType + 'static> Debug for UniformBuildBindingType<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let type_name = T::name();
		write!(f, "UniformBuildBindingType<{type_name}>")
	}
}

impl<T: UniformBindingType + 'static> BuildBindingType for UniformBuildBindingType<T> {
	fn type_definitions(&self) -> Vec<TypeDefinition> {
		T::type_definitions()
	}

	fn var_definition(&self, name: &str) -> String {
		format!("var<uniform> {name}: {}", T::name())
	}

	fn binding_type(&self) -> wgpu::BindingType {
		wgpu::BindingType::Buffer {
			ty: BufferBindingType::Uniform,
			has_dynamic_offset: false,
			min_binding_size: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct BindGroupLayoutEntryBuilder {
	name: String,
	visibility: wgpu::ShaderStages,
	ty: std::rc::Rc<dyn BuildBindingType>,
	count: Option<NonZero<u32>>,
}

impl BindGroupLayoutEntryBuilder {
	pub fn new(
		name: &str,
		visibility: wgpu::ShaderStages,
		ty: std::rc::Rc<dyn BuildBindingType>,
	) -> Self {
		Self {
			name: name.to_owned(),
			visibility,
			ty,
			count: None,
		}
	}

	pub fn with_count(mut self, count: u32) -> Self {
		self.count = std::num::NonZero::new(count);
		self
	}

	pub fn build(
		&self,
		group_index: u32,
		binding_index: u32,
	) -> (wgpu::BindGroupLayoutEntry, Vec<TypeDefinition>) {
		let entry = wgpu::BindGroupLayoutEntry {
			binding: binding_index,
			visibility: self.visibility,
			ty: self.ty.binding_type(),
			count: self.count,
		};
		let mut definitions = self.ty.type_definitions();
		definitions.push(TypeDefinition(format!(
			"@group({group_index}) @binding({binding_index})\n{};\n\n",
			self.ty.var_definition(&self.name)
		)));
		(entry, definitions)
	}
}

#[derive(Debug, Clone)]
pub struct BindGroupLayoutBuilder {
	label: Option<String>,
	entries: Vec<BindGroupLayoutEntryBuilder>,
}

impl Default for BindGroupLayoutBuilder {
	fn default() -> Self {
		Self {
			label: None,
			entries: Vec::new(),
		}
	}
}

impl BindGroupLayoutBuilder {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn with_label(mut self, name: String) -> Self {
		self.label = Some(name);
		self
	}

	pub fn add_entry(mut self, entry: BindGroupLayoutEntryBuilder) -> Self {
		self.entries.push(entry);
		self
	}

	fn build(
		self,
		group_index: u32,
		device: &wgpu::Device,
	) -> (BindGroupFactory, Vec<TypeDefinition>) {
		let mut definitions = Vec::new();
		let mut entries = Vec::with_capacity(self.entries.len());
		for (binding_index, entry_builder) in self.entries.iter().enumerate() {
			let (entry, entry_definitions) =
				entry_builder.build(group_index as u32, binding_index as u32);
			entries.push(entry);
			definitions.extend(entry_definitions);
		}
		let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: self.label.as_ref().map(String::as_str),
			entries: &entries,
		});
		(
			BindGroupFactory {
				label: self.label,
				layout: layout.into(),
			},
			definitions,
		)
	}
}

#[derive(Debug, Clone)]
pub struct PipelineFactoryBuilder<'a> {
	label: &'a str,
	source: &'a str,
	groups: Vec<BindGroupLayoutBuilder>,
}

impl<'a> PipelineFactoryBuilder<'a> {
	pub fn new(label: &'a str, source: &'a str) -> Self {
		Self {
			label,
			source,
			groups: Vec::new(),
		}
	}

	pub fn add_group(mut self, entry: BindGroupLayoutBuilder) -> Self {
		self.groups.push(entry);
		self
	}

	pub fn build(self, device: &wgpu::Device) -> PipelineFactory {
		// Build the binding group layouts/builders.
		let mut definitions = Vec::new();
		let mut bind_group_factories = Vec::new();
		for (group_index, group_builder) in self.groups.into_iter().enumerate() {
			let (factory, group_definitions) =
				group_builder.build(group_index as u32, device);
				bind_group_factories.push(factory);
			definitions.extend(group_definitions);
		}
		let bind_group_layouts: Vec<&wgpu::BindGroupLayout> = bind_group_factories
			.iter()
			.map(|b| b.layout.as_ref())
			.collect();

		// Build the source with additional definitions.
		let definitions = unique_definitions(&definitions);
		let mut source = String::new();
		for d in definitions {
			source.push_str(&d.0);
		}
		source.push_str(&self.source);

		// Create the shader module.
		let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: Some(self.label),
			source: wgpu::ShaderSource::Wgsl(source.into()),
		});

		// Create the render pipeline.
		let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some(self.label),
			bind_group_layouts: &bind_group_layouts,
			push_constant_ranges: &[],
		});

		PipelineFactory {
			shader_module,
			render_pipeline_layout,
			bind_group_factories,
		}
	}
}

pub struct PipelineFactory {
	shader_module: wgpu::ShaderModule,
	render_pipeline_layout: wgpu::PipelineLayout,
	bind_group_factories: Vec<BindGroupFactory>,
}

#[derive(Clone)]
pub struct BindGroupFactory {
	label: Option<String>,
	layout: Rc<wgpu::BindGroupLayout>,
}

impl BindGroupFactory {
	pub fn create(&self, device: &wgpu::Device, entries: &[wgpu::BindGroupEntry]) -> wgpu::BindGroup {
		device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: self.label.as_ref().map(String::as_str),
			layout: &self.layout,
			entries,
		})
	}
}

impl PipelineFactory {
	pub fn module(&self) -> &wgpu::ShaderModule {
		&self.shader_module
	}
	pub fn layout(&self) -> &wgpu::PipelineLayout {
		&self.render_pipeline_layout
	}
	pub fn bind_group_factories(&self) -> &[BindGroupFactory] {
		&self.bind_group_factories
	}
	// pub fn create_pipeline(
	// 	&self,
	//    device: &wgpu::Device,
	// 	targets: &[Option<wgpu::ColorTargetState>],
	// ) -> wgpu::RenderPipeline {
	// 	self
	// 		.device
	// 		.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
	// 			label: None,
	// 			layout: Some(&self.render_pipeline_layout),
	// 			vertex: wgpu::VertexState {
	// 				module: &self.shader_module,
	// 				entry_point: "vs_main",
	// 				compilation_options: Default::default(),
	// 				buffers: &[],
	// 			},
	// 			fragment: Some(wgpu::FragmentState {
	// 				module: &self.shader_module,
	// 				entry_point: "fs_main",
	// 				compilation_options: Default::default(),
	// 				targets,
	// 			}),
	// 			primitive: wgpu::PrimitiveState {
	// 				topology: wgpu::PrimitiveTopology::TriangleStrip,
	// 				strip_index_format: None,
	// 				front_face: wgpu::FrontFace::Ccw,
	// 				cull_mode: Some(wgpu::Face::Back),
	// 				polygon_mode: wgpu::PolygonMode::Fill,
	// 				unclipped_depth: false,
	// 				conservative: false,
	// 			},
	// 			depth_stencil: None,
	// 			multisample: wgpu::MultisampleState::default(),
	// 			multiview: None,
	// 			cache: None,
	// 		})
	// }
}
