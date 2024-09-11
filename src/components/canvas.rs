use crate::components::*;
use crate::engine::{Airbrush, AirbrushDrawable, InputPoint};
use crate::render;
use crate::util::create_local_derived;
use crate::*;
use glam::*;
use leptos::prelude::*;
use leptos_use::{use_element_size, UseElementSizeReturn};
use std::sync::Arc;
use util::CoordinateSource;
use util::LocalCallback;
use util::PointerCapture;
use util::SetExt;
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
			cull_mode: None,
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
			width: 4096,
			height: 4096,
			depth_or_array_layers: 1,
		},
		mip_level_count: 1,
		sample_count: 1,
		dimension: wgpu::TextureDimension::D2,
		format: texture_format,
		usage: wgpu::TextureUsages::all(),
		label: Some("canvas_texture"),
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

fn create_canvas_bind_groups(
	device: &wgpu::Device,
	texture_view: &wgpu::TextureView,
	sampler: &wgpu::Sampler,
) -> (
	shaders::canvas::bind_groups::BindGroup0,
	shaders::canvas::bind_groups::BindGroup1,
	wgpu::Buffer,
) {
	use shaders::canvas::bind_groups::*;

	let chart_to_canvas_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("chart_to_canvas"),
		contents: bytemuck::cast_slice(&[glam::Mat4::IDENTITY]),
		usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
	});

	let canvas_to_view_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("canvas_to_view"),
		contents: bytemuck::cast_slice(&[glam::Mat4::ZERO]),
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
		canvas_to_view_buffer,
	)
}

#[component]
pub fn Canvas(
	#[prop(into)] brush_color: Signal<Vec3>,
	#[prop(into)] brush_size: Signal<f64>,
	#[prop(into)] brush_softness: Signal<f64>,
) -> impl IntoView {
	let context: Arc<WgpuContext> = use_context().unwrap();
	let resources: Arc<render::Resources> = use_context().unwrap();

	let node_ref = NodeRef::new();
	let UseElementSizeReturn { width, height } = use_element_size(node_ref);

	let canvas_sampler = create_canvas_sampler(context.device());

	let canvas_texture_format = wgpu::TextureFormat::Rgba16Float;
	let (surface_texture_format, set_surface_texture_format) = signal_local(None);

	let canvas_texture_view = create_canvas_texture_view(context.device(), canvas_texture_format);
	let (canvas_bind_group0, canvas_bind_group1, canvas_to_view_buffer) =
		create_canvas_bind_groups(context.device(), &canvas_texture_view, &canvas_sampler);

	let render_pipeline = {
		let context = context.clone();
		let resources = resources.clone();
		create_local_derived(move || {
			Some(Arc::new(canvas_render_pipeline(
				context.device(),
				surface_texture_format.get()?,
				&resources.canvas,
			)))
		})
	};

	let canvas_to_screen = RwSignal::new(Mat4::from_scale_rotation_translation(
		Vec3::new(4096.0, 4096.0, 1.0),
		Quat::IDENTITY,
		Vec3::new(-2048.0, -2048.0, 0.0),
	));

	// This is the mapping from normalized device coordinates to framebuffer coordinates.
	// Equivalently, it transforms `@builtin(position)` from the vertex to the fragment shader.
	let view_to_screen = create_local_derived(move || {
		let scale = 0.5 * vec2(width.get() as f32, height.get() as f32);
		let scale = vec3(scale.x, scale.y, 1.0);
		Mat4::from_scale(scale) * Mat4::from_translation(vec3(1.0, 1.0, 0.0)) * Mat4::from_scale(vec3(1.0, -1.0, 1.0))
	});

	let screen_to_view = create_local_derived(move || view_to_screen.get().inverse());

	let canvas_to_view = create_local_derived(move || screen_to_view.get() * canvas_to_screen.get());

	let redraw_trigger = ArcTrigger::new();

	let render = {
		let context = context.clone();
		let canvas_bind_group0 = Arc::new(canvas_bind_group0);
		let canvas_bind_group1 = Arc::new(canvas_bind_group1);
		let canvas_to_view_buffer = Arc::new(canvas_to_view_buffer);
		let redraw_trigger = redraw_trigger.clone();
		create_local_derived(move || {
			let context = context.clone();
			redraw_trigger.track();
			let canvas_bind_group0 = canvas_bind_group0.clone();
			let canvas_bind_group1 = canvas_bind_group1.clone();
			let canvas_to_view_buffer = canvas_to_view_buffer.clone();
			let render_pipeline = render_pipeline.get();
			let canvas_to_view = canvas_to_view.get();
			let callback = move |view: wgpu::TextureView| {
				let Some(render_pipeline) = render_pipeline.clone() else {
					return;
				};

				context.queue().write_buffer(
					&canvas_to_view_buffer,
					0,
					bytemuck::cast_slice(&[canvas_to_view]),
				);

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
					render_pass.draw(0..4, 0..1);
				}
				context.queue().submit([encoder.finish()]);
			};
			LocalCallback::new(callback)
		})
	};

	let airbrush = Airbrush::new(context.device(), &resources, canvas_texture_format);
	let airbrush = std::rc::Rc::new(std::cell::RefCell::new(airbrush));

	let draw = {
		let context = context.clone();
		let canvas_texture_view = Arc::new(canvas_texture_view);
		move |drawable: AirbrushDrawable| {
			let mut encoder =
				context
					.device()
					.create_command_encoder(&wgpu::CommandEncoderDescriptor {
						label: Some("Drawing Encoder"),
					});

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
				drawable.draw(&mut render_pass);
			}
			context.queue().submit(std::iter::once(encoder.finish()));
			redraw_trigger.notify();
		}
	};

	let touchstart = move |e: leptos::ev::TouchEvent| {
		e.prevent_default();
	};

	let keys: KeyboardState = expect_context();

	let pointermove = {
		let airbrush = airbrush.clone();
		move |e: leptos::ev::PointerEvent| {
			if e.buttons() & 1 == 0 {
				return;
			}

			let screen_to_canvas = canvas_to_screen.get_untracked().inverse();

			let movement = {
				let screen_movement = e.pixel_movement();
				let movement = screen_to_canvas * vec4(screen_movement.x, screen_movement.y, 0f32, 0f32);
				movement.xy()
			};

			let position = {
				let screen_position = e.pixel_position();
				let position = screen_to_canvas * vec4(screen_position.x, screen_position.y, 0f32, 1f32);
				tracing::trace!(?screen_position, ?position, "pointermove");
				position.xy()
			};

			// Pan.
			if keys.is_pressed(" ") {
				tracing::trace!(?movement, "pointermove");
				canvas_to_screen.update(|m| {
					*m = (*m) * Mat4::from_translation(vec3(movement.x, movement.y, 0.0));
				});
				return;
			}

			// Draw.
			let mut airbrush: std::cell::RefMut<_> = (*airbrush).borrow_mut();

			let pressure = e.pressure();
			tracing::trace!(?position, "pointermove");
			let input_point = InputPoint {
				position,
				pressure,
				color: brush_color.get_untracked(),
				size: brush_size.get_untracked() as f32,
				softness: brush_softness.get_untracked() as f32,
			};
			if let Some(drawable) = airbrush.drag(context.queue(), input_point) {
				draw(drawable);
			}
		}
	};

	let pointerdown = {
		let airbrush = airbrush.clone();
		let pointermove = pointermove.clone();
		move |e: leptos::ev::PointerEvent| {
			(*airbrush).borrow_mut().start();

			e.set_pointer_capture();
			e.prevent_default();
			pointermove(e);
		}
	};

	let pointerup = {
		let airbrush = airbrush.clone();
		move |e: leptos::ev::PointerEvent| {
			(*airbrush).borrow_mut().stop();
			e.prevent_default();
		}
	};

	let configured = move |configuration: wgpu::SurfaceConfiguration| {
		set_surface_texture_format.try_set_or_log(Some(configuration.format));
	};
	let configured = LocalCallback::new(configured);

	view! {
		<div class="Canvas" node_ref=node_ref>
			<RenderSurface
				render=render
				configured=configured
				on:touchstart=touchstart
				on:pointermove=pointermove
				on:pointerdown=pointerdown
				on:pointerup=pointerup
			/>
		</div>
	}
}
