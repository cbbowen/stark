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

fn canvas_render_pipeline(
	device: &wgpu::Device,
	texture_format: wgpu::TextureFormat,
	shader: &render::Shader,
) -> wgpu::RenderPipeline {
	use shaders::canvas::*;
	let module = &shader.module;
	device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some("Render Pipeline"),
		layout: Some(&shader.layout),
		vertex: wgpu::VertexState {
			module,
			entry_point: ENTRY_VS_MAIN,
			compilation_options: Default::default(),
			buffers: &[],
		},
		fragment: Some(fragment_state(
			module,
			&fs_main_entry([Some(wgpu::ColorTargetState {
				format: texture_format,
				blend: Some(wgpu::BlendState::REPLACE),
				write_mask: wgpu::ColorWrites::ALL,
			})]),
		)),
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

fn create_canvas_texture_view(
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

fn create_canvas_sampler(device: &wgpu::Device) -> wgpu::Sampler {
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

fn create_drawing_action_bind_group(
	device: &wgpu::Device,
) -> (shaders::drawing::bind_groups::BindGroup0, wgpu::Buffer) {
	use shaders::drawing::bind_groups::*;
	let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("drawing_action"),
		contents: bytemuck::cast_slice(&[shaders::drawing::DrawingAction::zeroed()]),
		usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
	});
	let bind_group = BindGroup0::from_bindings(
		device,
		BindGroupLayout0 {
			action: buffer.as_entire_buffer_binding(),
		},
	);
	(bind_group, buffer)
}

fn create_canvas_bind_groups(
	device: &wgpu::Device,
	texture_view: &wgpu::TextureView,
	sampler: &wgpu::Sampler,
) -> (
	shaders::canvas::bind_groups::BindGroup0,
	shaders::canvas::bind_groups::BindGroup1,
) {
	use shaders::canvas::bind_groups::*;

	let chart_to_canvas_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("chart_to_canvas"),
		contents: bytemuck::cast_slice(&[geom::Similar2f::default().to_mat4x4_uniform()]),
		usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
	});

	let canvas_to_view =
		geom::Similar2f::new(geom::Scale2f::new(2.0), geom::Trans2f::new(-1.0, -1.0));
	let canvas_to_view_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("canvas_to_view"),
		contents: bytemuck::cast_slice(&[canvas_to_view.to_mat4x4_uniform()]),
		usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
	});

	(
		BindGroup0::from_bindings(
			device,
			BindGroupLayout0 {
				chart_to_canvas: chart_to_canvas_buffer.as_entire_buffer_binding(),
				chart_texture: texture_view,
				chart_sampler: sampler,
			},
		),
		BindGroup1::from_bindings(
			device,
			BindGroupLayout1 {
				canvas_to_view: canvas_to_view_buffer.as_entire_buffer_binding(),
			},
		),
	)
}

fn create_drawing_pipeline(
	device: &wgpu::Device,
	texture_format: wgpu::TextureFormat,
	shader: &render::Shader,
) -> wgpu::RenderPipeline {
	use shaders::drawing::*;
	device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some("Render Pipeline"),
		layout: Some(&shader.layout),
		vertex: wgpu::VertexState {
			module: &shader.module,
			entry_point: "vs_main",
			compilation_options: Default::default(),
			buffers: &[],
		},
		fragment: Some(fragment_state(
			&shader.module,
			&fs_main_entry([Some(wgpu::ColorTargetState {
				format: texture_format,
				blend: Some(wgpu::BlendState::ALPHA_BLENDING),
				write_mask: wgpu::ColorWrites::ALL,
			})]),
		)),
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
	let resources: render::Resources = expect_context();

	let texture_format = wgpu::TextureFormat::Rgba16Float;

	let (drawing_action_bind_group, drawing_action_buffer) =
		create_drawing_action_bind_group(context.device());
	let drawing_pipeline =
		create_drawing_pipeline(context.device(), texture_format, &resources.drawing);

	let canvas_texture_view = create_canvas_texture_view(context.device(), texture_format);
	let canvas_sampler = create_canvas_sampler(context.device());
	let (canvas_bind_group0, canvas_bind_group1) =
		create_canvas_bind_groups(context.device(), &canvas_texture_view, &canvas_sampler);
	let render_pipeline =
		canvas_render_pipeline(context.device(), texture_format, &resources.canvas);

	let redraw_trigger = create_trigger();
	// let interval = std::time::Duration::from_millis(1000);
	// crate::util::set_interval_and_clean_up(move || redraw_trigger.notify(), interval).ok_or_log();

	let render = {
		let context = context.clone();
		let canvas_bind_group0 = Rc::new(canvas_bind_group0);
		let canvas_bind_group1 = Rc::new(canvas_bind_group1);
		let render_pipeline = Rc::new(render_pipeline);
		create_derived(move || {
			let context = context.clone();
			redraw_trigger.track();
			let canvas_bind_group0 = canvas_bind_group0.clone();
			let canvas_bind_group1 = canvas_bind_group1.clone();
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
						..Default::default()
					});
					render_pass.set_pipeline(&render_pipeline);
					canvas_bind_group0.set(&mut render_pass);
					canvas_bind_group1.set(&mut render_pass);
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
		let canvas_texture_view = Rc::new(canvas_texture_view);
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
				bytemuck::cast_slice(&[shaders::drawing::DrawingAction {
					position: glam::Vec2::new(x as f32, y as f32),
				}]),
			);

			{
				let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
					label: Some("Drawing Pass"),
					color_attachments: &[
						// This is what @location(0) in the fragment shader targets
						Some(wgpu::RenderPassColorAttachment {
							view: &canvas_texture_view,
							resolve_target: None,
							ops: wgpu::Operations {
								load: wgpu::LoadOp::Load,
								store: wgpu::StoreOp::Store,
							},
						}),
					],
					..Default::default()
				});
				render_pass.set_pipeline(&drawing_pipeline);
				drawing_action_bind_group.set(&mut render_pass);
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
