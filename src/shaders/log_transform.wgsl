@group(0) @binding(0)
var source: texture_2d<f32>;

@group(0) @binding(1)
// Must be one of https://www.w3.org/TR/WGSL/#storage-texel-formats.
var destination: texture_storage_2d<r32float, write>;

const WORKGROUP_WIDTH: u32 = 16;
const WORKGROUP_HEIGHT: u32 = 16;

@compute
@workgroup_size(WORKGROUP_WIDTH, WORKGROUP_HEIGHT, 1)
fn log_transform(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
) {
	let texture_dimensions = textureDimensions(source);
	if gid.x >= texture_dimensions.x || gid.y >= texture_dimensions.y {
		return;
	}

	let input = textureLoad(source, gid.xy, 0).x;
	// Ideally, we would use `ln_1p(-input)` here.
	let output = -log(1 - clamp(input, 0.0, 1.0));
	textureStore(destination, gid.xy, vec4(output, 0, 0, 1));
}
