use core::f32;

use crate::render::*;
use crate::shaders::{copy_transform, horizontal_scan, log_transform};
use bon::builder;
use glam::*;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GenerateRotationsError {
	#[error("wrong texture dimension: {0:?}")]
	WrongTextureDimension(wgpu::TextureDimension),
}

#[builder(finish_fn = generate)]
pub fn rotations(
	#[builder(start_fn)] rotations: u32,
	#[builder(finish_fn)] device: &wgpu::Device,
	#[builder(finish_fn)] queue: &wgpu::Queue,
	#[builder(finish_fn)] resources: &Resources,
	source: &wgpu::Texture,
	#[builder(default)] layer_index: u32,
	format: Option<wgpu::TextureFormat>,
	#[builder(default = wgpu::TextureUsages::all())] usage: wgpu::TextureUsages,
) -> Result<wgpu::Texture, GenerateRotationsError> {
	use GenerateRotationsError::*;
	if source.dimension() != wgpu::TextureDimension::D2 {
		Err(WrongTextureDimension(source.dimension()))?;
	}

	let size = (source.width().max(source.height()) as f32 * 2f32.sqrt()).ceil() as u32;
	let scale = vec2(
		source.width() as f32 / size as f32,
		source.height() as f32 / size as f32,
	);

	let format = format.unwrap_or(source.format());

	let output_texture = texture()
		.label("generate_rotations::output_texture")
		.width(size)
		.height(size)
		.array_layers(rotations)
		.format(format)
		.usage(usage | wgpu::TextureUsages::RENDER_ATTACHMENT)
		.create(device);

	let copy_transform_shader = &resources.copy_transform;
	let copy_transform_pipeline = render_pipeline()
		.label("generate_rotations::copy_transform")
		.layout(&copy_transform_shader.layout)
		.vertex(wgpu::VertexState {
			module: &copy_transform_shader.module,
			entry_point: copy_transform::ENTRY_VS_MAIN,
			compilation_options: Default::default(),
			buffers: &[],
		})
		.fragment(copy_transform::fragment_state(
			&copy_transform_shader.module,
			&copy_transform::fs_main_entry([Some(wgpu::ColorTargetState {
				format,
				blend: Some(wgpu::BlendState::REPLACE),
				write_mask: wgpu::ColorWrites::ALL,
			})]),
		))
		.create(device);

	let source_view = source.create_view(&wgpu::TextureViewDescriptor {
		label: Some("generate_rotations::source_view"),
		base_array_layer: layer_index,
		array_layer_count: Some(1),
		..Default::default()
	});

	let source_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
		address_mode_u: wgpu::AddressMode::ClampToEdge,
		address_mode_v: wgpu::AddressMode::ClampToEdge,
		address_mode_w: wgpu::AddressMode::ClampToEdge,
		mag_filter: wgpu::FilterMode::Linear,
		min_filter: wgpu::FilterMode::Linear,
		mipmap_filter: wgpu::FilterMode::Linear,
		..Default::default()
	});

	let mut command_encoder = device.create_command_encoder(&Default::default());
	let rotation_step = f32::consts::TAU / rotations as f32;
	for rotation in 0..rotations {
		let destination_view = output_texture.create_view(&wgpu::TextureViewDescriptor {
			label: Some("generate_rotations::destination_view"),
			base_array_layer: rotation,
			array_layer_count: Some(1),
			dimension: Some(wgpu::TextureViewDimension::D2),
			..Default::default()
		});

		let transform_buffer = BindingBuffer::init_sized(&Mat2::from_scale_angle(
			scale,
			rotation_step * rotation as f32,
		))
		.create(device);

		let bind_group = copy_transform::bind_groups::BindGroup0::from_bindings(
			device,
			copy_transform::bind_groups::BindGroupLayout0 {
				transform: transform_buffer.as_entire_buffer_binding(),
				source_texture: &source_view,
				source_sampler: &source_sampler,
			},
		);

		let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &destination_view,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
					store: wgpu::StoreOp::Store,
				},
			})],
			..Default::default()
		});
		render_pass.set_pipeline(&copy_transform_pipeline);
		bind_group.set(&mut render_pass);
		render_pass.draw(0..4, 0..1);
	}
	queue.submit([command_encoder.finish()]);

	Ok(output_texture)
}

#[builder(finish_fn = generate)]
pub fn log_transform(
	#[builder(start_fn)] source: &wgpu::Texture,
	#[builder(finish_fn)] device: &wgpu::Device,
	#[builder(finish_fn)] queue: &wgpu::Queue,
	#[builder(finish_fn)] resources: &Resources,
	#[builder(default)] layer_index: u32,
	#[builder(default = wgpu::TextureUsages::all())] usage: wgpu::TextureUsages,
	#[builder(default = &[])] view_formats: &[wgpu::TextureFormat],
) -> wgpu::Texture {
	use log_transform::*;

	let destination = texture()
		.label("log_transform::destination")
		.width(source.width())
		.height(source.height())
		// This must match the format in the the shader.
		.format(wgpu::TextureFormat::R32Float)
		.view_formats(view_formats)
		.usage(usage | wgpu::TextureUsages::STORAGE_BINDING)
		.create(device);

	let shader = &resources.log_transform;
	let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
		label: Some("log_transform"),
		layout: Some(&shader.layout),
		module: &shader.module,
		entry_point: "log_transform",
		compilation_options: Default::default(),
		cache: None,
	});

	let source_view = source.create_view(&wgpu::TextureViewDescriptor {
		label: Some("log_transform::source"),
		base_array_layer: layer_index,
		array_layer_count: Some(1),
		dimension: Some(wgpu::TextureViewDimension::D2),
		..Default::default()
	});

	let destination_view = destination.create_view(&wgpu::TextureViewDescriptor {
		label: Some("log_transform::destination"),
		..Default::default()
	});

	let bind_group = bind_groups::BindGroup0::from_bindings(
		device,
		bind_groups::BindGroupLayout0 {
			source: &source_view,
			destination: &destination_view,
		},
	);

	let mut command_encoder = device.create_command_encoder(&Default::default());
	{
		let mut pass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
			label: Some("log_transform"),
			..Default::default()
		});
		let x_workgroups = (source.width() + WORKGROUP_WIDTH - 1) / WORKGROUP_WIDTH;
		let y_workgroups = (source.height() + WORKGROUP_HEIGHT - 1) / WORKGROUP_HEIGHT;
		pass.set_pipeline(&pipeline);
		bind_group.set(&mut pass);
		pass.dispatch_workgroups(x_workgroups, y_workgroups, 1);
	}
	queue.submit([command_encoder.finish()]);

	destination
}

#[builder(finish_fn = generate)]
pub fn horizontal_scan(
	#[builder(start_fn)] source: &wgpu::Texture,
	#[builder(finish_fn)] device: &wgpu::Device,
	#[builder(finish_fn)] queue: &wgpu::Queue,
	#[builder(finish_fn)] resources: &Resources,
	#[builder(default)] layer_index: u32,
	#[builder(default = wgpu::TextureUsages::all())] usage: wgpu::TextureUsages,
	#[builder(default = &[])] view_formats: &[wgpu::TextureFormat],
) -> wgpu::Texture {
	use horizontal_scan::*;

	let destination = texture()
		.label("generate_rotations::destination")
		.width(source.width())
		.height(source.height())
		// This must match the format in the the shader.
		.format(wgpu::TextureFormat::R32Float)
		.view_formats(view_formats)
		.usage(usage | wgpu::TextureUsages::STORAGE_BINDING)
		.create(device);

	let shader = &resources.horizontal_scan;
	let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
		label: Some("horizontal_scan"),
		layout: Some(&shader.layout),
		module: &shader.module,
		entry_point: "horizontal_scan",
		compilation_options: Default::default(),
		cache: None,
	});

	let source_view = source.create_view(&wgpu::TextureViewDescriptor {
		label: Some("horizontal_scan::source"),
		base_array_layer: layer_index,
		array_layer_count: Some(1),
		dimension: Some(wgpu::TextureViewDimension::D2),
		..Default::default()
	});

	let destination_view = destination.create_view(&wgpu::TextureViewDescriptor {
		label: Some("horizontal_scan::destination"),
		..Default::default()
	});

	let bind_group = bind_groups::BindGroup0::from_bindings(
		device,
		bind_groups::BindGroupLayout0 {
			source: &source_view,
			destination: &destination_view,
		},
	);

	let mut command_encoder = device.create_command_encoder(&Default::default());
	{
		let mut pass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
			label: Some("horizontal_scan"),
			..Default::default()
		});
		let num_workgroups = (source.height() + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
		pass.set_pipeline(&pipeline);
		bind_group.set(&mut pass);
		pass.dispatch_workgroups(num_workgroups, 1, 1);
	}
	queue.submit([command_encoder.finish()]);

	destination
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::test::*;

	#[test]
	fn test_rotations() -> anyhow::Result<()> {
		let context = WgpuTestContext::new()?;
		let resources = Resources::new(context.device());
		let source = context.create_image_texture("test/input/cs-gray-7f7f7f.png")?;
		const ROTATIONS: u32 = 3;
		let result = rotations(ROTATIONS)
			.source(&source)
			.usage(wgpu::TextureUsages::COPY_SRC)
			.generate(context.device(), context.queue(), &resources)?;
		for rotation in 0..ROTATIONS {
			context.golden_texture(
				&format!("engine/process_shape/rotations_{rotation}"),
				GoldenOptions::default(),
				&result,
				rotation,
			)?;
		}
		Ok(())
	}

	#[test]
	fn test_rotations_texture_format() -> anyhow::Result<()> {
		let context = WgpuTestContext::new()?;
		let resources = Resources::new(context.device());
		let source = context.create_image_texture("test/input/cs-gray-7f7f7f.png")?;
		let format = wgpu::TextureFormat::R8Unorm;
		let result = rotations(1)
			.source(&source)
			.format(format)
			.usage(wgpu::TextureUsages::COPY_SRC)
			.generate(context.device(), context.queue(), &resources)?;
		context.golden_texture(
			"engine/process_shape/rotations_texture_format",
			GoldenOptions::default(),
			&result,
			0,
		)?;
		Ok(())
	}

	#[test]
	fn test_log_transform() -> anyhow::Result<()> {
		let context = WgpuTestContext::new()?;
		let resources = Resources::new(context.device());
		let source = context.create_image_texture("test/input/cs-gray-7f7f7f.png")?;

		let destination = log_transform(&source)
			.generate(&context.device(), context.queue(), &resources);

		context.golden_texture(
			"engine/process_shape/log_transform",
			GoldenOptions::default(),
			&destination,
			0,
		)?;
		Ok(())
	}

	#[test]
	fn test_horizontal_scan() -> anyhow::Result<()> {
		let context = WgpuTestContext::new()?;
		let resources = Resources::new(context.device());
		let source = context.create_image_texture("test/input/cs-gray-7f7f7f.png")?;

		let destination = horizontal_scan(&source)
			.generate(&context.device(), context.queue(), &resources);

		context.golden_texture(
			"engine/process_shape/horizontal_scan",
			GoldenOptions::default(),
			&destination,
			0,
		)?;
		Ok(())
	}
}
