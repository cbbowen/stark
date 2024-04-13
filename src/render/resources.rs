use std::rc::Rc;

/// Resources that only need to be loaded once for a given device.
#[derive(Debug, Clone)]
pub struct Resources {
	// Shader modules.
	pub canvas_shader_module: Rc<wgpu::ShaderModule>,
	pub drawing_shader_module: Rc<wgpu::ShaderModule>,

	// Bind group layouts.
	pub chart_bind_group_layout: Rc<wgpu::BindGroupLayout>,
}

impl Resources {
	pub fn new(device: &wgpu::Device) -> Self {
		Resources {
			// Shader modules.
			canvas_shader_module: Rc::new(
				device.create_shader_module(wgpu::include_wgsl!("canvas.wgsl")),
			),
			drawing_shader_module: Rc::new(
				device.create_shader_module(wgpu::include_wgsl!("drawing.wgsl")),
			),

			// Bind group layouts.
			chart_bind_group_layout: Rc::new(device.create_bind_group_layout(
				&wgpu::BindGroupLayoutDescriptor {
					label: Some("chart_bind_group_layout"),
					entries: &[
						// canvas_to_view
					wgpu::BindGroupLayoutEntry {
						binding: 0,
						visibility: wgpu::ShaderStages::VERTEX,
						ty: wgpu::BindingType::Buffer {
							ty: wgpu::BufferBindingType::Uniform,
							has_dynamic_offset: false,
							min_binding_size: None,
						},
						count: None,
					},
					// chart_texture
					wgpu::BindGroupLayoutEntry {
						binding: 1,
						visibility: wgpu::ShaderStages::FRAGMENT,
						ty: wgpu::BindingType::Texture {
							multisampled: false,
							view_dimension: wgpu::TextureViewDimension::D2,
							sample_type: wgpu::TextureSampleType::Float { filterable: true },
						},
						count: None,
					},
					// chart_sampler
					wgpu::BindGroupLayoutEntry {
						binding: 2,
						visibility: wgpu::ShaderStages::FRAGMENT,
						ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
						count: None,
					},
					],
				},
			)),
		}
	}
}
