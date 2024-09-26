@group(0) @binding(0)
var source: texture_3d<f32>;

@group(0) @binding(1)
// Must be one of https://www.w3.org/TR/WGSL/#storage-texel-formats.
var destination: texture_storage_2d_array<r32float, write>;

const WORKGROUP_WIDTH: u32 = 16;
const WORKGROUP_HEIGHT: u32 = 16;

@compute
@workgroup_size(WORKGROUP_WIDTH, WORKGROUP_HEIGHT, 1)
fn depth_to_layers(
    @builtin(global_invocation_id)
    v: vec3<u32>,
) {
	let texture_dimensions = textureDimensions(source);
	if v.x >= texture_dimensions.x || v.y >= texture_dimensions.y || v.z >= texture_dimensions.z {
		return;
	}
	textureStore(destination, v.xy, v.z, textureLoad(source, v, 0));
}
