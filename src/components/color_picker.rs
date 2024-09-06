use super::render_surface;
use crate::shaders::color_picker::*;
use crate::util::*;
use crate::{render, WgpuContext};
use leptos::{component, event_target_value, view, IntoView};
use leptos::{expect_context, prelude::*};
use std::rc::Rc;
use wgpu::util::DeviceExt;

fn create_bind_group(device: &wgpu::Device) -> (bind_groups::BindGroup0, wgpu::Buffer) {
	let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("drawing_action"),
		contents: bytemuck::cast_slice(&[0.5f32]),
		usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
	});
	let bind_group = bind_groups::BindGroup0::from_bindings(
		device,
		bind_groups::BindGroupLayout0 {
			lightness: buffer.as_entire_buffer_binding(),
		},
	);
	(bind_group, buffer)
}

fn create_render_pipeline(
	device: &wgpu::Device,
	texture_format: wgpu::TextureFormat,
	shader: &render::Shader,
) -> wgpu::RenderPipeline {
	device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some("ColorPicker"),
		layout: Some(&shader.layout),
		vertex: wgpu::VertexState {
			module: &shader.module,
			entry_point: ENTRY_VS_MAIN,
			compilation_options: Default::default(),
			buffers: &[],
		},
		fragment: Some(fragment_state(
			&shader.module,
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

#[component]
pub fn ColorPicker(color: leptos::RwSignal<glam::Vec3>) -> impl IntoView {
	// Create a lens into `color`.
	let lightness = create_memo(move |_| color.get().x);
	let set_lightness = move |l| color.update(|lab| lab.x = l);

	let context: Rc<WgpuContext> = expect_context();
	let resources: render::Resources = expect_context();

	let (texture_format, set_texture_format) = create_signal(None);

	let render_pipeline = {
		let context = context.clone();
		create_derived(move || {
			texture_format.get().map(|f| {
				Rc::new(create_render_pipeline(
					context.device(),
					f,
					&resources.color_picker,
				))
			})
		})
	};

	let (bind_group, buffer) = create_bind_group(context.device());

	let render = {
		let context = context.clone();
		let bind_group = Rc::new(bind_group);
		create_derived(move || {
			let context = context.clone();
			let bind_group = bind_group.clone();
			let Some(render_pipeline) = render_pipeline.get() else {
				return leptos::Callback::new(|_view| ());
			};

			let lightness = lightness.get();
			context
				.queue()
				.write_buffer(&buffer, 0, bytemuck::cast_slice(&[lightness as f32]));

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
						color_attachments: &[Some(wgpu::RenderPassColorAttachment {
							view: &view,
							resolve_target: None,
							ops: wgpu::Operations {
								load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
								store: wgpu::StoreOp::Store,
							},
						})],
						..Default::default()
					});
					render_pass.set_pipeline(&render_pipeline);
					bind_group.set(&mut render_pass);
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
			let (surface, width, height) = args;
			let config = surface.get_default_config(context.adapter(), width, height);
			tracing::info!(?config, "ColorPicker::configure");
			set_texture_format.set(config.as_ref().map(|c| c.format));
			config
		}
	};

	let touchstart = move |e: leptos::ev::TouchEvent| {
		e.prevent_default();
	};

	let pointermove = move |e: leptos::ev::PointerEvent| {
		if e.buttons() & 1 != 0 {
			let Some(xy) = e.get_target_coordinates() else {
				return;
			};
			let ab = (xy - glam::Vec2::new(-0.09, 0.24)) / 3.8;
			color.update(|lab| {
				lab.y = ab.x;
				lab.z = ab.y;
			});
		}
	};

	let pointerdown = move |e: leptos::ev::PointerEvent| {
		e.set_pointer_capture();
		e.prevent_default();
		pointermove(e);
	};

	view! {
		<div class="ColorPicker">
			<render_surface::RenderSurface
				render=render
				configure=configure
				on:touchstart=touchstart
				on:pointermove=pointermove
				on:pointerdown=pointerdown
			/>

			<input
				type="range"
				min="0"
				max="1"
				step="0.001"
				prop:value=lightness
				on:input=move |ev| { set_lightness(event_target_value(&ev).parse().unwrap()) }
			/>

			<span>{lightness}</span>
		</div>
	}
}
