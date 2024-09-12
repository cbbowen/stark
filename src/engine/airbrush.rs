use crate::render;
use crate::render::Resources;
use crate::shaders::airbrush::*;
use encase::ShaderType;
use glam::Vec2;
use wgpu::util::DeviceExt;

fn create_vertex_buffer(device: &wgpu::Device) -> (wgpu::VertexBufferLayout<'static>, wgpu::Buffer) {
	let layout = VertexInput::vertex_buffer_layout(wgpu::VertexStepMode::Vertex);
	let buffer = device.create_buffer(&wgpu::BufferDescriptor {
		label: Some("airbrush::create_vertex_buffer"),
		size: layout.array_stride * 4,
		usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
		mapped_at_creation: false,
	});
	(layout, buffer)
}

fn create_pipeline(
	device: &wgpu::Device,
	texture_format: wgpu::TextureFormat,
	shader: &render::Shader,
	vertex_buffer_layout: wgpu::VertexBufferLayout<'_>,
) -> wgpu::RenderPipeline {
	device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some("airbrush"),
		layout: Some(&shader.layout),
		vertex: wgpu::VertexState {
			module: &shader.module,
			entry_point: ENTRY_VS_MAIN,
			compilation_options: Default::default(),
			buffers: &[vertex_buffer_layout],
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

fn create_bind_group(device: &wgpu::Device) -> (bind_groups::BindGroup0, wgpu::Buffer) {
	use bind_groups::*;
	let contents: Vec<_> = std::iter::repeat(0u8)
		.take(AirbrushAction::min_size().get() as usize)
		.collect();
	let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("airbrush"),
		contents: bytemuck::cast_slice(&contents),
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

#[derive(Clone, Copy)]
pub struct InputPoint {
	pub position: glam::Vec2,
	pub pressure: f32,
	pub color: glam::Vec3,
	pub size: f32,
	pub opacity: f32,
	pub softness: f32,
}

pub struct Airbrush {
	pipeline: wgpu::RenderPipeline,
	bind_group: bind_groups::BindGroup0,
	action_buffer: wgpu::Buffer,
	vertex_buffer: wgpu::Buffer,
	last_point: Option<InputPoint>,
}

pub struct AirbrushDrawable<'tool> {
	tool: &'tool Airbrush,
}

impl Airbrush {
	pub fn new(
		device: &wgpu::Device,
		resources: &Resources,
		texture_format: wgpu::TextureFormat,
	) -> Self {
		let (vertex_buffer_layout, vertex_buffer) = create_vertex_buffer(device);
		let pipeline = create_pipeline(device, texture_format, &resources.airbrush, vertex_buffer_layout);
		let (bind_group, action_buffer) = create_bind_group(device);
		Self {
			pipeline,
			bind_group,
			action_buffer,
			vertex_buffer,
			last_point: None,
		}
	}

	pub fn start(&mut self) {
	}

	pub fn drag(&mut self, queue: &wgpu::Queue, point: InputPoint) -> Option<AirbrushDrawable<'_>> {
		let last_point = self.last_point.replace(point)?;

		let p0 = last_point.position;
		let p1 = point.position;
		let tangent = (p1 - p0).normalize_or(Vec2::X);
		let normal = tangent.perp();
		let s0 = last_point.size * last_point.pressure;
		let s1 = point.size * point.pressure;

		let mut contents = encase::UniformBuffer::new(Vec::<u8>::new());
		contents
			.write(&AirbrushAction {
				seed: glam::Vec2::new(fastrand::f32(), fastrand::f32()),
				color: point.color,
				pressure: point.pressure,
				opacity: point.opacity,
				softness:point.softness,
			})
			.unwrap();
		queue.write_buffer(&self.action_buffer, 0, &contents.into_inner());

		let vertices = [
			p0 - s0 * tangent + s0 * normal,
			p0 - s0 * tangent - s0 * normal,
			p1 + s1 * tangent + s1 * normal,
			p1 + s1 * tangent - s1 * normal,
		];
		queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));

		Some(AirbrushDrawable {
			tool: self,
		})
	}

	pub fn stop(&mut self) {
		self.last_point = None;
	}
}

impl<'tool> AirbrushDrawable<'tool> {
	pub fn draw(
		&self,
		render_pass: &mut wgpu::RenderPass<'_>,
	) {
		render_pass.set_pipeline(&self.tool.pipeline);
		self.tool.bind_group.set(render_pass);
		render_pass.set_vertex_buffer(0, self.tool.vertex_buffer.slice(..));
		// TODO: Pass in uniforms for the position and other parameters.
		// https://sotrh.github.io/learn-wgpu/beginner/tutorial6-uniforms/#uniform-buffers-and-bind-groups
		render_pass.draw(0..4, 0..1);
	}
}
