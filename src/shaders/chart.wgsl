@group(0) @binding(0)
var<uniform> chart_to_canvas: mat4x4<f32>;
@group(0) @binding(1)
var chart_texture: texture_2d<f32>;
@group(0) @binding(2)
var chart_sampler: sampler;

// There's no actual logic in this file. It's purpose is to specify the layout of the shared binding group.
// TODO: Ideally, we would have a way to automatically insert this in the WGSL files that need it. A purely textual solution isn't great, though because these will change very frequently and so should generally be the last binding group, not the first.
