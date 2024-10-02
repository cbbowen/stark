#[derive(Default, Debug, Clone, Copy, PartialEq)]
struct BezierPoint<Y> {
	t: f32,
	y: Y,
	dy_dt: Y,
	// d2y_dt2: Y,
}

trait VectorSpace:
	Sized
	+ Clone
	+ Copy
	+ std::ops::Add<Self, Output = Self>
	+ std::ops::Sub<Self, Output = Self>
	+ std::ops::Mul<f32, Output = Self>
	+ std::ops::Div<f32, Output = Self>
{
	fn lerp(self, other: Self, t: f32) -> Self {
		self * (1.0 - t) + other * t
	}
}
impl<Vector> VectorSpace for Vector where
	Vector: Sized
		+ Clone
		+ Copy
		+ std::ops::Add<Self, Output = Self>
		+ std::ops::Sub<Self, Output = Self>
		+ std::ops::Mul<f32, Output = Self>
		+ std::ops::Div<f32, Output = Self>
{
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Bezier<Y> {
	t0: f32,
	t1: f32,
	p: [Y; 4],
}

impl<Y: VectorSpace> Bezier<Y> {
	pub fn from_endpoints_and_tangents(p0: BezierPoint<Y>, p1: BezierPoint<Y>) -> Self {
		let w = (p1.t - p0.t) / 3.0;
		Self {
			t0: p0.t,
			t1: p1.t,
			p: [p0.y, p0.y + p0.dy_dt * w, p1.y - p1.dy_dt * w, p1.y],
		}
	}

	pub fn evaluate_end(&self) -> BezierPoint<Y> {
		self.evaluate(self.t1)
	}

	pub fn restricted(self, t0: f32, t1: f32) -> Self {
		let p0 = self.evaluate(t0);
		let p1 = self.evaluate(t1);
		Self::from_endpoints_and_tangents(p0, p1)
	}

	pub fn linear(t0: f32, y0: Y, t1: f32, y1: Y) -> Self {
		Self {
			t0,
			t1,
			p: [y0, y0.lerp(y1, 1.0 / 3.0), y0.lerp(y1, 2.0 / 3.0), y1],
		}
	}

	pub fn evaluate(&self, t: f32) -> BezierPoint<Y> {
		debug_assert!(t >= self.t0);
		debug_assert!(t <= self.t1);
		let w = (self.t1 - self.t0).recip();
		let s = (t - self.t0) * w;
		let q = [
			self.p[0].lerp(self.p[1], s),
			self.p[1].lerp(self.p[2], s),
			self.p[2].lerp(self.p[3], s),
		];
		let dq = [
			self.p[1] - self.p[0],
			self.p[2] - self.p[1],
			self.p[3] - self.p[2],
		];
		// let d2q = [dq[1] - dq[0], dq[2] - dq[1]];
		BezierPoint {
			t,
			y: q[0].lerp(q[1], s).lerp(q[1].lerp(q[2], s), s),
			dy_dt: dq[0].lerp(dq[1], s).lerp(dq[1].lerp(dq[2], s), s) * (3.0 * w),
			// d2y_dt2: 6.0 * w * w * d2q[0].lerp(d2q[1], s),
		}
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
		Some(Bezier::linear(t0, y0, t1, y1))
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
				.or_else(|| Some(Bezier::linear(t0, y0, t1, y1)))
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
				.or_else(|| Some(Bezier::linear(t0, y0, t1, y1)))
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
pub struct InputSplineBuilder<I> {
	interpolator: I,
	input_points: std::collections::VecDeque<InputPoint>,
	output_points: Vec<BezierPoint<glam::Vec3>>,
}

impl<I: Interpolator> InputSplineBuilder<I> {
	pub fn new(interpolator: I) -> Self {
		Self {
			interpolator,
			input_points: Default::default(),
			output_points: Vec::new(),
		}
	}

	fn x_points(&self) -> impl Iterator<Item = (f32, f32)> + '_ {
		self.input_points.iter().map(|p| (p.t, p.x))
	}

	fn y_points(&self) -> impl Iterator<Item = (f32, f32)> + '_ {
		self.input_points.iter().map(|p| (p.t, p.y))
	}

	fn z_points(&self) -> impl Iterator<Item = (f32, f32)> + '_ {
		self.input_points.iter().map(|p| (p.t, p.pressure))
	}

	pub fn add_point(&mut self, point: InputPoint) -> Option<Bezier<glam::Vec3>> {
		let last_point: Option<BezierPoint<glam::Vec3>> = self.output_points.last().cloned();
		const MIN_INTERPOLATION_INTERVAL: f32 = 0.125;
		if let Some(last_point) = last_point {
			if point.t < last_point.t + MIN_INTERPOLATION_INTERVAL {
				return None;
			}
		}

		const PRESSURE_RESOLUTION: f32 = 256.0;

		self.input_points.push_back(point);
		let x_bezier = self.interpolator.fit(
			last_point.map(|p| BezierPoint {
				t: p.t,
				y: p.y.x,
				dy_dt: p.dy_dt.x,
			}),
			self.x_points(),
		)?;
		let y_bezier = self.interpolator.fit(
			last_point.map(|p| BezierPoint {
				t: p.t,
				y: p.y.y,
				dy_dt: p.dy_dt.y,
			}),
			self.y_points(),
		)?;
		let z_bezier = self.interpolator.fit(
			last_point.map(|p| BezierPoint {
				t: p.t,
				y: PRESSURE_RESOLUTION * p.y.z,
				dy_dt: PRESSURE_RESOLUTION * p.dy_dt.z,
			}),
			self.z_points(),
		)?;

		let t0 = if let Some(last_point) = last_point {
			last_point.t
		} else {
			self.input_points.pop_front()?.t
		};
		let t1 = self.input_points.pop_front()?.t;

		let x_bezier = x_bezier.restricted(t0, t1);
		let y_bezier = y_bezier.restricted(t0, t1);
		let z_bezier = z_bezier.restricted(t0, t1);
		let bezier = Bezier {
			t0,
			t1,
			p: [
				glam::vec3(x_bezier.p[0], y_bezier.p[0], z_bezier.p[0] / PRESSURE_RESOLUTION),
				glam::vec3(x_bezier.p[1], y_bezier.p[1], z_bezier.p[1] / PRESSURE_RESOLUTION),
				glam::vec3(x_bezier.p[2], y_bezier.p[2], z_bezier.p[2] / PRESSURE_RESOLUTION),
				glam::vec3(x_bezier.p[3], y_bezier.p[3], z_bezier.p[3] / PRESSURE_RESOLUTION),
			],
		};
		self.output_points.push(bezier.evaluate_end());
		Some(bezier)
	}

	pub fn finish(self) -> Option<Bezier<glam::Vec3>> {
		// TODO: Implement this.
		None
	}
}

fn solve_qp<const N: usize>(
	p: &[[f64; N]; N],
	q: &[f64; N],
	a: &[[f64; N]],
	b: &[f64],
	cones: &[clarabel::solver::SupportedConeT<f64>],
) -> Option<Vec<f64>> {
	debug_assert_eq!(a.len(), b.len());
	use clarabel::algebra::*;
	use clarabel::solver::*;

	let p = CscMatrix::from(p);
	let a = CscMatrix::from(a);
	let settings = DefaultSettings {
		verbose: false,
		max_iter: 16,
		tol_gap_abs: EPSILON,
		tol_feas: EPSILON,
		tol_infeas_abs: EPSILON,
		presolve_enable: false,
		..Default::default()
	};

	let mut solver = DefaultSolver::new(&p, q, &a, b, cones, settings);
	solver.solve();
	match solver.solution.status {
		SolverStatus::Solved | SolverStatus::AlmostSolved => {}
		status @ (SolverStatus::PrimalInfeasible
		| SolverStatus::DualInfeasible
		| SolverStatus::AlmostPrimalInfeasible
		| SolverStatus::AlmostDualInfeasible) => {
			tracing::error!(?status, ?a, ?b);
			None?
		}
		status => {
			tracing::warn!(?status, ?p, ?q, ?a, ?b);
		}
	};

	solver
		.solution
		.x
		.iter()
		.all(|x| x.is_finite())
		.then_some(solver.solution.x)
}

pub struct BezierSolver {
	t0: f64,
	t1: f64,
	a: Vec<[f64; 4]>,
	b: Vec<f64>,
	cones: Vec<clarabel::solver::SupportedConeT<f64>>,
}

impl BezierSolver {
	pub fn new(t0: f32, t1: f32) -> Self {
		Self {
			t0: t0 as f64,
			t1: t1 as f64,
			a: Vec::new(),
			b: Vec::new(),
			cones: Vec::new(),
		}
	}

	fn constraint_coefficients(&self, t: f32) -> [f64; 4] {
		let t = t as f64;
		let s = (t - self.t0) / (self.t1 - self.t0);
		let r = 1.0 - s;
		let s2 = s * s;
		let r2 = r * r;
		[r2 * r, 3.0 * r2 * s, 3.0 * r * s2, s * s2]
	}

	fn derivative_constraint_coefficients(&self, t: f32) -> [f64; 4] {
		let t = t as f64;
		let w = (self.t1 - self.t0).recip();
		let s = (t - self.t0) * w;
		let r = 1.0 - s;
		let c0 = w * 3.0 * r * r;
		let c1 = w * 6.0 * r * s;
		let c2 = w * 3.0 * s * s;
		[-c0, c0 - c1, c1 - c2, c2]
	}

	fn constrain_linear_lt(&mut self, coefficients: [f64; 4], value: f64) {
		self.a.push(coefficients);
		self.b.push(value);
		self
			.cones
			.push(clarabel::solver::SupportedConeT::NonnegativeConeT(1));
	}

	fn constrain_linear_eq(&mut self, coefficients: [f64; 4], value: f64) {
		self.a.push(coefficients);
		self.b.push(value);
		self
			.cones
			.push(clarabel::solver::SupportedConeT::ZeroConeT(1));
	}

	pub fn constrain_lt(mut self, t: f32, y: f32) -> Self {
		self.constrain_linear_lt(self.constraint_coefficients(t), y as f64);
		self
	}

	pub fn constrain_gt(mut self, t: f32, y: f32) -> Self {
		self.constrain_linear_lt(self.constraint_coefficients(t).map(|c| -c), -y as f64);
		self
	}

	pub fn constrain_eq(mut self, t: f32, y: f32) -> Self {
		self.constrain_linear_eq(self.constraint_coefficients(t), y as f64);
		self
	}

	pub fn constrain_derivative_eq(mut self, t: f32, dy_dt: f32) -> Self {
		self.constrain_linear_eq(self.derivative_constraint_coefficients(t), dy_dt as f64);
		self
	}

	pub fn solve_smooth(self) -> Option<Bezier<f32>> {
		let p = [
			[2.0 + EPSILON, -3.0, 0.0, 1.0],
			[-3.0, 6.0 - EPSILON, -3.0, 0.0],
			[0.0, -3.0, 6.0 - EPSILON, -3.0],
			[1.0, 0.0, -3.0, 2.0 + EPSILON],
		];
		let q = [0.0, 0.0, 0.0, 0.0];
		let solution = solve_qp(&p, &q, &self.a, &self.b, &self.cones)?;
		Some(Bezier {
			t0: self.t0 as f32,
			t1: self.t1 as f32,
			p: [
				solution[0] as f32,
				solution[1] as f32,
				solution[2] as f32,
				solution[3] as f32,
			],
		})
	}
}

pub struct InitialBezierSolver {
	t0: f64,
	t1: f64,
	y0: f64,
	p0: f64,
	p1: f64,
	a: Vec<[f64; 2]>,
	b: Vec<f64>,
	cones: Vec<clarabel::solver::SupportedConeT<f64>>,
}

impl InitialBezierSolver {
	pub fn new(t0: f32, y0: f32, dy_dt0: f32, t1: f32) -> Self {
		// We could go ahead and set this to `y0`, but the problem is better conditioned if we offset
		// by `y0` at the very end.
		let p0 = 0.0;
		Self {
			t0: t0 as f64,
			t1: t1 as f64,
			y0: y0 as f64,
			p0,
			p1: p0 + (dy_dt0 * (t1 - t0) / 3.0) as f64,
			a: Vec::new(),
			b: Vec::new(),
			cones: Vec::new(),
		}
	}

	fn constraint_coefficients(&self, t: f32) -> ([f64; 2], f64) {
		let t = t as f64;
		let s = (t - self.t0) / (self.t1 - self.t0);
		let r = 1.0 - s;
		let s2 = s * s;
		let r2 = r * r;
		(
			[3.0 * r * s2, s * s2],
			self.y0 + r2 * (r * self.p0 + 3.0 * s * self.p1),
		)
	}

	fn constrain_linear_lt(&mut self, coefficients: [f64; 2], value: f64) {
		self.a.push(coefficients);
		self.b.push(value);
		self
			.cones
			.push(clarabel::solver::SupportedConeT::NonnegativeConeT(1));
	}

	pub fn constrain_lt(mut self, t: f32, y: f32) -> Self {
		let (coefficients, offset) = self.constraint_coefficients(t);
		self.constrain_linear_lt(coefficients, y as f64 - offset);
		self
	}

	pub fn constrain_gt(mut self, t: f32, y: f32) -> Self {
		let (coefficients, offset) = self.constraint_coefficients(t);
		self.constrain_linear_lt(coefficients.map(|c| -c), -(y as f64 - offset));
		self
	}

	pub fn solve_smooth(self) -> Option<Bezier<f32>> {
		let p = [[6.0, -3.0], [-3.0, 2.0]];
		let q = [-3.0 * self.p1, 1.0 * self.p0];
		// We could consider using a different method here because unlike the non-initial version,
		// this problem is strictly convex.
		let solution = solve_qp(&p, &q, &self.a, &self.b, &self.cones)?;
		Some(Bezier {
			t0: self.t0 as f32,
			t1: self.t1 as f32,
			p: [
				(self.y0 + self.p0) as f32,
				(self.y0 + self.p1) as f32,
				(self.y0 + solution[0]) as f32,
				(self.y0 + solution[1]) as f32,
			],
		})
	}
}

const EPSILON: f64 = 1e-2;

#[cfg(test)]
mod tests {
	use super::*;
	use approx::{abs_diff_eq, assert_abs_diff_eq};
	use std::assert_matches::assert_matches;

	const EPSILON: f32 = super::EPSILON as f32;

	#[test]
	fn test_cubic_segment_solver() {
		let cubic = BezierSolver::new(0.0, 4.0)
			.constrain_gt(1.0, 2.0)
			.constrain_lt(3.0, 1.0)
			.solve_smooth()
			.unwrap();
		println!("{cubic:?}");
		assert_eq!(cubic.t0, 0.0);
		assert_eq!(cubic.t1, 4.0);
		assert!(cubic.evaluate(1.0).y > 2.0 - EPSILON);
		assert!(cubic.evaluate(3.0).y < 1.0 + EPSILON);
	}

	#[test]
	fn test_initial_cubic_segment_solver() {
		let cubic = InitialBezierSolver::new(0.0, 1.0, 1.0, 4.0)
			.constrain_gt(1.0, 2.0)
			.constrain_lt(3.0, 1.0)
			.solve_smooth()
			.unwrap();
		println!("{cubic:?}");
		assert_eq!(cubic.t0, 0.0);
		assert_eq!(cubic.t1, 4.0);
		assert_eq!(cubic.evaluate(0.0).y, 1.0);
		assert_abs_diff_eq!(cubic.evaluate(0.0).dy_dt, 1.0, epsilon = EPSILON);
		assert!(cubic.evaluate(1.0).y > 2.0 - EPSILON);
		assert!(cubic.evaluate(3.0).y < 1.0 + EPSILON);
	}

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
		let mut spline: InputSplineBuilder<LinearInterpolator> = Default::default();

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
		assert_eq!(segment.t0, 0.0);
		assert_eq!(segment.t1, 1.0);

		let segment = spline
			.add_point(InputPoint {
				t: 2.0,
				x: 2.0,
				..Default::default()
			})
			.unwrap();
		assert_eq!(segment.t0, 1.0);
		assert_eq!(segment.t1, 2.0);

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
		let mut interpolator: InputSplineBuilder<CubicInterpolator> = Default::default();

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
		assert_eq!(segment.t0, 0.0);
		assert_eq!(segment.t1, 1.0);

		let segment = interpolator
			.add_point(InputPoint {
				t: 4.0,
				x: 0.0,
				..Default::default()
			})
			.unwrap();
		assert_eq!(segment.t0, 1.0);
		assert_eq!(segment.t1, 2.0);

		assert!(interpolator.finish().is_none());
	}

	#[test]
	fn test_cubic_window_interpolator_zig_zag() {
		let mut spline: InputSplineBuilder<CubicInterpolator> = Default::default();

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
				.dy_dt
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
				.dy_dt
				.x,
			0.5,
			epsilon = EPSILON.sqrt()
		);
	}
}
