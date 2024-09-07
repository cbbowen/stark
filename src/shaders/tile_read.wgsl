// block
// Note that runtime-sized arrays must be `storage`.
// see https://google.github.io/tour-of-wgsl/types/arrays/runtime-sized-arrays/
@group(1) @binding(0)
var tile_texture: texture_2d_array<f32>;  // [layer_index]
@group(1) @binding(1)
var<storage> tile_data: array<TileData>;  // [layer_index]

// tile
struct InstanceInput {
	@location(0) layer_index: u32,
};
