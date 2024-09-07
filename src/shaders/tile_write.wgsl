// block
// Note that runtime-sized arrays must be `storage`.
// see https://google.github.io/tour-of-wgsl/types/arrays/runtime-sized-arrays/
@group(1) @binding(0)
var<storage> tile_data: array<TileData>;  // [layer_index]
@group(1) @binding(1)
var<uniform> layer_index: u32;

// tile
struct InstanceInput {
	// It would be nice to use multiview to write to all the relevant layers in a block with one draw call. However, I don't see a way to (efficiently) write to only a subset of the layers.
	// @builtin(view_index) view_index: u32,
};
