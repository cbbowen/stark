pub mod interface;
pub mod internal;

use duplicate::duplicate_item;

#[duplicate_item(
	name;
	[ atlas ];
	[ canvas ];
	[ airbrush ];
	[ color_picker ];
	[ copy_transform ];
 )]
pub mod name {
	use super::interface;
	use super::internal::name;

	#[derive(Debug, Default)]
	pub struct Interface;

	impl interface::RenderShaderInterface for Interface {
		const VERTEX_ENTRY: &str = name::ENTRY_VS_MAIN;
		const FRAGMENT_ENTRY: &str = name::ENTRY_FS_MAIN;
		const NUM_VERTEX_INPUTS: usize = name::NUM_VERTEX_INPUTS_VS_MAIN;
		const NUM_TARGETS: usize = name::NUM_TARGETS_FS_MAIN;
		type OverrideConstants = name::OverrideConstants;

		type FragmentEntry = name::FragmentEntry<{ Self::NUM_TARGETS }>;
		fn fragment_entry(
			targets: [Option<wgpu::ColorTargetState>; Self::NUM_TARGETS],
			overrides: &Self::OverrideConstants,
		) -> Self::FragmentEntry {
			name::fs_main_entry(targets, overrides)
		}
		fn fragment_state<'a>(
			module: &'a wgpu::ShaderModule,
			entry: &'a Self::FragmentEntry,
		) -> wgpu::FragmentState<'a> {
			name::fragment_state(module, entry)
		}
		fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
			name::create_shader_module(device)
		}
		fn create_pipeline_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
			name::create_pipeline_layout(device)
		}
	}

	// Forward everything from the shader. It would be nice if we could be a bit more selective here.
	pub use name::*;

	pub type Shader = interface::RenderShader<Interface>;
}

// `cargo expand --lib shaders::horizontal_scan`

// Directly expose the compute shaders for now.
pub use internal::horizontal_scan;
pub use internal::log_transform;

// Expose parts of the tile read/write templates.
pub use internal::tile_read_internal::TileData;
pub mod tile_read {
	pub use super::internal::tile_read_internal::bind_groups::BindGroup1 as BindGroup;
	pub use super::internal::tile_read_internal::bind_groups::BindGroupLayout1 as BindGroupLayout;
	pub use super::internal::tile_read_internal::InstanceInput;
}
pub mod tile_write {
	pub use super::internal::tile_write_internal::bind_groups::BindGroup1 as BindGroup;
	pub use super::internal::tile_write_internal::bind_groups::BindGroupLayout1 as BindGroupLayout;
}
