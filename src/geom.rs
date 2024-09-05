use glam::Vec2;

pub struct AABox {
	min: Vec2,
	max: Vec2,
}

impl AABox {
	pub fn new(min: Vec2, max: Vec2) -> Self {
		Self { min, max }
	}

	pub fn empty() -> Self {
		Self::new(Vec2::MAX, Vec2::MIN)
	}

	pub fn is_empty(&self) -> bool {
		self.min.x > self.max.x && self.min.y > self.max.y
	}

	pub fn expanded_to_contain(self, point: Vec2) -> Self {
		Self::new(self.min.min(point), self.max.max(point))
	}

	pub fn containing(points: impl Iterator<Item = Vec2>) -> Self {
		points.fold(Self::empty(), |b, p| b.expanded_to_contain(p))
	}

	pub fn contains(&self, point: Vec2) -> bool {
		point.x < self.max.x
			&& point.y < self.max.y
			&& !(point.x < self.min.x)
			&& !(point.y < self.min.y)
	}

	pub fn corners(&self) -> [Vec2; 4] {
		[
			self.min,
			Vec2::new(self.min[0], self.max[1]),
			self.max,
			Vec2::new(self.max[0], self.min[1]),
		]
	}
}
