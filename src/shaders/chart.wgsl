// usage
@group(0) @binding(0)
var chart_sampler: sampler;
@group(0) @binding(1)
var<uniform> canvas_to_view: mat4x4<f32>;

struct TileData {
	chart_to_canvas: mat4x4<f32>,
};
include!("tile_read.wgsl") {}
