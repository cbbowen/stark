@group(0) @binding(0)
var source: texture_2d<f32>;

@group(0) @binding(1)
// Must be one of https://www.w3.org/TR/WGSL/#storage-texel-formats.
var destination: texture_storage_2d<r32float, write>;

const WORKGROUP_SIZE: u32 = 256;

@compute
@workgroup_size(WORKGROUP_SIZE, 1, 1)
fn horizontal_scan(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
) {
	let y = gid.x;
	let texture_dimensions = textureDimensions(source);
	if y >= texture_dimensions.y {
		return;
	}
	let scale = 1 / f32(texture_dimensions.x);
	var value: vec4<f32> = vec4(0, 0, 0, 0);
	for (var x: u32 = 0; x < texture_dimensions.x; x++) {
		let xy = vec2(x, y);
		value += textureLoad(source, xy, 0);
		textureStore(destination, xy, scale * value);
	}
}
