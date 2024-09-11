use crate::components::*;
use crate::engine::{Airbrush, AirbrushDrawable, InputPoint};
use crate::render;
use crate::util::create_local_derived;
use crate::*;
use glam::Vec4Swizzles;
use leptos::prelude::*;
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
pub fn Canvas(#[prop(into)] drawing_color: Signal<glam::Vec3>) -> impl IntoView {
	let context: Arc<WgpuContext> = use_context().unwrap();
	let resources: Arc<render::Resources> = use_context().unwrap();

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

	let canvas_to_view = RwSignal::new(glam::Mat4::from_scale_rotation_translation(
		glam::Vec3::new(2.0, 2.0, 1.0),
		glam::Quat::IDENTITY,
		glam::Vec3::new(-1.0, -1.0, 0.0),
	));

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
		move |drawable: AirbrushDrawable, color: glam::Vec3| {
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
				drawable.draw(context.queue(), &mut render_pass, color);
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

			// Pan.
			if keys.is_pressed(" ") {
				if let Some(movement) = e.target_movement() {
					canvas_to_view.update(|mat| {
						*mat =
							glam::Mat4::from_translation(glam::vec3(movement.x, movement.y, 0.0)) * *mat;
					})
				}
				return;
			}

			// Draw.
			if let Some(position) = e.target_position() {
				let mut airbrush: std::cell::RefMut<_> = (*airbrush).borrow_mut();

				let pressure = e.pressure();
				let canvas_to_view = canvas_to_view.get();
				let view_to_canvas = canvas_to_view.inverse();
				let position = view_to_canvas * glam::vec4(position.x, position.y, 0.0, 1.0);
				let position = position.xy();
				if let Some(drawable) = airbrush.drag(InputPoint { position, pressure }) {
					draw(drawable, drawing_color.get_untracked());
				}
			};
		}
	};

	let pointerdown = {
		let pointermove = pointermove.clone();
		move |e: leptos::ev::PointerEvent| {
			e.set_pointer_capture();
			e.prevent_default();
			pointermove(e);
		}
	};

	let configured = move |configuration: wgpu::SurfaceConfiguration| {
		set_surface_texture_format.try_set_or_log(Some(configuration.format));
	};
	let configured = LocalCallback::new(configured);

	view! {
		<div class="Canvas">
			<RenderSurface
				render=render
				configured=configured
				on:touchstart=touchstart
				on:pointermove=pointermove
				on:pointerdown=pointerdown
			/>
		</div>
	}
}
