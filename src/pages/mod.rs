use crate::components::*;
use crate::util::*;
use leptos::*;
use render_canvas::*;
use std::rc::Rc;

fn test_renderable() -> Renderable {
	let render_context: Rc<RenderContext> = expect_context();
	let adapter = render_context.adapter();

	if adapter.is_none() {
		return Rc::new(|_view: wgpu::TextureView| Ok(()));
	}
	let adapter = adapter.unwrap();

	let device = adapter.device();
	let shader_module =
		device.create_shader_module(wgpu::include_wgsl!("../renderables/shaders.wgsl"));
	let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
		label: Some("Render Pipeline Layout"),
		bind_group_layouts: &[],
		push_constant_ranges: &[],
	});
	let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
	});

	let renderable = move |view: wgpu::TextureView| {
		let device = adapter.device();
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

		// submit will accept anything that implements IntoIter
		adapter.queue().submit(std::iter::once(encoder.finish()));
		Ok(())
	};
	Rc::new(renderable)
}

#[component]
pub fn Home() -> impl IntoView {
	let test_renderable = create_cache(test_renderable);
	let (renderable, set_renderable) = create_signal::<render_canvas::Renderable>(test_renderable());

	let interval = std::time::Duration::from_millis(1000);
	set_interval_and_clean_up(move || set_renderable(test_renderable()), interval).ok_or_log();

	view! { <div>"Bruh." <RenderCanvas renderable=renderable/></div> }
}

#[component]
pub fn NotFound() -> impl IntoView {
	view! { "Not found" }
}
