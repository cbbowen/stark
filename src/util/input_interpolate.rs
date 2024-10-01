use glam::FloatExt;

use crate::util::ResultExt;

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct EvaluatedPoint {
	t: f32,
	y: f32,
	dy_dt: f32,
	d2y_dt2: f32,
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct CubicSegment {
	t0: f32,
	t1: f32,
	p: [f32; 4],
}

impl CubicSegment {
	pub fn evaluate(&self, t: f32) -> EvaluatedPoint {
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
		let d2q = [dq[1] - dq[0], dq[2] - dq[1]];
		EvaluatedPoint {
			t,
			y: q[0].lerp(q[1], s).lerp(q[1].lerp(q[2], s), s),
			dy_dt: 3.0 * w * dq[0].lerp(dq[1], s).lerp(dq[1].lerp(dq[2], s), s),
			d2y_dt2: 6.0 * w * w * d2q[0].lerp(d2q[1], s),
		}
	}

	pub fn evaluate_first(&self) -> EvaluatedPoint {
		self.evaluate(self.t0)
	}

	pub fn evaluate_last(&self) -> EvaluatedPoint {
		self.evaluate(self.t1)
	}

	pub fn restricted(self, t0: f32, t1: f32) -> Self {
		let p0 = self.evaluate(t0);
		let p1 = self.evaluate(t1);
		let w = (t1 - t0) / 3.0;
		Self {
			t0,
			t1,
			p: [p0.y, p0.y + w * p0.dy_dt, p1.y - w * p1.dy_dt, p1.y],
		}
	}

	pub fn linear(t0: f32, y0: f32, t1: f32, y1: f32) -> Self {
		Self {
			t0,
			t1,
			p: [y0, y0.lerp(y1, 1.0 / 3.0), y0.lerp(y1, 2.0 / 3.0), y1],
		}
	}
}

pub trait FixedLengthInterpolator {
	fn fit(
		&self,
		initial: Option<EvaluatedPoint>,
		points: impl IntoIterator<Item = (f32, f32)>,
	) -> Option<CubicSegment>;
}

const MIN_INTERPOLATION_INTERVAL: f32 = 0.125;

#[derive(Default, Debug, Clone, Copy)]
pub struct LinearInterpolator;

impl FixedLengthInterpolator for LinearInterpolator {
	fn fit(
		&self,
		initial: Option<EvaluatedPoint>,
		points: impl IntoIterator<Item = (f32, f32)>,
	) -> Option<CubicSegment> {
		let mut points = points.into_iter();
		let (t0, y0) = if let Some(initial) = initial {
			(initial.t, initial.y)
		} else {
			points.next()?
		};
		let (t1, y1) = points.find(|&(t, _)| t > t0 + MIN_INTERPOLATION_INTERVAL)?;
		Some(CubicSegment::linear(t0, y0, t1, y1))
	}
}

#[derive(Default, Debug, Clone, Copy)]
pub struct CubicInterpolator;

impl FixedLengthInterpolator for CubicInterpolator {
	fn fit(
		&self,
		initial: Option<EvaluatedPoint>,
		points: impl IntoIterator<Item = (f32, f32)>,
	) -> Option<CubicSegment> {
		let mut points = points.into_iter();
		if let Some(initial) = initial {
			let t0 = initial.t;
			let y0 = initial.y;
			let (t1, y1) = points.find(|&(t, _)| t > t0 + MIN_INTERPOLATION_INTERVAL)?;
			let (t2, y2) = points.find(|&(t, _)| t > t1 + MIN_INTERPOLATION_INTERVAL)?;
			InitialCubicSegmentSolver::new(t0, y0, initial.dy_dt, t2)
				.constrain_lt(t1, y1 + 0.5)
				.constrain_gt(t1, y1 - 0.5)
				.constrain_lt(t2, y2 + 0.5)
				.constrain_gt(t2, y2 - 0.5)
				.solve_smooth()
				.or_else(|| Some(CubicSegment::linear(t0, y0, t1, y1)))
		} else {
			let (t0, y0) = points.next()?;
			let (t1, y1) = points.find(|&(t, _)| t > t0 + MIN_INTERPOLATION_INTERVAL)?;
			let (t2, y2) = points.find(|&(t, _)| t > t1 + MIN_INTERPOLATION_INTERVAL)?;
			let (t3, y3) = points.find(|&(t, _)| t > t2 + MIN_INTERPOLATION_INTERVAL)?;
			CubicSegmentSolver::new(t0, t3)
				.constrain_lt(t0, y0 + 0.5)
				.constrain_gt(t0, y0 - 0.5)
				.constrain_lt(t1, y1 + 0.5)
				.constrain_gt(t1, y1 - 0.5)
				.constrain_lt(t2, y2 + 0.5)
				.constrain_gt(t2, y2 - 0.5)
				.constrain_lt(t3, y3 + 0.5)
				.constrain_gt(t3, y3 - 0.5)
				.solve_smooth()
				.or_else(|| Some(CubicSegment::linear(t0, y0, t1, y1)))
		}
	}
}

#[derive(Default, Debug, Clone)]
pub struct WindowInterpolator<Inner> {
	inner: Inner,
	last_point: Option<EvaluatedPoint>,
	points: std::collections::VecDeque<(f32, f32)>,
}

impl<Inner: FixedLengthInterpolator> WindowInterpolator<Inner> {
	pub fn new(inner: Inner) -> Self {
		Self {
			inner,
			last_point: None,
			points: Default::default(),
		}
	}

	pub fn add_point(&mut self, point: (f32, f32)) -> Option<CubicSegment> {
		self.points.push_back(point);
		let cubic = self
			.inner
			.fit(self.last_point, self.points.iter().copied())?;
		if self.last_point.is_none() {
			self.points.pop_front()?;
		}
		let (t1, _) = self.points.pop_front()?;
		self.last_point = Some(cubic.evaluate(t1));
		Some(cubic.restricted(cubic.t0, t1))
	}

	pub fn finish(self) -> Option<CubicSegment> {
		let (t1, _) = self.points.back()?.clone();
		let cubic = self
			.inner
			.fit(self.last_point, self.points.iter().copied())?;
		Some(cubic.restricted(cubic.t0, t1))
	}
}

fn solve_qp<const N: usize>(
	p: &[[f32; N]; N],
	q: &[f32; N],
	a: &[[f32; N]],
	b: &[f32],
	cones: &[clarabel::solver::SupportedConeT<f32>],
) -> Option<Vec<f32>> {
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
			tracing::error!(?status);
			None?
		}
		status => {
			tracing::warn!(?status);
		}
	};

	Some(solver.solution.x)
}

pub struct CubicSegmentSolver {
	t0: f32,
	t1: f32,
	a: Vec<[f32; 4]>,
	b: Vec<f32>,
	cones: Vec<clarabel::solver::SupportedConeT<f32>>,
}

impl CubicSegmentSolver {
	pub fn new(t0: f32, t1: f32) -> Self {
		Self {
			t0,
			t1,
			a: Vec::new(),
			b: Vec::new(),
			cones: Vec::new(),
		}
	}

	fn constraint_coefficients(&self, t: f32) -> [f32; 4] {
		let s = (t - self.t0) / (self.t1 - self.t0);
		let r = 1.0 - s;
		let s2 = s * s;
		let r2 = r * r;
		[r2 * r, 3.0 * r2 * s, 3.0 * r * s2, s * s2]
	}

	fn derivative_constraint_coefficients(&self, t: f32) -> [f32; 4] {
		let w = (self.t1 - self.t0).recip();
		let s = (t - self.t0) * w;
		let r = 1.0 - s;
		let c0 = w * 3.0 * r * r;
		let c1 = w * 6.0 * r * s;
		let c2 = w * 3.0 * s * s;
		[-c0, c0 - c1, c1 - c2, c2]
	}

	fn constrain_linear_lt(&mut self, coefficients: [f32; 4], value: f32) {
		self.a.push(coefficients);
		self.b.push(value);
		self
			.cones
			.push(clarabel::solver::SupportedConeT::NonnegativeConeT(1));
	}

	fn constrain_linear_eq(&mut self, coefficients: [f32; 4], value: f32) {
		self.a.push(coefficients);
		self.b.push(value);
		self
			.cones
			.push(clarabel::solver::SupportedConeT::ZeroConeT(1));
	}

	pub fn constrain_lt(mut self, t: f32, y: f32) -> Self {
		self.constrain_linear_lt(self.constraint_coefficients(t), y);
		self
	}

	pub fn constrain_gt(mut self, t: f32, y: f32) -> Self {
		self.constrain_linear_lt(self.constraint_coefficients(t).map(|c| -c), -y);
		self
	}

	pub fn constrain_eq(mut self, t: f32, y: f32) -> Self {
		self.constrain_linear_eq(self.constraint_coefficients(t), y);
		self
	}

	pub fn constrain_derivative_eq(mut self, t: f32, dy_dt: f32) -> Self {
		self.constrain_linear_eq(self.derivative_constraint_coefficients(t), dy_dt);
		self
	}

	pub fn solve_smooth(self) -> Option<CubicSegment> {
		let p = [
			[2.0 + EPSILON, -3.0, 0.0, 1.0],
			[-3.0, 6.0 - EPSILON, -3.0, 0.0],
			[0.0, -3.0, 6.0 - EPSILON, -3.0],
			[1.0, 0.0, -3.0, 2.0 + EPSILON],
		];
		let q = [0.0, 0.0, 0.0, 0.0];
		let solution = solve_qp(&p, &q, &self.a, &self.b, &self.cones)?;
		Some(CubicSegment {
			t0: self.t0,
			t1: self.t1,
			p: [solution[0], solution[1], solution[2], solution[3]],
		})
	}
}

pub struct InitialCubicSegmentSolver {
	t0: f32,
	t1: f32,
	p0: f32,
	p1: f32,
	a: Vec<[f32; 2]>,
	b: Vec<f32>,
	cones: Vec<clarabel::solver::SupportedConeT<f32>>,
}

impl InitialCubicSegmentSolver {
	pub fn new(t0: f32, y0: f32, dy_dt0: f32, t1: f32) -> Self {
		Self {
			t0,
			t1,
			p0: y0,
			p1: y0 + dy_dt0 * (t1 - t0) / 3.0,
			a: Vec::new(),
			b: Vec::new(),
			cones: Vec::new(),
		}
	}

	fn constraint_coefficients(&self, t: f32) -> ([f32; 2], f32) {
		let s = (t - self.t0) / (self.t1 - self.t0);
		let r = 1.0 - s;
		let s2 = s * s;
		let r2 = r * r;
		(
			[3.0 * r * s2, s * s2],
			r2 * (r * self.p0 + 3.0 * s * self.p1),
		)
	}

	fn constrain_linear_lt(&mut self, coefficients: [f32; 2], value: f32) {
		self.a.push(coefficients);
		self.b.push(value);
		self
			.cones
			.push(clarabel::solver::SupportedConeT::NonnegativeConeT(1));
	}

	pub fn constrain_lt(mut self, t: f32, y: f32) -> Self {
		let (coefficients, offset) = self.constraint_coefficients(t);
		self.constrain_linear_lt(coefficients, y - offset);
		self
	}

	pub fn constrain_gt(mut self, t: f32, y: f32) -> Self {
		let (coefficients, offset) = self.constraint_coefficients(t);
		self.constrain_linear_lt(coefficients.map(|c| -c), -(y - offset));
		self
	}

	pub fn solve_smooth(self) -> Option<CubicSegment> {
		let p = [[6.0, -3.0], [-3.0, 2.0]];
		let q = [-3.0 * self.p1, 1.0 * self.p0];
		let solution = solve_qp(&p, &q, &self.a, &self.b, &self.cones)?;
		Some(CubicSegment {
			t0: self.t0,
			t1: self.t1,
			p: [self.p0, self.p1, solution[0], solution[1]],
		})
	}
}

const EPSILON: f32 = 1e-2;

#[cfg(test)]
mod tests {
	use super::*;
	use approx::{abs_diff_eq, assert_abs_diff_eq};
	use std::assert_matches::assert_matches;

	#[test]
	fn test_cubic_segment_solver() {
		let cubic = CubicSegmentSolver::new(0.0, 4.0)
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
		let cubic = InitialCubicSegmentSolver::new(0.0, 1.0, 1.0, 4.0)
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
			EvaluatedPoint {
				t: 0.0,
				y: 0.0,
				dy_dt,
				d2y_dt2,
			} if abs_diff_eq!(dy_dt, 1.0, epsilon = EPSILON) && abs_diff_eq!(d2y_dt2, 0.0, epsilon = EPSILON)
		));
		assert!(matches!(
			cubic.evaluate(1.0),
			EvaluatedPoint {
				t: 1.0,
				y: 1.0,
				dy_dt,
				d2y_dt2,
			}  if abs_diff_eq!(dy_dt, 1.0, epsilon = EPSILON) && abs_diff_eq!(d2y_dt2, 0.0, epsilon = EPSILON)
		));

		let cubic = interpolator
			.fit(
				Some(EvaluatedPoint {
					t: 0.0,
					y: 0.0,
					dy_dt: 0.0,
					d2y_dt2: 0.0,
				}),
				[(1.0, 1.0)],
			)
			.unwrap();
		assert_matches!(
			cubic.evaluate(0.0),
			EvaluatedPoint {
				t: 0.0,
				y: 0.0,
				dy_dt,
				d2y_dt2,
			}  if abs_diff_eq!(dy_dt, 1.0, epsilon = EPSILON) && abs_diff_eq!(d2y_dt2, 0.0, epsilon = EPSILON)
		);
		assert_matches!(
			cubic.evaluate(1.0),
			EvaluatedPoint {
				t: 1.0,
				y: 1.0,
				dy_dt,
				d2y_dt2,
			}  if abs_diff_eq!(dy_dt, 1.0, epsilon = EPSILON) && abs_diff_eq!(d2y_dt2, 0.0, epsilon = EPSILON)

		);
	}

	#[test]
	fn test_linear_window_interpolator() {
		let mut interpolator: WindowInterpolator<LinearInterpolator> = Default::default();

		assert!(interpolator.add_point((0.0, 0.0)).is_none());

		let segment = interpolator.add_point((1.0, 1.0)).unwrap();
		assert_eq!(segment.t0, 0.0);
		assert_eq!(segment.t1, 1.0);

		let segment = interpolator.add_point((2.0, 0.0)).unwrap();
		assert_eq!(segment.t0, 1.0);
		assert_eq!(segment.t1, 2.0);

		assert!(interpolator.finish().is_none());
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
			EvaluatedPoint { t: 0.0, y, .. } if abs_diff_eq!(y, 0.5, epsilon = 2.0 * EPSILON)
		);
		assert_matches!(
			cubic.evaluate(1.0),
			EvaluatedPoint {
				t: 1.0,
				y,
				..
			} if abs_diff_eq!(y, 1.0, epsilon = 2.0 * EPSILON));

		let cubic = interpolator
			.fit(
				Some(EvaluatedPoint {
					t: 0.0,
					y: 0.0,
					dy_dt: 1.0,
					d2y_dt2: 0.0,
				}),
				[(1.0, 0.0), (2.0, -1.0)],
			)
			.unwrap();
		assert_matches!(
			cubic.evaluate(0.0),
			EvaluatedPoint {
				t: 0.0,
				y,
				dy_dt,
				..
			} if abs_diff_eq!(y, 0.0, epsilon = 2.0 * EPSILON) && abs_diff_eq!(dy_dt, 1.0, epsilon = 2.0 * EPSILON)
		);
	}

	#[test]
	fn test_cubic_window_interpolator() {
		let mut interpolator: WindowInterpolator<CubicInterpolator> = Default::default();

		assert!(interpolator.add_point((0.0, 0.0)).is_none());
		assert!(interpolator.add_point((1.0, 1.0)).is_none());
		assert!(interpolator.add_point((2.0, 2.0)).is_none());

		let segment = interpolator.add_point((3.0, 1.0)).unwrap();
		assert_eq!(segment.t0, 0.0);
		assert_eq!(segment.t1, 1.0);

		let segment = interpolator.add_point((4.0, 0.0)).unwrap();
		assert_eq!(segment.t0, 1.0);
		assert_eq!(segment.t1, 2.0);

		assert!(interpolator.finish().is_some());
	}

	#[test]
	fn test_cubic_window_interpolator_zig_zag() {
		let mut interpolator: WindowInterpolator<CubicInterpolator> = Default::default();

		assert!(interpolator.add_point((0.0, 0.0)).is_none());
		assert!(interpolator.add_point((1.0, 1.0)).is_none());
		assert!(interpolator.add_point((2.0, 1.0)).is_none());
		assert!(interpolator.add_point((3.0, 2.0)).is_some());
		assert!(interpolator.add_point((4.0, 2.0)).is_some());
		assert!(interpolator.add_point((5.0, 3.0)).is_some());
		assert!(interpolator.add_point((6.0, 3.0)).is_some());
		assert!(interpolator.add_point((7.0, 4.0)).is_some());
		assert!(interpolator.add_point((8.0, 4.0)).is_some());
		assert_abs_diff_eq!(
			interpolator
				.add_point((9.0, 5.0))
				.unwrap()
				.evaluate_first()
				.dy_dt,
			0.5,
			epsilon = EPSILON.sqrt()
		);
		assert_abs_diff_eq!(
			interpolator
				.add_point((10.0, 5.0))
				.unwrap()
				.evaluate_first()
				.dy_dt,
			0.5,
			epsilon = EPSILON.sqrt()
		);
	}
}
