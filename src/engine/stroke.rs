#[repr(C)]
// TODO: This needs to be serializable.k
#[derive(Debug, Clone)]
struct StrokePoint {
	position: [f32; 2],
	// TODO: We also need pressure, tilt, etc.
}

// TODO: This needs to be serializable.
#[derive(Debug, Clone)]
struct Stroke {
	points: Vec<StrokePoint>,
}

struct ActiveStroke {
	stroke: Stroke,
}

impl ActiveStroke {
	pub fn add_point(&mut self, point: StrokePoint) {
		self.stroke.points.push(point);
	}
}
