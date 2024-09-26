@group(0) @binding(0)
var source: texture_2d_array<f32>;

@group(0) @binding(1)
// Must be one of https://www.w3.org/TR/WGSL/#storage-texel-formats.
var destination: texture_storage_3d<r32float, write>;

const WORKGROUP_WIDTH: u32 = 16;
const WORKGROUP_HEIGHT: u32 = 16;

@compute
@workgroup_size(WORKGROUP_WIDTH, WORKGROUP_HEIGHT, 1)
fn layers_to_depth(
    @builtin(global_invocation_id)
    v: vec3<u32>,
) {
	let texture_dimensions = textureDimensions(source);
	if v.x >= texture_dimensions.x || v.y >= texture_dimensions.y {
		return;
	}
	if v.z >= textureNumLayers(source) {
		return;
	}
	textureStore(destination, v, textureLoad(source, v.xy, v.z, 0));
}
