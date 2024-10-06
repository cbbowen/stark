use super::bezier::*;
use super::VectorSpace;
use itertools::Itertools as _;

#[derive(Default, Debug, Clone, PartialEq)]
pub struct TemporalCurve<Position, Params> {
	points: Vec<TemporalCurvePoint<Position, Params>>,
}

impl<Position: VectorSpace, Params: VectorSpace> TemporalCurve<Position, Params> {
	pub fn new(points: Vec<TemporalCurvePoint<Position, Params>>) -> Option<Self> {
		(!points.is_empty()).then_some(TemporalCurve { points })
	}

	pub fn segments(&self) -> impl Iterator<Item = TemporalCurveSegment<Position, Params>> + '_ {
		self
			.points
			.iter()
			.tuple_windows()
			.map(|(p0, p1)| TemporalCurveSegment::interpolate(*p0, *p1))
	}

	fn segment(&self, i: usize) -> Option<TemporalCurveSegment<Position, Params>> {
		let next_point = self.points.get(i + 1)?;
		let prev_point = &self.points[i];
		Some(TemporalCurveSegment::interpolate(*prev_point, *next_point))
	}

	pub fn find_segment_containing(&self, t: f32) -> TemporalCurveSegment<Position, Params> {
		let i = self.points.partition_point(|p| !(p.t() > t));
		if i == 0 {
			TemporalCurveSegment::extrapolate(*self.points.first().unwrap())
		} else {
			self
				.segment(i - 1)
				.unwrap_or_else(|| TemporalCurveSegment::extrapolate(*self.points.last().unwrap()))
		}
	}

	pub fn evaluate(&self, t: f32) -> TemporalCurvePoint<Position, Params> {
		self.find_segment_containing(t).evaluate(t)
	}
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct TemporalCurvePoint<Position, Params> {
	position: BezierPoint<Position>,
	params: Params,
}

impl<Position: VectorSpace, Params: VectorSpace> TemporalCurvePoint<Position, Params> {
	pub fn t(&self) -> f32 {
		self.position.t
	}

	pub fn position(&self) -> Position {
		self.position.y
	}

	pub fn velocity(&self) -> Position {
		self.position.dy_dt
	}

	pub fn params(&self) -> Params {
		self.params
	}
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct TemporalCurveSegment<Position, Params> {
	position_bezier: Bezier<Position>,
	params0: Params,
	params1: Params,
}

impl<Position: VectorSpace, Params: VectorSpace> TemporalCurveSegment<Position, Params> {
	pub fn interpolate(p0: TemporalCurvePoint<Position, Params>, p1: TemporalCurvePoint<Position, Params>) -> Self {
		TemporalCurveSegment {
			position_bezier: Bezier::from_endpoints_and_tangents(p0.position, p1.position),
			params0: p0.params,
			params1: p1.params,
		}
	}

	pub fn extrapolate(p: TemporalCurvePoint<Position, Params>) -> Self {
		TemporalCurveSegment {
			position_bezier: Bezier::linear_at(p.position),
			params0: p.params,
			params1: p.params,
		}
	}

	pub fn t0(&self) -> f32 {
		self.position_bezier.t0
	}

	pub fn t1(&self) -> f32 {
		self.position_bezier.t1
	}

	pub fn lerp_factor(&self, t: f32) -> f32 {
		self.position_bezier.lerp_factor(t)
	}

	pub fn evaluate(&self, t: f32) -> TemporalCurvePoint<Position, Params> {
		TemporalCurvePoint {
			position: self.position_bezier.evaluate(t),
			params: self.params0.lerp(self.params1, self.lerp_factor(t)),
		}
	}

	pub fn evaluate_end(&self) -> TemporalCurvePoint<Position, Params> {
		self.evaluate(self.t1())
	}
}

pub struct SpatialCurve {
	temporal: TemporalCurve<glam::Vec2, f32>,
	distances: Vec<f32>,
}

// Returns `x` such that the integral of `dy_dx` from 0 to `x` equals `y`.
fn solve_positive_integral(dy_dx: impl Fn(f64) -> f64, mut y: f64, x_0: f64) -> f64 {
	const EPS: f64 = 1e-5;
	let mut a = 0.0;
	let mut x_i = x_0;
	for i in 0..50 {
		let dy_dx_i = dy_dx(x_i) + (-i as f64).exp2();
		let y_i = quadrature::integrate(&dy_dx, a, x_i, EPS);
		let delta_y = y_i.integral - y;

		// This is an optimization, not necessary for correctness.
		if delta_y < 0.0 {
			y = -delta_y;
			a = x_i;
		}

		x_i -= delta_y / (dy_dx_i + y_i.error_estimate);
		if delta_y.abs() + y_i.error_estimate <= EPS {
			break;
		}
	}
	x_i
}

impl SpatialCurve {
	pub fn new(temporal: TemporalCurve<glam::Vec2, f32>) -> Self {
		let eps = 1e-6;
		let lengths = temporal.segments().map(|s| {
			quadrature::integrate(
				|t| s.evaluate(t as f32).velocity().length() as f64,
				s.t0() as f64,
				s.t1() as f64,
				eps,
			)
			.integral as f32
		});
		let distances = lengths
			.scan(0.0, |d, l| {
				*d += l;
				Some(*d)
			})
			.collect();
		Self {
			temporal,
			distances,
		}
	}

	pub fn evaluate(&self, s: f32) -> Option<TemporalCurvePoint<glam::Vec2, f32>> {
		if s < 0.0 || s > *self.distances.last().unwrap() {
			return None;
		}
		let i = self.distances.partition_point(move |&s_i| !(s_i > s));
		let segment = self.temporal.segment(i)?;
		let prev_distance = if i == 0 { 0.0 } else { self.distances[i - 1] };
		let next_distance = self.distances[i];
		let t = (segment.t0()
			+ (segment.t1() - segment.t0()) * (0.5 + s - prev_distance)
				/ (1.0 + next_distance - prev_distance))
			.clamp(segment.t0(), segment.t1());
		let t = solve_positive_integral(
			|t| segment.evaluate(t as f32).velocity().length() as f64,
			s as f64,
			t as f64,
		);

		Some(segment.evaluate(t as f32))
	}
}

pub trait Interpolator {
	fn fit(
		&self,
		initial: Option<BezierPoint<f32>>,
		points: impl IntoIterator<Item = (f32, f32)>,
	) -> Option<Bezier<f32>>;
}

#[derive(Default, Debug, Clone, Copy)]
pub struct LinearInterpolator;

impl Interpolator for LinearInterpolator {
	fn fit(
		&self,
		initial: Option<BezierPoint<f32>>,
		points: impl IntoIterator<Item = (f32, f32)>,
	) -> Option<Bezier<f32>> {
		let mut points = points.into_iter();
		let (t0, y0) = if let Some(initial) = initial {
			(initial.t, initial.y)
		} else {
			points.next()?
		};
		let (t1, y1) = points.next()?;
		Some(Bezier::linear_between(t0, y0, t1, y1))
	}
}

#[derive(Default, Debug, Clone, Copy)]
pub struct CubicInterpolator;

impl Interpolator for CubicInterpolator {
	fn fit(
		&self,
		initial: Option<BezierPoint<f32>>,
		points: impl IntoIterator<Item = (f32, f32)>,
	) -> Option<Bezier<f32>> {
		let mut points = points.into_iter();
		if let Some(initial) = initial {
			let t0 = initial.t;
			let y0 = initial.y;
			let (t1, y1) = points.next()?;
			let (t2, y2) = points.next()?;
			InitialBezierSolver::new(t0, y0, initial.dy_dt, t2)
				.constrain_lt(t1, y1 + 0.5)
				.constrain_gt(t1, y1 - 0.5)
				.constrain_lt(t2, y2 + 0.5)
				.constrain_gt(t2, y2 - 0.5)
				.solve_smooth()
				.or_else(|| Some(Bezier::linear_between(t0, y0, t1, y1)))
		} else {
			let (t0, y0) = points.next()?;
			let (t1, y1) = points.next()?;
			let (t2, y2) = points.next()?;
			let (t3, y3) = points.next()?;
			BezierSolver::new(t0, t3)
				.constrain_lt(t0, y0 + 0.5)
				.constrain_gt(t0, y0 - 0.5)
				.constrain_lt(t1, y1 + 0.5)
				.constrain_gt(t1, y1 - 0.5)
				.constrain_lt(t2, y2 + 0.5)
				.constrain_gt(t2, y2 - 0.5)
				.constrain_lt(t3, y3 + 0.5)
				.constrain_gt(t3, y3 - 0.5)
				.solve_smooth()
				.or_else(|| Some(Bezier::linear_between(t0, y0, t1, y1)))
		}
	}
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct InputPoint {
	pub t: f32,
	pub x: f32,
	pub y: f32,
	pub pressure: f32,
}

#[derive(Default, Debug, Clone)]
pub struct InputDifferentiator<I> {
	interpolator: I,
	input_points: std::collections::VecDeque<InputPoint>,
	last_point: Option<TemporalCurvePoint<glam::Vec2, f32>>,
}

impl<I: Interpolator> InputDifferentiator<I> {
	pub fn new(interpolator: I) -> Self {
		Self {
			interpolator,
			input_points: Default::default(),
			last_point: Default::default(),
		}
	}

	fn x_points(&self) -> impl Iterator<Item = (f32, f32)> + '_ {
		self.input_points.iter().map(|p| (p.t, p.x))
	}

	fn y_points(&self) -> impl Iterator<Item = (f32, f32)> + '_ {
		self.input_points.iter().map(|p| (p.t, p.y))
	}

	pub fn add_point(&mut self, point: InputPoint) -> Option<TemporalCurvePoint<glam::Vec2, f32>> {
		let last_point = self.last_point.clone();
		const MIN_INTERPOLATION_INTERVAL: f32 = 0.125;
		if let Some(last_point) = last_point {
			if point.t < last_point.t() + MIN_INTERPOLATION_INTERVAL {
				return None;
			}
		}

		self.input_points.push_back(point);
		let x_bezier = self.interpolator.fit(
			last_point.map(|p| BezierPoint {
				t: p.t(),
				y: p.position().y,
				dy_dt: p.velocity().x,
			}),
			self.x_points(),
		)?;
		let y_bezier = self.interpolator.fit(
			last_point.map(|p| BezierPoint {
				t: p.t(),
				y: p.position().y,
				dy_dt: p.velocity().y,
			}),
			self.y_points(),
		)?;

		let (_t0, _params0) = if let Some(last_point) = last_point {
			(last_point.t(), last_point.params)
		} else {
			let p = self.input_points.pop_front()?;
			(p.t, p.pressure)
		};
		let InputPoint {
			t: t1,
			pressure: params1,
			..
		} = self.input_points.pop_front()?;

		let x1 = x_bezier.evaluate(t1);
		let y1 = y_bezier.evaluate(t1);
		let output_point = TemporalCurvePoint {
			position: BezierPoint {
				t: t1,
				y: glam::vec2(x1.y, y1.y),
				dy_dt: glam::vec2(x1.dy_dt, y1.dy_dt),
			},
			params: params1
		};
		self.last_point = Some(output_point);
		Some(output_point)

		// let x_bezier = x_bezier.restricted(t0, t1);
		// let y_bezier = y_bezier.restricted(t0, t1);
		// let position_bezier = Bezier {
		// 	t0,
		// 	t1,
		// 	p: [
		// 		glam::vec2(x_bezier.p[0], y_bezier.p[0]),
		// 		glam::vec2(x_bezier.p[1], y_bezier.p[1]),
		// 		glam::vec2(x_bezier.p[2], y_bezier.p[2]),
		// 		glam::vec2(x_bezier.p[3], y_bezier.p[3]),
		// 	],
		// };
		// let curve = TemporalCurveSegment {
		// 	position_bezier,
		// 	params0,
		// 	params1,
		// };
		// self.last_point = Some(curve.evaluate_end());
		// Some(curve)
	}

	pub fn finish(self) -> Option<TemporalCurvePoint<glam::Vec2, f32>> {
		// TODO: Implement this.
		None
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use approx::{abs_diff_eq, assert_abs_diff_eq};
	use std::assert_matches::assert_matches;

	const EPSILON: f32 = 1e-2;

	#[test]
	fn test_linear_interpolator() {
		let interpolator = LinearInterpolator;
		assert!(interpolator.fit(None, [(0.0, 0.0)]).is_none());

		let cubic = interpolator.fit(None, [(0.0, 0.0), (1.0, 1.0)]).unwrap();
		assert!(matches!(
			cubic.evaluate(0.0),
			BezierPoint {
				t: 0.0,
				y: 0.0,
				dy_dt,
			} if abs_diff_eq!(dy_dt, 1.0, epsilon = EPSILON)
		));
		assert!(matches!(
			cubic.evaluate(1.0),
			BezierPoint {
				t: 1.0,
				y: 1.0,
				dy_dt,
			}  if abs_diff_eq!(dy_dt, 1.0, epsilon = EPSILON)
		));

		let cubic = interpolator
			.fit(
				Some(BezierPoint {
					t: 0.0,
					y: 0.0,
					dy_dt: 0.0,
				}),
				[(1.0, 1.0)],
			)
			.unwrap();
		assert_matches!(
			cubic.evaluate(0.0),
			BezierPoint {
				t: 0.0,
				y: 0.0,
				dy_dt,
			}  if abs_diff_eq!(dy_dt, 1.0, epsilon = EPSILON)
		);
		assert_matches!(
			cubic.evaluate(1.0),
			BezierPoint {
				t: 1.0,
				y: 1.0,
				dy_dt,
			}  if abs_diff_eq!(dy_dt, 1.0, epsilon = EPSILON)

		);
	}

	#[test]
	fn test_linear_window_interpolator() {
		let mut spline: InputDifferentiator<LinearInterpolator> = Default::default();

		assert!(spline
			.add_point(InputPoint {
				t: 0.0,
				x: 0.0,
				..Default::default()
			})
			.is_none());

		let segment = spline
			.add_point(InputPoint {
				t: 1.0,
				x: 1.0,
				..Default::default()
			})
			.unwrap();
		assert_eq!(segment.t0(), 0.0);
		assert_eq!(segment.t1(), 1.0);

		let segment = spline
			.add_point(InputPoint {
				t: 2.0,
				x: 2.0,
				..Default::default()
			})
			.unwrap();
		assert_eq!(segment.t0(), 1.0);
		assert_eq!(segment.t1(), 2.0);

		assert!(spline.finish().is_none());
	}

	#[test]
	fn test_cubic_interpolator() {
		let interpolator = CubicInterpolator;
		assert!(interpolator.fit(None, [(0.0, 0.0)]).is_none());

		let cubic = interpolator
			.fit(None, [(0.0, 0.0), (1.0, 1.5), (2.0, 1.5), (3.0, 0.5)])
			.unwrap();
		assert_matches!(
			cubic.evaluate(0.0),
			BezierPoint { t: 0.0, y, .. } if abs_diff_eq!(y, 0.5, epsilon = 2.0 * EPSILON)
		);
		assert_matches!(
			cubic.evaluate(1.0),
			BezierPoint {
				t: 1.0,
				y,
				..
			} if abs_diff_eq!(y, 1.0, epsilon = 2.0 * EPSILON));

		let cubic = interpolator
			.fit(
				Some(BezierPoint {
					t: 0.0,
					y: 0.0,
					dy_dt: 1.0,
				}),
				[(1.0, 0.0), (2.0, -1.0)],
			)
			.unwrap();
		assert_matches!(
			cubic.evaluate(0.0),
			BezierPoint {
				t: 0.0,
				y,
				dy_dt,
				..
			} if abs_diff_eq!(y, 0.0, epsilon = 2.0 * EPSILON) && abs_diff_eq!(dy_dt, 1.0, epsilon = 2.0 * EPSILON)
		);
	}

	#[test]
	fn test_cubic_window_interpolator() {
		let mut interpolator: InputDifferentiator<CubicInterpolator> = Default::default();

		assert!(interpolator
			.add_point(InputPoint {
				t: 0.0,
				x: 0.0,
				..Default::default()
			})
			.is_none());
		assert!(interpolator
			.add_point(InputPoint {
				t: 1.0,
				x: 1.0,
				..Default::default()
			})
			.is_none());
		assert!(interpolator
			.add_point(InputPoint {
				t: 2.0,
				x: 2.0,
				..Default::default()
			})
			.is_none());

		let segment = interpolator
			.add_point(InputPoint {
				t: 3.0,
				x: 1.0,
				..Default::default()
			})
			.unwrap();
		assert_eq!(segment.t0(), 0.0);
		assert_eq!(segment.t1(), 1.0);

		let segment = interpolator
			.add_point(InputPoint {
				t: 4.0,
				x: 0.0,
				..Default::default()
			})
			.unwrap();
		assert_eq!(segment.t0(), 1.0);
		assert_eq!(segment.t1(), 2.0);

		assert!(interpolator.finish().is_none());
	}

	#[test]
	fn test_cubic_window_interpolator_zig_zag() {
		let mut spline: InputDifferentiator<CubicInterpolator> = Default::default();

		assert!(spline
			.add_point(InputPoint {
				t: 0.0,
				x: 0.0,
				..Default::default()
			})
			.is_none());
		assert!(spline
			.add_point(InputPoint {
				t: 1.0,
				x: 1.0,
				..Default::default()
			})
			.is_none());
		assert!(spline
			.add_point(InputPoint {
				t: 2.0,
				x: 1.0,
				..Default::default()
			})
			.is_none());
		assert!(spline
			.add_point(InputPoint {
				t: 3.0,
				x: 2.0,
				..Default::default()
			})
			.is_some());
		assert!(spline
			.add_point(InputPoint {
				t: 4.0,
				x: 2.0,
				..Default::default()
			})
			.is_some());
		assert!(spline
			.add_point(InputPoint {
				t: 5.0,
				x: 3.0,
				..Default::default()
			})
			.is_some());
		assert!(spline
			.add_point(InputPoint {
				t: 6.0,
				x: 3.0,
				..Default::default()
			})
			.is_some());
		assert!(spline
			.add_point(InputPoint {
				t: 7.0,
				x: 4.0,
				..Default::default()
			})
			.is_some());
		assert!(spline
			.add_point(InputPoint {
				t: 8.0,
				x: 4.0,
				..Default::default()
			})
			.is_some());
		assert_abs_diff_eq!(
			spline
				.add_point(InputPoint {
					t: 9.0,
					x: 5.0,
					..Default::default()
				})
				.unwrap()
				.evaluate_end()
				.velocity()
				.x,
			0.5,
			epsilon = EPSILON.sqrt()
		);
		assert_abs_diff_eq!(
			spline
				.add_point(InputPoint {
					t: 10.0,
					x: 5.0,
					..Default::default()
				})
				.unwrap()
				.evaluate_end()
				.velocity()
				.x,
			0.5,
			epsilon = EPSILON.sqrt()
		);
	}

	#[test]
	fn test_temporal_curve() {
		let dy_dt = glam::vec2(1.0, 2.0);
		let temporal = TemporalCurve::new(vec![
			TemporalCurvePoint {
				position: BezierPoint {
					t: 0.0,
					y: 0.0 * dy_dt,
					dy_dt,
				},
				params: 0.0,
			},
			TemporalCurvePoint {
				position: BezierPoint {
					t: 1.0,
					y: 1.0 * dy_dt,
					dy_dt,
				},
				params: 0.0,
			},
		])
		.unwrap();

		let evaluated = temporal.evaluate(0.5);
		assert_abs_diff_eq!(evaluated.position().x, 0.5 * dy_dt.x);
		assert_abs_diff_eq!(evaluated.position().y, 0.5 * dy_dt.y);
	}

	#[test]
	fn test_spatial_curve() {
		let dy_dt = glam::vec2(1.0, 2.0);
		let temporal = TemporalCurve::new(vec![
			TemporalCurvePoint {
				position: BezierPoint {
					t: 0.0,
					y: 0.0 * dy_dt,
					dy_dt,
				},
				params: 0.0,
			},
			TemporalCurvePoint {
				position: BezierPoint {
					t: 1.0,
					y: 1.0 * dy_dt,
					dy_dt,
				},
				params: 0.0,
			},
		])
		.unwrap();
		let spatial = SpatialCurve::new(temporal);

		let evaluated = spatial.evaluate(0.5).unwrap();
		assert_abs_diff_eq!(evaluated.position().x, 0.5 * dy_dt.normalize().x);
		assert_abs_diff_eq!(evaluated.position().y, 0.5 * dy_dt.normalize().y);
	}
}
