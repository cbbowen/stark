struct TileData {
	// This inconvenient to invert. Alternatively, we could have separate read and write data.
	// chart_to_canvas: mat4x4<f32>,
	chart_to_canvas_scale: vec2<f32>,
	chart_to_canvas_translation: vec2<f32>,
};
