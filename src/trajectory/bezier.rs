use std::ops::*;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubicBezier<X> {
	p: [X; 4],
	duration_scale: f64,
}

fn lerp<X: Clone + Add<Output = X> + Sub<Output = X> + Mul<f64, Output = X>>(
	a: X,
	b: X,
	t: f64,
) -> X {
	a.clone() + (b - a) * t
}

pub struct CubicBezierControl<X> {
	final_state: QuadraticState<X>,
	final_time: f64,
}

impl<X> Trajectory for CubicBezier<X>
where
	X: Clone + Add<Output = X> + Sub<Output = X> + Mul<f64, Output = X>,
{
	type State = QuadraticState<X>;
	type Control = CubicBezierControl<X>;

	fn evaluate(&self, duration: Duration) -> Self::State {
		let t = duration.get() * self.duration_scale;

		let (dq0, dq1, dq2) = (
			self.p1() - self.p0(),
			self.p2() - self.p1(),
			self.p3() - self.p2(),
		);
		let (q0, q1, q2) = (
			self.p0() + dq0.clone() * t,
			self.p1() + dq1.clone() * t,
			self.p2() + dq2.clone() * t,
		);

		QuadraticState::new(
			lerp(lerp(q0, q1.clone(), t), lerp(q1, q2, t), t),
			lerp(lerp(dq0, dq1.clone(), t), lerp(dq1, dq2, t), t) * (3.0 * self.duration_scale),
		)
	}

	fn control(self) -> Self::Control {
		let t_final = self.duration_scale.recip();
		let [_, _, p2, p3] = self.p;
		let velocity = (p3.clone() - p2) * (3.0 * self.duration_scale);
		CubicBezierControl {
			final_state: QuadraticState::new(p3, velocity),
			final_time: t_final,
		}
	}

	fn from_state_and_control(state: Self::State, control: Self::Control) -> Self {
		Self::interpolate(state, control.final_state, control.final_time)
	}
}

impl<X> CubicBezier<X>
where
	X: Clone + Add<Output = X> + Sub<Output = X> + Mul<f64, Output = X>,
{
	fn p0(&self) -> X {
		self.p[0].clone()
	}
	fn p1(&self) -> X {
		self.p[1].clone()
	}
	fn p2(&self) -> X {
		self.p[2].clone()
	}
	fn p3(&self) -> X {
		self.p[3].clone()
	}

	pub fn from_control_points(p: [X; 4], t_final: f64) -> Self {
		CubicBezier {
			p,
			duration_scale: t_final.recip(),
		}
	}

	pub fn interpolate(
		p_initial: QuadraticState<X>,
		p_final: QuadraticState<X>,
		t_final: f64,
	) -> Self {
		let w = t_final / 3.0;
		let [initial_position, initial_velocity] = p_initial.get();
		let [final_position, final_velocity] = p_final.get();
		let p = [
			initial_position.clone(),
			initial_position.clone() + initial_velocity.clone() * w,
			final_position.clone() - final_velocity * w,
			final_position,
		];
		Self::from_control_points(p, t_final)
	}

	pub fn interpolate_linear(p_initial: X, p_final: X, t_final: f64) -> Self {
		let tangent = (p_final.clone() - p_initial.clone()) * (1.0 / 3.0);
		let p1 = p_initial.clone() + tangent.clone();
		let p2 = p_final.clone() - tangent;
		Self::from_control_points([p_initial, p1, p2, p_final], t_final)
	}

	pub fn restricted(&self, t_initial: Duration, duration: Duration) -> Self {
		let t_final = t_initial + duration;
		let p_initial = self.evaluate(t_initial);
		let p_final = self.evaluate(t_final);
		Self::interpolate(p_initial, p_final, duration.get())
	}
}

impl<X> CubicBezier<X> {
	pub fn map_affine<Y>(self, f: impl Fn(X, f64) -> Y) -> CubicBezier<Y> {
		CubicBezier {
			p: self.p.map(|p| f(p, 1.0)),
			duration_scale: self.duration_scale,
		}
	}

	pub fn zip_affine<Y, Z>(
		self,
		other: CubicBezier<Y>,
		f: impl Fn(X, Y, f64) -> Z,
	) -> CubicBezier<Z> {
		let [p0, p1, p2, p3] = self.p;
		let [q0, q1, q2, q3] = other.p;
		CubicBezier {
			p: [
				f(p0, q0, 1.0),
				f(p1, q1, 1.0),
				f(p2, q2, 1.0),
				f(p3, q3, 1.0),
			],
			duration_scale: self.duration_scale,
		}
	}
}

fn solve_qp<const N: usize>(
	p: &[[f64; N]; N],
	q: &[f64; N],
	a: &[[f64; N]],
	b: &[f64],
	cones: &[clarabel::solver::SupportedConeT<f64>],
	epsilon: f64,
) -> Result<Vec<f64>> {
	debug_assert_eq!(a.len(), b.len());
	use clarabel::algebra::*;
	use clarabel::solver::*;

	let p = CscMatrix::from(p);
	let a = CscMatrix::from(a);
	let settings = DefaultSettings {
		verbose: false,
		max_iter: 16,
		tol_gap_abs: epsilon,
		tol_feas: epsilon,
		tol_infeas_abs: epsilon,
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
			Err(Error::SolveQPFailed)?
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
		.ok_or(Error::SolveQPNotFinite)
}

pub struct CubicBezierSolver {
	duration_scale: f64,
	a: Vec<[f64; 4]>,
	b: Vec<f64>,
	cones: Vec<clarabel::solver::SupportedConeT<f64>>,
}

impl CubicBezierSolver {
	pub fn new(t_final: f64) -> Self {
		Self {
			duration_scale: t_final.recip(),
			a: Vec::new(),
			b: Vec::new(),
			cones: Vec::new(),
		}
	}

	fn constraint_coefficients(&self, t: f64) -> [f64; 4] {
		let s = t * self.duration_scale;
		let r = 1.0 - s;
		let s2 = s * s;
		let r2 = r * r;
		[r2 * r, 3.0 * r2 * s, 3.0 * r * s2, s * s2]
	}

	fn derivative_constraint_coefficients(&self, t: f64) -> [f64; 4] {
		let s = t * self.duration_scale;
		let r = 1.0 - s;
		let c0 = self.duration_scale * 3.0 * r * r;
		let c1 = self.duration_scale * 6.0 * r * s;
		let c2 = self.duration_scale * 3.0 * s * s;
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

	pub fn constrain_lt(&mut self, t: f64, y: f64) -> &mut Self {
		self.constrain_linear_lt(self.constraint_coefficients(t), y as f64);
		self
	}

	pub fn constrain_gt(&mut self, t: f64, y: f64) -> &mut Self {
		self.constrain_linear_lt(self.constraint_coefficients(t).map(|c| -c), -y as f64);
		self
	}

	pub fn constrain_eq(&mut self, t: f64, y: f64) -> &mut Self {
		self.constrain_linear_eq(self.constraint_coefficients(t), y as f64);
		self
	}

	pub fn constrain_derivative_eq(&mut self, t: f64, dy_dt: f64) -> &mut Self {
		self.constrain_linear_eq(self.derivative_constraint_coefficients(t), dy_dt as f64);
		self
	}

	pub fn solve_smooth(&self) -> Result<CubicBezier<f64>> {
		const EPSILON: f64 = 1e-2;
		let p = [
			[2.0 + EPSILON, -3.0, 0.0, 1.0],
			[-3.0, 6.0 - EPSILON, -3.0, 0.0],
			[0.0, -3.0, 6.0 - EPSILON, -3.0],
			[1.0, 0.0, -3.0, 2.0 + EPSILON],
		];
		let q = [0.0, 0.0, 0.0, 0.0];
		let solution = solve_qp(&p, &q, &self.a, &self.b, &self.cones, EPSILON)?;
		Ok(CubicBezier {
			p: [solution[0], solution[1], solution[2], solution[3]],
			duration_scale: self.duration_scale,
		})
	}
}

pub struct CubicBezierControlSolver {
	duration_scale: f64,
	y0: f64,
	p1: f64,
	a: Vec<[f64; 2]>,
	b: Vec<f64>,
	cones: Vec<clarabel::solver::SupportedConeT<f64>>,
}

impl CubicBezierControlSolver {
	pub fn new(initial_state: QuadraticState<f64>, t_final: f64) -> Self {
		let [y0, dy_dt0] = initial_state.get();
		Self {
			duration_scale: t_final.recip(),
			y0,
			p1: dy_dt0 * (t_final / 3.0),
			a: Vec::new(),
			b: Vec::new(),
			cones: Vec::new(),
		}
	}

	fn constraint_coefficients(&self, t: f64) -> ([f64; 2], f64) {
		let s = t * self.duration_scale;
		let r = 1.0 - s;
		let s2 = s * s;
		(
			[3.0 * r * s2, s * s2],
			self.y0 + self.p1 * (3.0 * r * r * s),
		)
	}

	fn constrain_linear_lt(&mut self, coefficients: [f64; 2], value: f64) {
		self.a.push(coefficients);
		self.b.push(value);
		self
			.cones
			.push(clarabel::solver::SupportedConeT::NonnegativeConeT(1));
	}

	pub fn constrain_lt(&mut self, t: f64, y: f64) -> &mut Self {
		let (coefficients, offset) = self.constraint_coefficients(t);
		self.constrain_linear_lt(coefficients, y - offset);
		self
	}

	pub fn constrain_gt(&mut self, t: f64, y: f64) -> &mut Self {
		let (coefficients, offset) = self.constraint_coefficients(t);
		self.constrain_linear_lt(coefficients.map(|c| -c), -(y - offset));
		self
	}

	pub fn solve_smooth(&self) -> Result<CubicBezier<f64>> {
		const EPSILON: f64 = 1e-2;
		let p = [[6.0, -3.0], [-3.0, 2.0]];
		let q = [-3.0 * self.p1, 0.0];
		// We could consider using a different method here because this problem is strictly convex.
		let solution = solve_qp(&p, &q, &self.a, &self.b, &self.cones, EPSILON)?;
		Ok(CubicBezier {
			p: [
				self.y0,
				self.y0 + self.p1,
				self.y0 + solution[0],
				self.y0 + solution[1],
			],
			duration_scale: self.duration_scale,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use approx::assert_abs_diff_eq;

	const EPSILON: f64 = 1e-2;

	#[test]
	fn test_cubic_segment_solver() {
		let cubic = CubicBezierSolver::new(4.0)
			.constrain_gt(1.0, 2.0)
			.constrain_lt(3.0, 1.0)
			.solve_smooth()
			.unwrap();
		println!("{cubic:?}");
		assert!(*cubic.evaluate(1.0.try_into().unwrap()).position() > 2.0 - EPSILON);
		assert!(*cubic.evaluate(3.0.try_into().unwrap()).position() < 1.0 + EPSILON);
	}

	#[test]
	fn test_initial_cubic_segment_solver() {
		let cubic = CubicBezierControlSolver::new(QuadraticState::new(1.0, 1.0), 4.0)
			.constrain_gt(1.0, 2.0)
			.constrain_lt(3.0, 1.0)
			.solve_smooth()
			.unwrap();
		println!("{cubic:?}");
		assert_eq!(*cubic.evaluate(Duration::default()).position(), 1.0);
		assert_abs_diff_eq!(
			*cubic.evaluate(Duration::default()).velocity(),
			1.0,
			epsilon = EPSILON
		);
		assert!(*cubic.evaluate(1.0.try_into().unwrap()).position() > 2.0 - EPSILON);
		assert!(*cubic.evaluate(3.0.try_into().unwrap()).position() < 1.0 + EPSILON);
	}

	#[test]
	fn test_initial_cubic_segment_solver_strict() {
		let cubic = CubicBezierControlSolver::new(QuadraticState::new(0.0, 0.0), 4.0)
			.constrain_gt(1.0, 1.0)
			.constrain_lt(1.0, 1.0)
			.constrain_gt(2.0, 2.0)
			.constrain_lt(2.0, 2.0)
			.solve_smooth()
			.unwrap();
		println!("{cubic:?}");
		assert_abs_diff_eq!(
			*cubic.evaluate(0.0.try_into().unwrap()).position(),
			0.0,
			epsilon = EPSILON
		);
		assert_abs_diff_eq!(
			*cubic.evaluate(1.0.try_into().unwrap()).position(),
			1.0,
			epsilon = EPSILON
		);
		assert_abs_diff_eq!(
			*cubic.evaluate(2.0.try_into().unwrap()).position(),
			2.0,
			epsilon = EPSILON
		);
	}
}
