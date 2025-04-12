use std::collections::VecDeque;

use crate::{trajectory::{
	CubicBezier, CubicBezierControlSolver, CubicBezierSolver, Duration, Linear, QuadraticState,
	Result, Spline, Trajectory,
}, util::ResultExt};

use glam::{dvec2, DVec2};

type CoordinateTrajectory = CubicBezier<f64>;
type CoordinateState = <CoordinateTrajectory as Trajectory>::State;
type PositionPiece = CubicBezier<DVec2>;
type ParamTrajectory = Spline<Linear<f64>>;

fn coordinate_fit(
	initial: Option<(f64, CoordinateState)>,
	points: impl IntoIterator<Item = (f64, f64)>,
) -> Result<Option<(CubicBezier<f64>, bool)>> {
	let points = points.into_iter();
	if let Some((t0, initial)) = initial {
		let points: Vec<_> = points.take(2).collect();
		if points.len() < 1 {
			return Ok(None);
		}
		let has_max_points = points.len() >= 2;
		let t_last = points.last().unwrap().0;
		let mut solver = CubicBezierControlSolver::new(initial, t_last - t0);
		for (t, y) in points {
			solver
				.constrain_lt(t - t0, y + 0.5)
				.constrain_gt(t - t0, y - 0.5);
		}
		solver.solve_smooth().map(|c| Some((c, has_max_points)))
	} else {
		let points: Vec<_> = points.take(4).collect();
		if points.len() < 2 {
			return Ok(None);
		}
		let has_max_points = points.len() >= 4;
		let t0 = points.first().unwrap().0;
		let t_last = points.last().unwrap().0;
		let mut solver = CubicBezierSolver::new(t_last - t0);
		for (t, y) in points {
			solver
				.constrain_lt(t - t0, y + 0.5)
				.constrain_gt(t - t0, y - 0.5);
		}
		solver.solve_smooth().map(|c| Some((c, has_max_points)))
	}
}

pub type PointParams = f64;

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct InputPoint {
	pub t: f64,
	pub x: f64,
	pub y: f64,
	pub params: PointParams,
}

#[derive(Default, Clone)]
pub struct InputDifferentiator {
	input_points: VecDeque<InputPoint>,
	last_point: Option<(f64, (QuadraticState<DVec2>, PointParams))>,
}

impl InputDifferentiator {
	pub fn new() -> Self {
		Self {
			input_points: Default::default(),
			last_point: None,
		}
	}

	fn x_points(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
		self.input_points.iter().map(|p| (p.t, p.x))
	}

	fn y_points(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
		self.input_points.iter().map(|p| (p.t, p.y))
	}

	fn next_fit(&mut self) -> Option<((PositionPiece, ParamTrajectory), f64, Option<Duration>)> {
		let last_point = self.last_point.take().clone();
		let x_fit = coordinate_fit(
			last_point.map(|(t, (position, _))| (t, position.map_affine(|p, _| p.x))),
			self.x_points(),
		);
		let y_fit = coordinate_fit(
			last_point.map(|(t, (position, _))| (t, position.map_affine(|p, _| p.y))),
			self.y_points(),
		);
		let (Ok(x_fit), Ok(y_fit)) = (x_fit, y_fit) else {
			// If coordinate fitting fails, try to recover by forgetting input points.
			self.input_points.pop_front();
			self.input_points.pop_front();
			return None;
		};
		let (x_bezier, fit_max_points) = x_fit?;
		let (y_bezier, y_fit_max_points) = y_fit?;
		debug_assert_eq!(y_fit_max_points, fit_max_points);

		let position_trajectory = x_bezier.zip_affine(y_bezier, |x, y, _| dvec2(x, y));

		let last_params = if let Some(last_point) = last_point {
			Some((last_point.0, last_point.1.1))
		} else if fit_max_points {
			self.input_points.pop_front().map(|p| (p.t, p.params))
		} else {
			None
		};

		let input_params = self.input_points.iter().map(|p| (p.t, p.params));
		let times_and_params = last_params.into_iter().chain(input_params);
		let (t0, param_trajectory) = ParamTrajectory::interpolate(times_and_params).unwrap();

		let trajectory = (position_trajectory, param_trajectory);
		let t1 = fit_max_points.then(|| self.input_points.pop_front().unwrap().t);
		let d = t1.and_then(|t1| (t1 - t0).try_into().ok_or_log());
		if let Some(d) = d {
			self.last_point = Some((t1.unwrap(), trajectory.evaluate(d)));
		}
		Some((trajectory, t0, d))
	}

	// Returns a trajectory and a duration along it that is now immutable. The rest of the trajectory
	// is a prediction.
	// TODO: I think we need a way to provide a prediction without making anything immutable. This
	// better supports short paths.
	pub fn add_point(
		&mut self,
		point: InputPoint,
	) -> Option<((PositionPiece, ParamTrajectory), f64, Option<Duration>)> {
		const MIN_INTERPOLATION_INTERVAL: f64 = 0.125;
		if let Some((last_t, _)) = self.last_point {
			if point.t < last_t + MIN_INTERPOLATION_INTERVAL {
				return None;
			}
		}
		self.input_points.push_back(point);
		self.next_fit()
	}

	pub fn reset(&mut self) {
		self.input_points.clear();
		self.last_point = None;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use approx::assert_abs_diff_eq;

	const EPSILON: f64 = 1e-2;

	#[test]
	fn test_step_smoothing() {
		let mut differentiator = InputDifferentiator::new();

		assert!(differentiator
			.add_point(InputPoint {
				t: 0.0,
				x: 0.0,
				..Default::default()
			})
			.is_none());

		assert!(differentiator
			.add_point(InputPoint {
				t: 1.0,
				x: 1.5,
				..Default::default()
			})
			.is_some());

		assert!(differentiator
			.add_point(InputPoint {
				t: 2.0,
				x: 1.5,
				..Default::default()
			})
			.is_some());

		let ((traj, _), t0, d) = differentiator
			.add_point(InputPoint {
				t: 3.0,
				x: 3.5,
				..Default::default()
			})
			.unwrap();
		assert_abs_diff_eq!(t0, 0.0);
		let d = d.unwrap();
		assert_abs_diff_eq!(d.get(), 1.0);
		let p = traj.evaluate(d);
		assert_abs_diff_eq!(p.position().x, 1.0, epsilon = EPSILON);
		assert_abs_diff_eq!(p.position().y, 0.0, epsilon = EPSILON);
		// The X velocity hasn't quite converged at this point.
		assert_abs_diff_eq!(p.velocity().y, 0.0, epsilon = EPSILON);

		let ((traj, _), t0, d) = differentiator
			.add_point(InputPoint {
				t: 4.0,
				x: 3.5,
				..Default::default()
			})
			.unwrap();
		assert_abs_diff_eq!(t0, 1.0);
		let d = d.unwrap();
		assert_abs_diff_eq!(d.get(), 1.0);
		let p = traj.evaluate(d);
		assert_abs_diff_eq!(p.position().x, 2.0, epsilon = EPSILON);
		assert_abs_diff_eq!(p.position().y, 0.0, epsilon = EPSILON);
		assert_abs_diff_eq!(p.velocity().x, 1.0, epsilon = EPSILON);
		assert_abs_diff_eq!(p.velocity().y, 0.0, epsilon = EPSILON);

		let ((traj, _), t0, d) = differentiator
			.add_point(InputPoint {
				t: 5.0,
				x: 5.5,
				..Default::default()
			})
			.unwrap();
		assert_abs_diff_eq!(t0, 2.0);
		let d = d.unwrap();
		assert_abs_diff_eq!(d.get(), 1.0);
		let p = traj.evaluate(d);
		assert_abs_diff_eq!(p.position().x, 3.0, epsilon = EPSILON);
		assert_abs_diff_eq!(p.position().y, 0.0, epsilon = EPSILON);
		assert_abs_diff_eq!(p.velocity().x, 1.0, epsilon = EPSILON);
		assert_abs_diff_eq!(p.velocity().y, 0.0, epsilon = EPSILON);
	}
}
