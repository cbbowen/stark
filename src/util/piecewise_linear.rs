use ordered_float::OrderedFloat;
use std::{fmt::Debug, ops::*};

pub trait Interpolable:
	Copy
	+ Default
	+ Add<Self, Output = Self>
	+ Sub<Self, Output = Self>
	+ Mul<f32, Output = Self>
	+ Div<f32, Output = Self>
{
}
impl<T> Interpolable for T where
	Self: Copy
		+ Default
		+ Add<Self, Output = Self>
		+ Sub<Self, Output = Self>
		+ Mul<f32, Output = Self>
		+ Div<f32, Output = Self>
{
}

#[derive(Clone, Debug)]
struct Point<Y> {
	x: OrderedFloat<f32>,
	y: Y,
}

impl<Y> Ord for Point<Y> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.x.cmp(&other.x)
	}
}

impl<Y> PartialOrd for Point<Y> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl<Y> PartialEq for Point<Y> {
	fn eq(&self, other: &Self) -> bool {
		self.cmp(other) == std::cmp::Ordering::Equal
	}
}

impl<Y> Eq for Point<Y> {}

impl<Y> From<(f32, Y)> for Point<Y> {
	fn from((x, y): (f32, Y)) -> Self {
		Self {
			x: OrderedFloat::from(x),
			y,
		}
	}
}

pub struct Linear<Y> {
	slope: Y,
	intercept: Y,
}

impl<Y: Interpolable> Linear<Y> {
	pub fn constant(y: Y) -> Self {
		Self {
			slope: Y::default(),
			intercept: y,
		}
	}

	pub fn fit(x0: f32, y0: Y, x1: f32, y1: Y) -> Self {
		let diff = x1 - x0;
		let slope = if diff > 0.0 {
			(y1 - y0) * (1.0 / diff)
		} else {
			Y::default()
		};
		let intercept = ((y0 + y1) - slope * (x0 + x1)) * 0.5f32;
		Self { slope, intercept }
	}

	pub fn evaluate(&self, x: f32) -> Y {
		self.slope * x + self.intercept
	}
}

pub struct LinearPiece<Y> {
	pub domain: Range<f32>,
	pub extension: Linear<Y>,
}

impl<Y: Interpolable> LinearPiece<Y> {
	fn new(prev: Option<&Point<Y>>, next: Option<&Point<Y>>) -> Option<Self> {
		if let Some(prev) = prev {
			if let Some(next) = next {
				Some(Self {
					domain: *prev.x..*next.x,
					extension: Linear::fit(*prev.x, prev.y, *next.x, next.y),
				})
			} else {
				Some(Self {
					domain: *prev.x..f32::INFINITY,
					extension: Linear::constant(prev.y),
				})
			}
		} else {
			let next = next?;
			Some(Self {
				domain: f32::NEG_INFINITY..*next.x,
				extension: Linear::constant(next.y),
			})
		}
	}

	pub fn evaluate(&self, x: f32) -> Y {
		self
			.extension
			.evaluate(x.clamp(self.domain.start, self.domain.end))
	}
}

pub struct PiecewiseLinear<Y> {
	points: Vec<Point<Y>>,
}

pub struct Iter<'a, Y> {
	prev: Option<&'a Point<Y>>,
	next: std::iter::Fuse<std::slice::Iter<'a, Point<Y>>>,
}

impl<'a, Y: Interpolable> IntoIterator for &'a PiecewiseLinear<Y> {
	type Item = LinearPiece<Y>;
	type IntoIter = Iter<'a, Y>;
	fn into_iter(self) -> Self::IntoIter {
		Iter {
			prev: None,
			next: self.points.iter().fuse(),
		}
	}
}

impl<'a, Y: Interpolable> Iterator for Iter<'a, Y> {
	type Item = LinearPiece<Y>;
	fn next(&mut self) -> Option<Self::Item> {
		let next = self.next.next();
		let prev = std::mem::replace(&mut self.prev, next);
		LinearPiece::new(prev, next)
	}
}

impl<Y: Interpolable> PiecewiseLinear<Y> {
	pub fn new(points: impl IntoIterator<Item = (f32, Y)>) -> Option<Self> {
		let mut points: Vec<Point<Y>> = points.into_iter().map(Point::from).collect();
		points.sort();
		if points.len() == 0 {
			None
		} else {
			Some(Self { points })
		}
	}

	pub fn piece_at(&self, x: f32) -> LinearPiece<Y> {
		let next_index = self.points.partition_point(|p| !(x < *p.x));
		let prev = if next_index == 0 {
			None
		} else {
			Some(&self.points[next_index - 1])
		};
		let next = self.points.get(next_index);
		LinearPiece::new(prev, next).unwrap()
	}

	pub fn evaluate(&self, x: f32) -> Y {
		self.piece_at(x).extension.evaluate(x)
	}

	pub fn linear_map<Z: Interpolable>(&self, f: impl Fn(&Y) -> Z) -> PiecewiseLinear<Z> {
		PiecewiseLinear {
			points: self
				.points
				.iter()
				.map(|p| Point { x: p.x, y: f(&p.y) })
				.collect(),
		}
	}

	// This essentially allows composing piecewise functions, but the interface is currently too tricky to expose.
	fn zip_flat_piece_map<Z: Interpolable, V, VIter: Iterator<Item=V>>(
		&self,
		other: &PiecewiseLinear<Z>,
		f: impl Fn(Range<f32>, &Linear<Y>, &Linear<Z>) -> VIter,
	) -> Vec<V> {
		let mut points = Vec::with_capacity(self.points.len() + other.points.len());
		let mut it_y = self.into_iter();
		let mut it_z = other.into_iter();
		let mut y_piece = it_y.next();
		let mut z_piece = it_z.next();
		loop {
			let Some(prev_y_piece) = &y_piece else { break };
			let Some(prev_z_piece) = &z_piece else { break };
			if prev_z_piece.domain.end < prev_y_piece.domain.end {
				z_piece = it_z.next();
				let Some(z_piece) = &z_piece else { continue };
				let domain = z_piece.domain.start..z_piece.domain.end.min(prev_y_piece.domain.end);
				points.extend(f(domain, &prev_y_piece.extension, &z_piece.extension));
			} else if prev_y_piece.domain.end < prev_z_piece.domain.end {
				y_piece = it_y.next();
				let Some(y_piece) = &y_piece else { continue };
				let domain = y_piece.domain.start..y_piece.domain.end.min(prev_z_piece.domain.end);
				points.extend(f(domain, &y_piece.extension, &prev_z_piece.extension));
			} else {
				// This case isnt' strictly necessary, but it avoids duplicate x-values.
				y_piece = it_y.next();
				z_piece = it_z.next();
				let Some(y_piece) = &y_piece else { continue };
				let Some(z_piece) = &z_piece else { continue };
				let domain = y_piece.domain.start.max(z_piece.domain.start)..y_piece.domain.end.min(z_piece.domain.end);
				points.extend(f(domain, &y_piece.extension, &z_piece.extension));
			}
		}
		points
	}

	fn zip_flat_piece_linear_map<Z: Interpolable, W: Interpolable, WPoints: Iterator<Item=(f32, W)>>(
		&self,
		other: &PiecewiseLinear<Z>,
		f: impl Fn(Range<f32>, &Linear<Y>, &Linear<Z>) -> WPoints,
	) -> PiecewiseLinear<W> {
		PiecewiseLinear { points: self.zip_flat_piece_map(other, move |d, y, z| f(d, y, z).map(Point::from)) }
	}

	pub fn bilinear_map<Z: Interpolable, W: Interpolable>(
		&self,
		other: &PiecewiseLinear<Z>,
		f: impl Fn(Y, Z) -> W,
	) -> PiecewiseLinear<W> {
		self.zip_flat_piece_linear_map(other, move |domain, y_linear, z_linear| {
			let x = domain.start;
			let y = y_linear.evaluate(x);
			let z = z_linear.evaluate(x);
			std::iter::once((x, f(y, z)))
		})
	}

	pub fn map_merged_inflection_points<Z: Interpolable, V>(
		&self,
		other: &PiecewiseLinear<Z>,
		f: impl Fn(f32, Y, Z) -> V,
	) -> Vec<V> {
		self.zip_flat_piece_map(
			other, move |domain, y_linear, z_linear| {
				let x = domain.start;
				let y = y_linear.evaluate(x);
				let z = z_linear.evaluate(x);
				std::iter::once(f(x, y, z))
			}
		)
	}
}

impl PiecewiseLinear<f32> {
	pub fn pointwise_max(
		&self,
		other: &Self,
	) -> Self {
		self.zip_flat_piece_linear_map(other, move |domain, y_linear, z_linear| {
			let x = domain.start;
			let y = y_linear.evaluate(x);
			let z = z_linear.evaluate(x);
			let result = std::iter::once((x, y.max(z)));

			let slope_diff = y_linear.slope - z_linear.slope;
			let intercept_difference = z_linear.intercept - y_linear.intercept;
			let x = intercept_difference / slope_diff;
			let intersection_point = if x > domain.start && x < domain.end {
				let y = y_linear.evaluate(x);
				let z = z_linear.evaluate(x);
				Some((x, y.max(z)))
			} else {
				None
			};

			result.chain(intersection_point)
		})
	}

	pub fn pointwise_min(
		&self,
		other: &Self,
	) -> Self {
		self.zip_flat_piece_linear_map(other, move |domain, y_linear, z_linear| {
			let x = domain.start;
			let y = y_linear.evaluate(x);
			let z = z_linear.evaluate(x);
			let result = std::iter::once((x, y.min(z)));

			let slope_diff = y_linear.slope - z_linear.slope;
			let intercept_difference = z_linear.intercept - y_linear.intercept;
			let x = intercept_difference / slope_diff;
			let intersection_point = if x > domain.start && x < domain.end {
				let y = y_linear.evaluate(x);
				let z = z_linear.evaluate(x);
				Some((x, y.min(z)))
			} else {
				None
			};

			result.chain(intersection_point)
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn basic_1() {
		let a = PiecewiseLinear::new([(0.0, 1.0)]).unwrap();
		assert_eq!(a.evaluate(-1.0), 1.0);
		assert_eq!(a.evaluate(0.0), 1.0);
		assert_eq!(a.evaluate(1.0), 1.0);
	}

	#[test]
	fn basic_2() {
		let a = PiecewiseLinear::new([(2.0, 2.0), (4.0, 6.0)]).unwrap();
		assert_eq!(a.evaluate(1.0), 2.0);
		assert_eq!(a.evaluate(2.0), 2.0);
		assert_eq!(a.evaluate(3.0), 4.0);
		assert_eq!(a.evaluate(4.0), 6.0);
		assert_eq!(a.evaluate(5.0), 6.0);
	}

	#[test]
	fn linear_map() {
		let a = PiecewiseLinear::new([(2.0, 2.0), (4.0, 6.0)]).unwrap();
		let b = a.linear_map(|y| y * 2.0 + 1.0);
		assert_eq!(b.evaluate(1.0), 5.0);
		assert_eq!(b.evaluate(2.0), 5.0);
		assert_eq!(b.evaluate(3.0), 9.0);
		assert_eq!(b.evaluate(4.0), 13.0);
		assert_eq!(b.evaluate(5.0), 13.0);
	}

	#[test]
	fn bilinear_map_1() {
		let a = PiecewiseLinear::new([(2.0, 2.0), (4.0, 6.0)]).unwrap();
		let b = PiecewiseLinear::new([(0.0, 1.0)]).unwrap();

		let c = a.bilinear_map(&b, |y, z| y + z);
		assert_eq!(c.evaluate(1.0), 3.0);
		assert_eq!(c.evaluate(2.0), 3.0);
		assert_eq!(c.evaluate(3.0), 5.0);
		assert_eq!(c.evaluate(4.0), 7.0);
		assert_eq!(c.evaluate(5.0), 7.0);

		let c = b.bilinear_map(&a, |z, y| y + z);
		assert_eq!(c.evaluate(1.0), 3.0);
		assert_eq!(c.evaluate(2.0), 3.0);
		assert_eq!(c.evaluate(3.0), 5.0);
		assert_eq!(c.evaluate(4.0), 7.0);
		assert_eq!(c.evaluate(5.0), 7.0);
	}

	#[test]
	fn bilinear_map_2() {
		let a = PiecewiseLinear::new([(2.0, 2.0), (4.0, 6.0)]).unwrap();
		let b = PiecewiseLinear::new([(1.0, 1.0), (3.0, 5.0)]).unwrap();

		let c = a.bilinear_map(&b, |y, z| y + 2.0 * z);
		assert_eq!(c.evaluate(0.0), 4.0);
		assert_eq!(c.evaluate(1.0), 4.0);
		assert_eq!(c.evaluate(2.0), 8.0);
		assert_eq!(c.evaluate(3.0), 14.0);
		assert_eq!(c.evaluate(4.0), 16.0);
		assert_eq!(c.evaluate(5.0), 16.0);

		let c = b.bilinear_map(&a, |z, y| y + 2.0 * z);
		assert_eq!(c.evaluate(0.0), 4.0);
		assert_eq!(c.evaluate(1.0), 4.0);
		assert_eq!(c.evaluate(2.0), 8.0);
		assert_eq!(c.evaluate(3.0), 14.0);
		assert_eq!(c.evaluate(4.0), 16.0);
		assert_eq!(c.evaluate(5.0), 16.0);
	}

	#[test]
	fn pointwise_max() {
		let a = PiecewiseLinear::new([(2.0, 2.0), (4.0, 6.0), (6.0, -1.0), (8.0, 5.0)]).unwrap();
		let b = PiecewiseLinear::new([(1.0, 2.0), (3.0, 6.0), (7.0, 5.0)]).unwrap();

		let epsilon = 0.0001;

		let c = a.pointwise_max(&b);
		for x in [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0] {
			assert!(c.evaluate(x) + epsilon > a.evaluate(x).max(b.evaluate(x)));
			assert!(c.evaluate(x) - epsilon < a.evaluate(x).max(b.evaluate(x)));
		}

		let c  = b.pointwise_max(&a);
		for x in [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0] {
			assert!(c.evaluate(x) + epsilon > a.evaluate(x).max(b.evaluate(x)));
			assert!(c.evaluate(x) - epsilon < a.evaluate(x).max(b.evaluate(x)));
		}
	}

	#[test]
	fn pointwise_min() {
		let a = PiecewiseLinear::new([(2.0, 2.0), (4.0, 6.0), (6.0, -1.0), (8.0, 5.0)]).unwrap();
		let b = PiecewiseLinear::new([(1.0, 2.0), (3.0, 6.0), (7.0, 5.0)]).unwrap();

		let epsilon = 0.0001;

		let c = a.pointwise_min(&b);
		for x in [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0] {
			assert!(c.evaluate(x) + epsilon > a.evaluate(x).min(b.evaluate(x)));
			assert!(c.evaluate(x) - epsilon < a.evaluate(x).min(b.evaluate(x)));
		}

		let c  = b.pointwise_min(&a);
		for x in [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0] {
			assert!(c.evaluate(x) + epsilon > a.evaluate(x).min(b.evaluate(x)));
			assert!(c.evaluate(x) - epsilon < a.evaluate(x).min(b.evaluate(x)));
		}
	}
}
