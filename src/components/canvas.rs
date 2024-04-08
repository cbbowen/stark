use crate::components::*;
use crate::render;
use crate::util::create_derived;
use crate::util::ResultExt;
use leptos::*;
use std::rc::Rc;

#[component]
pub fn Canvas() -> impl IntoView {
	let context: render::Context = expect_context();
	let device = context.device;
	let queue = context.queue;

	let texture_format = wgpu::TextureFormat::Rgba16Float;

	let shader_module = device.create_shader_module(wgpu::include_wgsl!("canvas.wgsl"));
	let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
		label: Some("Render Pipeline Layout"),
		bind_group_layouts: &[],
		push_constant_ranges: &[],
	});
	let render_pipeline = Rc::new(
		device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("Render Pipeline"),
			layout: Some(&render_pipeline_layout),
			vertex: wgpu::VertexState {
				module: &shader_module,
				entry_point: "vs_main",
				buffers: &[],
			},
			fragment: Some(wgpu::FragmentState {
				module: &shader_module,
				entry_point: "fs_main",
				targets: &[Some(wgpu::ColorTargetState {
					// TODO: We need to get this from or put it in the `SurfaceConfiguration`.
					// format: wgpu::TextureFormat::Bgra8Unorm,
					format: wgpu::TextureFormat::Rgba16Float,
					blend: Some(wgpu::BlendState::REPLACE),
					write_mask: wgpu::ColorWrites::ALL,
				})],
			}),
			primitive: wgpu::PrimitiveState {
				topology: wgpu::PrimitiveTopology::TriangleList,
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
		}),
	);

	let redraw_trigger = create_trigger();
	let interval = std::time::Duration::from_millis(1000);
	crate::util::set_interval_and_clean_up(move || redraw_trigger.notify(), interval).ok_or_log();

	let render = create_derived(move || {
		// redraw_trigger.track();
		let device = device.clone();
		let queue = queue.clone();
		let render_pipeline = render_pipeline.clone();
		leptos::Callback::new(move |view: wgpu::TextureView| {
			tracing::trace!("render");

			let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
				render_pass.draw(0..3, 0..1);
			}
			queue.submit(std::iter::once(encoder.finish()));
		})
	});

	let configure = move |args: render_surface::ConfigureArgs| {
		tracing::info!("configure");
		let (surface, width, height) = args;
		let default = surface.get_default_config(&context.adapter, width, height)?;
		Some(wgpu::SurfaceConfiguration {
			format: texture_format,
			..default
		})
	};

	view! {
		<div class="Canvas">
			<RenderSurface render=render configure=configure/>
		</div>
	}
}
