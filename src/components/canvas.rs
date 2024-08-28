use crate::components::*;
use crate::render;
use crate::util::create_derived;
use crate::*;
use bytemuck::Zeroable;
use leptos::*;
use leptos_use::use_element_size;
use leptos_use::UseElementSizeReturn;
use std::rc::Rc;
use wgpu::util::DeviceExt;

// #[repr(C)]
// #[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// struct CanvasUniform {
//     view_proj: [[f32; 4]; 4],
// }

fn canvas_render_pipeline(
	device: &wgpu::Device,
	texture_format: wgpu::TextureFormat,
	render_drawing_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
	let resources: render::Resources = expect_context();
	let shader_module = resources.canvas_shader_module.clone();
	let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
		label: Some("Render Pipeline Layout"),
		bind_group_layouts: &[render_drawing_bind_group_layout],
		push_constant_ranges: &[],
	});
	device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some("Render Pipeline"),
		layout: Some(&render_pipeline_layout),
		vertex: wgpu::VertexState {
			module: &shader_module,
			entry_point: "vs_main",
			compilation_options: Default::default(),
			buffers: &[],
		},
		fragment: Some(wgpu::FragmentState {
			module: &shader_module,
			entry_point: "fs_main",
			compilation_options: Default::default(),
			targets: &[Some(wgpu::ColorTargetState {
				format: texture_format,
				blend: Some(wgpu::BlendState::REPLACE),
				write_mask: wgpu::ColorWrites::ALL,
			})],
		}),
		primitive: wgpu::PrimitiveState {
			topology: wgpu::PrimitiveTopology::TriangleStrip,
			strip_index_format: None,
			front_face: wgpu::FrontFace::Ccw,
			cull_mode: Some(wgpu::Face::Back),
			polygon_mode: wgpu::PolygonMode::Fill,
			unclipped_depth: false,
			conservative: false,
		},
		depth_stencil: None,
		multisample: wgpu::MultisampleState::default(),
		multiview: None,
		cache: None,
	})
}

fn create_drawing_texture_view(
	device: &wgpu::Device,
	texture_format: wgpu::TextureFormat,
) -> wgpu::TextureView {
	let texture = device.create_texture(&wgpu::TextureDescriptor {
		size: wgpu::Extent3d {
			width: 8192,
			height: 8192,
			depth_or_array_layers: 1,
		},
		mip_level_count: 1,
		sample_count: 1,
		dimension: wgpu::TextureDimension::D2,
		format: texture_format,
		usage: wgpu::TextureUsages::all(),
		label: Some("drawing_texture"),
		view_formats: &[texture_format],
	});
	texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn create_drawing_sampler(device: &wgpu::Device) -> wgpu::Sampler {
	device.create_sampler(&wgpu::SamplerDescriptor {
		address_mode_u: wgpu::AddressMode::ClampToEdge,
		address_mode_v: wgpu::AddressMode::ClampToEdge,
		address_mode_w: wgpu::AddressMode::ClampToEdge,
		mag_filter: wgpu::FilterMode::Linear,
		min_filter: wgpu::FilterMode::Nearest,
		mipmap_filter: wgpu::FilterMode::Nearest,
		..Default::default()
	})
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct DrawingActionUniform {
	position: [f32; 2],
}

fn create_drawing_action_bind_group(
	device: &wgpu::Device,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup, wgpu::Buffer) {
	let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		entries: &[wgpu::BindGroupLayoutEntry {
			binding: 0,
			visibility: wgpu::ShaderStages::VERTEX,
			ty: wgpu::BindingType::Buffer {
				ty: wgpu::BufferBindingType::Uniform,
				has_dynamic_offset: false,
				min_binding_size: None,
			},
			count: None,
		}],
		label: Some("drawing_action_bind_group_layout"),
	});
	let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("drawing_action"),
		contents: bytemuck::cast_slice(&[DrawingActionUniform::zeroed()]),
		usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
	});
	let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
		layout: &bind_group_layout,
		entries: &[wgpu::BindGroupEntry {
			binding: 0,
			resource: buffer.as_entire_binding(),
		}],
		label: Some("drawing_action_bind_group"),
	});
	(bind_group_layout, bind_group, buffer)
}

fn create_render_drawing_bind_group(
	device: &wgpu::Device,
	texture_view: &wgpu::TextureView,
	sampler: &wgpu::Sampler,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
	let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		entries: &[
			// chart_to_canvas
			wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStages::VERTEX,
				ty: wgpu::BindingType::Buffer {
					ty: wgpu::BufferBindingType::Uniform,
					has_dynamic_offset: false,
					min_binding_size: util::nonzero_size_of::<geom::Mat4x4fUniform>(),
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
				// This should match the filterable field of the
				// corresponding Texture entry above.
				ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
				count: None,
			},
		],
		label: Some("texture_bind_group_layout"),
	});
	let chart_to_canvas_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("chart_to_canvas"),
		contents: bytemuck::cast_slice(&[geom::Similar2f::default().to_mat4x4_uniform()]),
		usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
	});
	let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
		layout: &bind_group_layout,
		entries: &[
			// chart_to_canvas
			wgpu::BindGroupEntry {
				binding: 0,
				resource: chart_to_canvas_buffer.as_entire_binding(),
			},
			// chart_texture
			wgpu::BindGroupEntry {
				binding: 1,
				resource: wgpu::BindingResource::TextureView(texture_view),
			},
			// chart_sampler
			wgpu::BindGroupEntry {
				binding: 2,
				resource: wgpu::BindingResource::Sampler(sampler),
			},
		],
		label: Some("texture_bind_group"),
	});
	(bind_group_layout, bind_group)
}

fn create_drawing_pipeline(
	device: &wgpu::Device,
	texture_format: wgpu::TextureFormat,
	drawing_action_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
	let resources: render::Resources = expect_context();
	let shader_module = resources.drawing_shader_module;
	let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
		label: Some("Render Pipeline Layout"),
		bind_group_layouts: &[drawing_action_bind_group_layout],
		push_constant_ranges: &[],
	});
	device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some("Render Pipeline"),
		layout: Some(&render_pipeline_layout),
		vertex: wgpu::VertexState {
			module: &shader_module,
			entry_point: "vs_main",
			compilation_options: Default::default(),
			buffers: &[],
		},
		fragment: Some(wgpu::FragmentState {
			module: &shader_module,
			entry_point: "fs_main",
			compilation_options: Default::default(),
			targets: &[Some(wgpu::ColorTargetState {
				format: texture_format,
				blend: Some(wgpu::BlendState::ALPHA_BLENDING),
				write_mask: wgpu::ColorWrites::ALL,
			})],
		}),
		primitive: wgpu::PrimitiveState {
			topology: wgpu::PrimitiveTopology::TriangleStrip,
			strip_index_format: None,
			front_face: wgpu::FrontFace::Ccw,
			cull_mode: Some(wgpu::Face::Back),
			polygon_mode: wgpu::PolygonMode::Fill,
			unclipped_depth: false,
			conservative: false,
		},
		depth_stencil: None,
		multisample: wgpu::MultisampleState::default(),
		multiview: None,
		cache: None,
	})
}

#[component]
pub fn Canvas() -> impl IntoView {
	let context: Rc<WgpuContext> = expect_context();

	let texture_format = wgpu::TextureFormat::Rgba16Float;

	let (drawing_action_bind_group_layout, drawing_action_bind_group, drawing_action_buffer) =
		create_drawing_action_bind_group(context.device());
	let drawing_pipeline = create_drawing_pipeline(
		context.device(),
		texture_format,
		&drawing_action_bind_group_layout,
	);

	let drawing_texture_view = create_drawing_texture_view(context.device(), texture_format);
	let drawing_sampler = create_drawing_sampler(context.device());
	let (render_drawing_bind_group_layout, render_drawing_bind_group) =
		create_render_drawing_bind_group(context.device(), &drawing_texture_view, &drawing_sampler);
	let render_pipeline = canvas_render_pipeline(
		context.device(),
		texture_format,
		&render_drawing_bind_group_layout,
	);

	let redraw_trigger = create_trigger();
	// let interval = std::time::Duration::from_millis(1000);
	// crate::util::set_interval_and_clean_up(move || redraw_trigger.notify(), interval).ok_or_log();

	let render = {
		let context = context.clone();
		let render_drawing_bind_group = Rc::new(render_drawing_bind_group);
		let render_pipeline = Rc::new(render_pipeline);
		create_derived(move || {
			let context = context.clone();
			redraw_trigger.track();
			let render_drawing_bind_group = render_drawing_bind_group.clone();
			let render_pipeline = render_pipeline.clone();
			leptos::Callback::new(move |view: wgpu::TextureView| {
				let mut encoder =
					context
						.device()
						.create_command_encoder(&wgpu::CommandEncoderDescriptor {
							label: Some("Render Encoder"),
						});

				{
					let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
						label: Some("Render Pass"),
						color_attachments: &[
							// This is what @location(0) in the fragment shader targets
							Some(wgpu::RenderPassColorAttachment {
								view: &view,
								resolve_target: None,
								ops: wgpu::Operations {
									load: wgpu::LoadOp::Clear(wgpu::Color {
										r: 0.4,
										g: 0.3,
										b: 0.2,
										a: 1.0,
									}),
									store: wgpu::StoreOp::Store,
								},
							}),
						],
						depth_stencil_attachment: None,
						occlusion_query_set: None,
						timestamp_writes: None,
					});
					render_pass.set_pipeline(&render_pipeline);
					render_pass.set_bind_group(0, &render_drawing_bind_group, &[]);
					// TODO: Pass in uniforms for the camera.
					render_pass.draw(0..4, 0..1);
				}
				context.queue().submit([encoder.finish()]);
			})
		})
	};

	let configure = {
		let context = context.clone();
		move |args: render_surface::ConfigureArgs| {
			tracing::info!("configure");
			let (surface, width, height) = args;
			let default = surface.get_default_config(context.adapter(), width, height)?;
			Some(wgpu::SurfaceConfiguration {
				format: texture_format,
				..default
			})
		}
	};

	let draw = {
		let context = context.clone();
		let drawing_texture_view = Rc::new(drawing_texture_view);
		let drawing_action_bind_group = Rc::new(drawing_action_bind_group);
		let drawing_action_buffer = Rc::new(drawing_action_buffer);
		move |x: f64, y: f64| {
			let drawing_action_bind_group = drawing_action_bind_group.clone();
			let mut encoder =
				context
					.device()
					.create_command_encoder(&wgpu::CommandEncoderDescriptor {
						label: Some("Drawing Encoder"),
					});

			context.queue().write_buffer(
				&drawing_action_buffer,
				0,
				bytemuck::cast_slice(&[DrawingActionUniform {
					position: [x as f32, y as f32],
				}]),
			);

			{
				let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
					label: Some("Drawing Pass"),
					color_attachments: &[
						// This is what @location(0) in the fragment shader targets
						Some(wgpu::RenderPassColorAttachment {
							view: &drawing_texture_view,
							resolve_target: None,
							ops: wgpu::Operations {
								load: wgpu::LoadOp::Load,
								store: wgpu::StoreOp::Store,
							},
						}),
					],
					depth_stencil_attachment: None,
					occlusion_query_set: None,
					timestamp_writes: None,
				});
				render_pass.set_pipeline(&drawing_pipeline);
				render_pass.set_bind_group(0, &drawing_action_bind_group, &[]);
				// TODO: Pass in uniforms for the position and other parameters.
				// https://sotrh.github.io/learn-wgpu/beginner/tutorial6-uniforms/#uniform-buffers-and-bind-groups
				render_pass.draw(0..4, 0..1);
			}
			context.queue().submit(std::iter::once(encoder.finish()));
			redraw_trigger.notify();
		}
	};

	let render_surface_element = create_node_ref();
	let UseElementSizeReturn { width, height } = use_element_size(render_surface_element);

	let mousemove = move |e: leptos::ev::MouseEvent| {
		let width = width.get_untracked();
		let height = height.get_untracked();
		if e.buttons() & 1 != 0 {
			draw(e.x() as f64 / width, e.y() as f64 / height);
		}
	};

	view! {
		<div class="Canvas">
			<RenderSurface
				node_ref=render_surface_element
				render=render
				configure=configure
				on:mousemove=mousemove
			/>
		</div>
	}
}
