include!("tile_data.wgsl") {}

// block
// Note that runtime-sized arrays must be `storage`.
// see https://google.github.io/tour-of-wgsl/types/arrays/runtime-sized-arrays/

// Indexed by `layer_index`.
// Sample with `textureSample(tile_texture, sampler, coords, layer_index)`.
@group(1) @binding(0)
var tile_texture: texture_2d_array<f32>;

// Indexed by `layer_index`.
// Access with `tile_data[layer_index]`.
@group(1) @binding(1)
var<storage> tile_data: array<TileData>;

// tile
struct InstanceInput {
	@location(0) layer_index: u32,
};
