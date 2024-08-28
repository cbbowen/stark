use cgmath::{num_traits::Inv, prelude::*};
use std::ops::{Mul, Neg};

type Vec2<T> = cgmath::Vector2<T>;
pub type Vec2f = Vec2<f32>;
pub type Vec2i = Vec2<i32>;

fn perp<T: Neg<Output = T>>(v: Vec2<T>) -> Vec2<T> {
	Vec2::new(-v.y, v.x)
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Mat4x4fUniform([[f32; 4]; 4]);

trait Uniform: bytemuck::Pod + bytemuck::Zeroable {
	fn wgsl_type_definition() -> &'static str {
		""
	}
	fn wgsl_type_name() -> &'static str;
}

impl Uniform for Mat4x4fUniform {
	fn wgsl_type_name() -> &'static str {
		"mat4x4<f32>"
	}
}

/// Linear transformation that preserves orientations.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Scale2f(pub f32);

impl Default for Scale2f {
	fn default() -> Self {
		Self::new(1.0)
	}
}

impl Scale2f {
	pub fn new(factor: f32) -> Self {
		Self(factor)
	}

	pub fn factor(self) -> f32 {
		self.0
	}

	pub fn transform(self, p: Vec2f) -> Vec2f {
		self.factor() * p
	}

	pub fn inverse(self) -> Self {
		Self::new(self.factor().inv())
	}
}

/// Linear transformation that preserves angles.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Ortho2f(Vec2f);

impl From<Scale2f> for Ortho2f {
	fn from(value: Scale2f) -> Self {
		Self(Vec2f::new(value.factor(), 0.0))
	}
}

impl Ortho2f {
	pub fn applied_to_unit_x(self) -> Vec2f {
		self.0
	}

	pub fn from_radians(radians: f32) -> Self {
		let (sin, cos) = radians.sin_cos();
		Self(Vec2f::new(cos, sin))
	}

	pub fn from_radians_and_scale(radians: f32, scale: f32) -> Self {
		let (sin, cos) = radians.sin_cos();
		Self(scale * Vec2f::new(cos, sin))
	}

	pub fn transform(self, p: Vec2f) -> Vec2f {
		let Vec2 { x, y } = self.applied_to_unit_x();
		x * p + y * perp(p)
	}

	pub fn inverse(self) -> Self {
		let applied_to_unit_x = self.applied_to_unit_x();
		Self(Vec2::new(applied_to_unit_x.x, -applied_to_unit_x.y) / applied_to_unit_x.magnitude2())
	}
}

impl Default for Ortho2f {
	fn default() -> Self {
		Self(Vec2f::new(1.0, 0.0))
	}
}

impl Mul for Ortho2f {
	type Output = Self;
	fn mul(self, other: Self) -> Self {
		Self(self.transform(other.applied_to_unit_x()))
	}
}

/// Affine transformation that preserves distances and orientations.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Trans2f(pub Vec2f);

impl Default for Trans2f {
	fn default() -> Self {
		Trans2f(Vec2::zero())
	}
}

impl Trans2f {
	pub fn new(x: f32, y: f32) -> Self {
		Self(Vec2::new(x, y))
	}

	pub fn offset(self) -> Vec2<f32> {
		self.0
	}

	pub fn transform(self, p: Vec2f) -> Vec2f {
		p + self.offset()
	}

	pub fn inverse(self) -> Self {
		Self(-self.offset())
	}
}

/// Affine transformation that preserves angles.
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct Similar2f {
	pub orthogonal: Ortho2f,
	pub translation: Trans2f,
}

impl From<Trans2f> for Similar2f {
	fn from(value: Trans2f) -> Self {
		Self {
			translation: value,
			..Default::default()
		}
	}
}

impl From<Ortho2f> for Similar2f {
	fn from(value: Ortho2f) -> Self {
		Self {
			orthogonal: value,
			..Default::default()
		}
	}
}

impl Similar2f {
	pub fn new(orthogonal: impl Into<Ortho2f>, translation: impl Into<Trans2f>) -> Self {
		Similar2f {
			orthogonal: orthogonal.into(),
			translation: translation.into(),
		}
	}

	pub fn transform(self, p: Vec2f) -> Vec2f {
		self.translation.transform(self.orthogonal.transform(p))
	}

	pub fn inverse(self) -> Self {
		let orthogonal = self.orthogonal.inverse();
		let translation = Trans2f(orthogonal.transform(self.translation.inverse().offset()));
		Self {
			orthogonal,
			translation,
		}
	}

	pub fn to_mat4x4_uniform(self) -> Mat4x4fUniform {
		let Self {
			orthogonal: Ortho2f(ortho_x),
			translation: Trans2f(offset),
		} = self;
		Mat4x4fUniform([
			[ortho_x.x, -ortho_x.y, 0.0, offset.x],
			[ortho_x.y, ortho_x.x, 0.0, offset.y],
			[0.0, 0.0, 1.0, 0.0],
			[0.0, 0.0, 0.0, 1.0],
		])
	}
}

impl Mul for Similar2f {
	type Output = Self;
	fn mul(self, other: Self) -> Self {
		Self {
			orthogonal: self.orthogonal * other.orthogonal,
			translation: Trans2f(self.transform(other.translation.offset())),
		}
	}
}

pub struct AABox<T: Bounded + Copy + Ord> {
	min: Vec2<T>,
	max: Vec2<T>,
}

impl<T: Bounded + Copy + Ord> AABox<T> {
	pub fn new(min: Vec2<T>, max: Vec2<T>) -> Self {
		Self { min, max }
	}

	pub fn empty() -> Self {
		Self::new(Vec2::max_value(), Vec2::min_value())
	}

	pub fn is_empty(&self) -> bool {
		self.min.x > self.max.x && self.min.y > self.max.y
	}

	pub fn expanded_to_contain(self, point: Vec2<T>) -> Self {
		Self::new(self.min.zip(point, &T::min), self.max.zip(point, &T::max))
	}

	pub fn containing(points: impl Iterator<Item = Vec2<T>>) -> Self {
		points.fold(Self::empty(), |b, p| b.expanded_to_contain(p))
	}

	pub fn contains(&self, point: Vec2<T>) -> bool {
		point.x < self.max.x
			&& point.y < self.max.y
			&& !(point.x < self.min.x)
			&& !(point.y < self.min.y)
	}

	pub fn corners(&self) -> [Vec2<T>; 4] {
		[
			self.min,
			Vec2::new(self.min[0], self.max[1]),
			self.max,
			Vec2::new(self.max[0], self.min[1]),
		]
	}
}
