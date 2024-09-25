use crate::components::*;
use crate::render::{self, BindingBuffer};
use crate::util::create_local_derived;
use crate::*;
use engine::*;
use glam::*;
use leptos::prelude::*;
use leptos_use::{use_element_size, UseElementSizeReturn};
use std::sync::{Arc, RwLock};
use util::CoordinateSource;
use util::LocalCallback;
use util::PointerCapture;
use util::SetExt;

const MULTISAMPLE_COUNT: u32 = 4;

fn create_canvas_sampler(device: &wgpu::Device) -> wgpu::Sampler {
	device.create_sampler(&wgpu::SamplerDescriptor {
		address_mode_u: wgpu::AddressMode::ClampToEdge,
		address_mode_v: wgpu::AddressMode::ClampToEdge,
		address_mode_w: wgpu::AddressMode::ClampToEdge,
		mag_filter: wgpu::FilterMode::Nearest,
		min_filter: wgpu::FilterMode::Linear,
		mipmap_filter: wgpu::FilterMode::Linear,
		..Default::default()
	})
}

#[component]
pub fn Canvas(
	#[prop(into)] brush_color: Signal<Vec3>,
	#[prop(into)] brush_size: Signal<f64>,
	#[prop(into)] brush_rate: Signal<f64>,
	#[prop(into)] brush_opacity: Signal<f64>,
) -> impl IntoView {
	let context: Arc<WgpuContext> = use_context().unwrap();
	let device = context.device();
	let resources: Arc<render::Resources> = use_context().unwrap();

	let node_ref = NodeRef::new();
	let UseElementSizeReturn { width, height } = use_element_size(node_ref);

	let canvas_texture_format = wgpu::TextureFormat::Rgba16Float;
	let atlas = Atlas::new(context.clone(), canvas_texture_format);
	let atlas_buffer_layout = atlas.buffer_layout();
	let atlas = Arc::new(RwLock::new(atlas));

	let canvas_pipeline_layout = resources
		.canvas
		.pipeline_layout()
		.get();
	let canvas_sampler = create_canvas_sampler(&device);
	let canvas_to_view_buffer = BindingBuffer::init(&Mat4::ZERO)
		.label("canvas_to_view")
		.usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
		.create(&device);
	let canvas_bind_group = canvas_pipeline_layout
		.bind_group_layouts()
		.0
		.bind_group()
		.chart_sampler(&canvas_sampler)
		.canvas_to_view(canvas_to_view_buffer.as_entire_buffer_binding())
		.create();

	let (surface_configuration, set_surface_configuration) =
		signal_local::<Option<wgpu::SurfaceConfiguration>>(None);
	let surface_texture_format = Memo::new(move |_| surface_configuration.get().map(|c| c.format));
	let surface_texture_size =
		Memo::new(move |_| surface_configuration.get().map(|c| (c.width, c.height)));

	let render_pipeline = {
		let device = context.device().clone();
		let canvas_pipeline_layout = canvas_pipeline_layout.clone();
		let vertex_buffer_layouts = [atlas_buffer_layout];
		create_local_derived(move || {
			let pipeline = canvas_pipeline_layout
				.vs_main_pipeline(wgpu::VertexStepMode::Instance)
				.primitive(wgpu::PrimitiveState {
					topology: wgpu::PrimitiveTopology::TriangleStrip,
					..Default::default()
				})
				.fragment(shaders::canvas::FragmentEntry::fs_main {
					targets: [Some(wgpu::ColorTargetState {
						format: surface_texture_format.get()?,
						// TODO: We will probably need to change this to support layers.
						blend: Some(wgpu::BlendState::REPLACE),
						write_mask: wgpu::ColorWrites::ALL,
					})],
				})
				.multisample(wgpu::MultisampleState {
					count: MULTISAMPLE_COUNT,
					..Default::default()
				})
				.get();
			Some(Arc::new(pipeline))
		})
	};

	let surface_texture_view = {
		let device = context.device().clone();
		create_local_derived(move || {
			let size = surface_texture_size.get()?;
			Some(Arc::new(
				render::texture()
					.label("Canvas::surface_texture")
					.width(size.0)
					.height(size.1)
					.sample_count(MULTISAMPLE_COUNT)
					.format(surface_texture_format.get()?)
					.usage(wgpu::TextureUsages::RENDER_ATTACHMENT)
					.create(&device)
					.create_view(&wgpu::TextureViewDescriptor::default()),
			))
		})
	};

	let canvas_to_screen = RwSignal::new(Mat4::from_scale_rotation_translation(
		Vec3::new(1.0, 1.0, 1.0),
		Quat::IDENTITY,
		Vec3::new(-0.0, -0.0, 0.0),
	));

	// This is the mapping from normalized device coordinates to framebuffer coordinates.
	// Equivalently, it transforms `@builtin(position)` from the vertex to the fragment shader.
	let view_to_screen = create_local_derived(move || {
		let scale = 0.5 * vec2(width.get() as f32, height.get() as f32);
		let scale = vec3(scale.x, scale.y, 1.0);
		Mat4::from_scale(scale)
			* Mat4::from_translation(vec3(1.0, 1.0, 0.0))
			* Mat4::from_scale(vec3(1.0, -1.0, 1.0))
	});

	let screen_to_view = create_local_derived(move || view_to_screen.get().inverse());

	let canvas_to_view = create_local_derived(move || screen_to_view.get() * canvas_to_screen.get());

	let screen_to_canvas = create_local_derived(move || canvas_to_screen.get().inverse());

	let redraw_trigger = ArcTrigger::new();

	let render = {
		let context = context.clone();
		let atlas = atlas.clone();
		let canvas_bind_group = Arc::new(canvas_bind_group);
		let canvas_to_view_buffer = Arc::new(canvas_to_view_buffer);
		let redraw_trigger = redraw_trigger.clone();
		create_local_derived(move || {
			let context = context.clone();
			redraw_trigger.track();
			let atlas = atlas.clone();
			let canvas_bind_group = canvas_bind_group.clone();
			let canvas_to_view_buffer = canvas_to_view_buffer.clone();
			let render_pipeline = render_pipeline.get();
			let canvas_to_view = canvas_to_view.get();
			// let background_color = thaw::Theme::use_rw_theme()
			// 	.with(|theme| color_from_css_string(&theme.color.color_neutral_background_static));
			let surface_texture_view = surface_texture_view.get();
			let callback = move |view: wgpu::TextureView| {
				let Some(render_pipeline) = &render_pipeline else {
					return;
				};
				let Some(surface_texture_view) = &surface_texture_view else {
					return;
				};

				canvas_to_view_buffer.write(context.queue(), canvas_to_view);

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
								view: &surface_texture_view,
								resolve_target: Some(&view),
								ops: wgpu::Operations {
									load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
									store: wgpu::StoreOp::Store,
								},
							}),
						],
						..Default::default()
					});
					render_pass.set_pipeline(&render_pipeline);
					canvas_bind_group.set(&mut render_pass);

					let atlas = atlas.read().unwrap();
					// TODO: Only render the visible tiles.
					let charts: Vec<_> = atlas.charts().collect();
					let tiles: Vec<_> = charts.iter().map(|c| c.tile()).collect();
					draw_tiles(&mut render_pass, 0..4, &tiles);
				}
				context.queue().submit([encoder.finish()]);
			};
			Callback::new(callback)
		})
	};

	let airbrush = Airbrush::new(
		context.device(),
		context.queue(),
		&resources,
		canvas_texture_format,
	);
	let airbrush = std::rc::Rc::new(std::cell::RefCell::new(airbrush));

	let draw = {
		let context = context.clone();
		let atlas = atlas.clone();
		move |drawable: AirbrushDrawable| {
			let mut atlas = atlas.write().unwrap();

			let mut encoder =
				context
					.device()
					.create_command_encoder(&wgpu::CommandEncoderDescriptor {
						label: Some("Drawing Encoder"),
					});

			// Find the minimal set of tiles to write to.
			for chart_key in drawable.get_chart_keys() {
				let chart = atlas.get_chart_mut(chart_key);
				let view = chart.tile().texture_view();
				let chart_bind_group = chart.tile().write_bind_group();

				{
					let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
						label: Some("Drawing Pass"),
						color_attachments: &[
							// This is what @location(0) in the fragment shader targets
							Some(wgpu::RenderPassColorAttachment {
								view: &view,
								resolve_target: None,
								ops: wgpu::Operations {
									load: wgpu::LoadOp::Load,
									store: wgpu::StoreOp::Store,
								},
							}),
						],
						..Default::default()
					});
					chart_bind_group.set(&mut render_pass);
					drawable.draw(&mut render_pass);
				}
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
			let button0 = e.buttons() & 1 != 0;
			let button1 = e.buttons() & 2 != 0;
			let button2 = e.buttons() & 4 != 0;

			let screen_to_canvas = screen_to_canvas.get_untracked();

			let movement = {
				let screen_movement = e.pixel_movement();
				let movement =
					screen_to_canvas * vec4(screen_movement.x, screen_movement.y, 0f32, 0f32);
				movement.xy()
			};

			let position = {
				let screen_position = e.pixel_position();
				let position =
					screen_to_canvas * vec4(screen_position.x, screen_position.y, 0f32, 1f32);
				position.xy()
			};

			// Pan.
			if (button0 && keys.is_pressed(" ")) || button2 {
				canvas_to_screen.update(|m| {
					*m = (*m) * Mat4::from_translation(vec3(movement.x, movement.y, 0.0));
				});
				return;
			}

			// Draw.
			if button0 {
				let mut airbrush: std::cell::RefMut<_> = (*airbrush).borrow_mut();

				let pressure = e.pressure();
				let input_point = InputPoint {
					position,
					pressure,
					color: brush_color.get_untracked(),
					size: brush_size.get_untracked() as f32,
					opacity: brush_opacity.get_untracked() as f32,
					rate: brush_rate.get_untracked() as f32,
				};
				if let Some(drawable) = airbrush.drag(context.queue(), input_point) {
					draw(drawable);
				}
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

	let wheel = move |e: leptos::ev::WheelEvent| {
		let screen_to_canvas = screen_to_canvas.get_untracked();
		let position = {
			let screen_position = e.pixel_position();
			let position = screen_to_canvas * vec4(screen_position.x, screen_position.y, 0f32, 1f32);
			position.xy()
		};
		let translation = vec3(position.x, position.y, 0.0);

		let mut scale = 1.272;
		if e.delta_y() > 0.0 {
			scale = 1.0 / scale;
		}
		let transform = Mat4::from_translation(translation)
			* Mat4::from_scale(vec3(scale, scale, 1.0))
			* Mat4::from_translation(-translation);
		canvas_to_screen.update(|m| *m = (*m) * transform);
		e.prevent_default();
	};

	let configured = move |configuration: wgpu::SurfaceConfiguration| {
		set_surface_configuration.try_set_or_log(Some(configuration));
	};
	let configured = LocalCallback::new(configured);

	// let on_fetch_tile_texture_url = Trigger::new();
	// let texture_url = LocalResource::new(move || {
	// 	on_fetch_tile_texture_url.track();
	// 	let atlas = atlas.clone();
	// 	async move {
	// 		let chart = atlas.read().unwrap().get_chart(&ChartKey::find_containing(glam::Vec2::ZERO))?;
	// 		Some(chart.tile().encode_texture_as_url().await.unwrap())
	// 	}
	// });

	view! {
		<div class="Canvas" node_ref=node_ref>
			// <div class="debug">
			// <button on:click=move |_| { on_fetch_tile_texture_url.notify() }>"Fetch tile texture"</button>
			// // <a href=move || { texture_url.get().map(|s| s.take()).unwrap_or_default() } target="_blank">"Download texture"</a>
			// <img src=move || { texture_url.get().map(|s| s.take()).unwrap_or_default() } />
			// </div>
			<RenderSurface
				render=render
				configured=configured
				on:touchstart=touchstart
				on:pointermove=pointermove
				on:pointerdown=pointerdown
				on:pointerup=pointerup
				on:wheel=wheel
			/>
		</div>
	}
}
