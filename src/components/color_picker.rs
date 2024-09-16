use super::render_surface;
use crate::shaders::{self, color_picker::*};
use crate::util::*;
use crate::{render, WgpuContext};
use leptos::prelude::*;
use leptos::{component, view, IntoView};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use itertools::Itertools;

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
			&fs_main_entry(
				[Some(wgpu::ColorTargetState {
					format: texture_format,
					blend: Some(wgpu::BlendState::REPLACE),
					write_mask: wgpu::ColorWrites::ALL,
				})],
				&OverrideConstants { proof: None },
			),
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
pub fn ColorPicker(color: RwSignal<glam::Vec3>) -> impl IntoView {
	// Create a lens into `color`.
	let lightness = Memo::new(move |_| color.get().x);
	let set_lightness = move |l| color.update(|lab| lab.x = l);

	let context: Arc<WgpuContext> = use_context().unwrap();
	let resources: Arc<render::Resources> = use_context().unwrap();

	let (texture_format, set_texture_format) = signal(None);

	let render_pipeline = {
		let context = context.clone();
		create_local_derived(move || {
			Some(Arc::new(create_render_pipeline(
				context.device(),
				texture_format.get()?,
				&resources.color_picker,
			)))
		})
	};

	let (bind_group, buffer) = create_bind_group(context.device());

	let render = {
		let context = context.clone();
		let bind_group = Arc::new(bind_group);
		create_local_derived(move || {
			let context = context.clone();
			let bind_group = bind_group.clone();
			let render_pipeline = render_pipeline.get();

			let lightness = lightness.get();
			context
				.queue()
				.write_buffer(&buffer, 0, bytemuck::cast_slice(&[lightness as f32]));

			let callback = move |view: wgpu::TextureView| {
				let Some(render_pipeline) = render_pipeline.as_ref() else {
					return;
				};
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
			};
			Callback::new(callback)
		})
	};

	let touchstart = move |e: leptos::ev::TouchEvent| {
		e.prevent_default();
	};

	let pointermove = move |e: leptos::ev::PointerEvent| {
		if e.buttons() & 1 != 0 {
			let Some(xy) = e.target_position() else {
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

	let configured = move |configuration: wgpu::SurfaceConfiguration| {
		set_texture_format.set(Some(configuration.format));
	};
	let configured = LocalCallback::new(configured);

	let style = move || {
		let fractions = (0..=10).map(|i| i as f32 * 0.1);
		let color = color.get();
		let colors = fractions.clone().map(|l| oklab_to_rgb(glam::vec3(l, color.y, color.z)));
		let colors = colors.map(|c| {
			format!(
				"rgb({},{},{})",
				(c.x.clamp(0.0, 1.0) * 255.5) as u8,
				(c.y.clamp(0.0, 1.0) * 255.5) as u8,
				(c.z.clamp(0.0, 1.0) * 255.5) as u8,
			)
		});
		let mut gradient_percents = colors
			.into_iter()
			.zip(fractions.into_iter())
			.map(|(c, l)| format!("{} {}%", c, l * 100.0));
		format!(
			"background-image: linear-gradient(to right, {});",
			gradient_percents.join(", ")
		)
	};

	view! {
		<div class="ColorPicker">
			<render_surface::RenderSurface
				render=render
				configured=configured
				on:touchstart=touchstart
				on:pointermove=pointermove
				on:pointerdown=pointerdown
			/>

			<svg class="ColorPickerOverlay" width="300" height="300">
				<g
				transform="scale(300, 300)
				           translate(0.5, 0.5)
				           scale(0.5263, 0.5263)
							  translate(-0.09, -0.24)">
					<line x1="-1" y1="-1" x2="1" y2="1" stroke="gray" stroke-width="0.01"/>
					<line x1="1" y1="-1" x2="-1" y2="1" stroke="gray" stroke-width="0.01"/>
				</g>
			</svg>

			<input
				type="range"
				min="0.001"
				max="0.999"
				step="0.001"
				prop:value=lightness
				style=style
				on:input=move |ev| { set_lightness(event_target_value(&ev).parse().unwrap()) }
			/>
		</div>
	}
}
