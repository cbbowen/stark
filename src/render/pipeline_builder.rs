use std::fmt::{format, Debug};
use std::num::NonZero;
use std::rc::Rc;

use itertools::Itertools;
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

pub struct BindGroupLayoutEntry {
	name: String,
	visibility: wgpu::ShaderStages,
	binding_type: wgpu::BindingType,
	count: Option<NonZero<u32>>,

	type_definitions: Vec<TypeDefinition>,
	var_definition: String,
}

impl BindGroupLayoutEntry {
	pub fn new(name: &str, visibility: wgpu::ShaderStages, ty: &dyn BuildBindingType) -> Self {
		Self {
			name: name.to_string(),
			visibility,
			binding_type: ty.binding_type(),
			count: None,
			type_definitions: ty.type_definitions(),
			var_definition: ty.var_definition(name),
		}
	}
}

#[derive(Debug)]
pub struct BindGroupLayout {
	label: Option<String>,
	layout: wgpu::BindGroupLayout,
	type_definitions: Vec<TypeDefinition>,
	var_definitions: Vec<String>,
}

impl BindGroupLayout {
	pub fn new(device: &wgpu::Device, label: &str, entries: impl IntoIterator<Item=BindGroupLayoutEntry>) -> Self {
		let label = Some(label.to_string());
		let mut layout_entries = Vec::new();
		let mut type_definitions = Vec::new();
		let mut var_definitions = Vec::new();
		for (binding, entry) in entries.into_iter().enumerate() {
			let binding = binding as u32;
			layout_entries.push(
				wgpu::BindGroupLayoutEntry {
					binding,
					visibility: entry.visibility,
					ty: entry.binding_type,
					count: entry.count,
				}
			);
			type_definitions.extend(entry.type_definitions);
			var_definitions.push(format!("@binding({binding}) {}", entry.var_definition));
		}
		let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: label.as_deref(),
			entries: &layout_entries,
		});
		Self { label, layout, type_definitions, var_definitions }
	}

	pub fn create_bind_group(
		&self,
		device: &wgpu::Device,
		resources: &[wgpu::BindingResource],
	) -> wgpu::BindGroup {
		let entries: Vec<_> = resources
			.into_iter()
			.enumerate()
			.map(|(binding, resource)| wgpu::BindGroupEntry {
				binding: binding as u32,
				resource: resource.clone(),
			})
			.collect();
		let entries: &[wgpu::BindGroupEntry<'_>] = entries.as_slice();
		device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: self.label.as_deref(),
			layout: &self.layout,
			entries,
		})
	}
}

#[derive(Debug)]
pub struct PipelineFactory {
	label: Option<String>,
	shader_module: wgpu::ShaderModule,
	render_pipeline_layout: wgpu::PipelineLayout,
	bind_group_layouts: Vec<BindGroupLayout>,
}

impl PipelineFactory {
	fn new_impl(device: &wgpu::Device, label: &str, source: &str, bind_group_layouts: Vec<BindGroupLayout>) -> Self {
		let label = Some(label.to_string());
		// Build the source with additional definitions.
		let mut definitions = Vec::new();
		for (group, layout) in bind_group_layouts.iter().enumerate() {
			definitions.extend(layout.type_definitions.iter().cloned());
			for var_definition in layout.var_definitions.iter() {
				definitions.push(TypeDefinition(format!("@group({group}) {var_definition};\n\n")));
			}
		}
		let definitions = unique_definitions(&definitions);
		let mut full_source = String::new();
		for d in definitions {
			full_source.push_str(&d.0);
		}
		full_source.push_str(source);
		tracing::info!(?label, ?full_source, "full shader source");

		// Create the shader module.
		let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: label.as_deref(),
			source: wgpu::ShaderSource::Wgsl(full_source.into()),
		});

		// Create the render pipeline.
		let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: label.as_deref(),
			bind_group_layouts: &bind_group_layouts.iter().map(|g| &g.layout).collect_vec(),
			push_constant_ranges: &[],
		});

		PipelineFactory {
			label,
			shader_module,
			render_pipeline_layout,
			bind_group_layouts,
		}
	}

	pub fn new(device: &wgpu::Device, label: &str, source: &str, bind_group_layouts: impl IntoIterator<Item=BindGroupLayout>) -> Self {
		let bind_group_layouts: Vec<_> = bind_group_layouts.into_iter().collect();
		Self::new_impl(device, label, source, bind_group_layouts)
	}

	pub fn module(&self) -> &wgpu::ShaderModule {
		&self.shader_module
	}
	pub fn layout(&self) -> &wgpu::PipelineLayout {
		&self.render_pipeline_layout
	}
	pub fn bind_group_layouts(&self) -> &[BindGroupLayout] {
		&self.bind_group_layouts
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
