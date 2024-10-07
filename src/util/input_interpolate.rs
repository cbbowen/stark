use std::collections::VecDeque;

use crate::trajectory::CubicBezier;
use crate::trajectory::CubicBezierControlSolver;
use crate::trajectory::CubicBezierSolver;
use crate::trajectory::Duration;
use crate::trajectory::Linear;
use crate::trajectory::QuadraticState;
use crate::trajectory::Trajectory;

use glam::dvec2;
use glam::DVec2;

type CoordinateTrajectory = CubicBezier<f64>;
type CoordinateState = <CoordinateTrajectory as Trajectory>::State;
type PositionTrajectory = CubicBezier<DVec2>;
type ParamTrajectory = Linear<f64>;

fn coordinate_fit(
	initial: Option<(f64, CoordinateState)>,
	points: impl IntoIterator<Item = (f64, f64)>,
) -> Option<CubicBezier<f64>> {
	let mut points = points.into_iter();
	if let Some((t0, initial)) = initial {
		let (t1, y1) = points.next()?;
		let (t2, y2) = points.next()?;
		let (d1, d2) = (t1 - t0, t2 - t0);
		CubicBezierControlSolver::new(initial, d2)
			.constrain_lt(d1, y1 + 0.5)
			.constrain_gt(d1, y1 - 0.5)
			.constrain_lt(d2, y2 + 0.5)
			.constrain_gt(d2, y2 - 0.5)
			.solve_smooth()
	} else {
		let points: Vec<_> = points.take(4).collect();
		if points.len() < 2 {
			return None;
		}
		let t0 = points.first()?.0;
		let t_last = points.last()?.0;
		let mut solver = CubicBezierSolver::new(t_last - t0);
		for (t, y) in points {
			solver
				.constrain_lt(t - t0, y + 0.5)
				.constrain_gt(t - t0, y - 0.5);
		}
		solver.solve_smooth()
	}
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct InputPoint {
	pub t: f64,
	pub x: f64,
	pub y: f64,
	pub pressure: f64,
}

#[derive(Default, Clone)]
pub struct InputDifferentiator {
	input_points: VecDeque<InputPoint>,
	last_point: Option<(f64, (QuadraticState<DVec2>, f64))>,
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

	pub fn add_point(
		&mut self,
		point: InputPoint,
	) -> Option<((PositionTrajectory, ParamTrajectory), f64)> {
		let last_point = self.last_point.clone();
		const MIN_INTERPOLATION_INTERVAL: f64 = 0.125;
		if let Some((last_t, _)) = last_point {
			if point.t < last_t + MIN_INTERPOLATION_INTERVAL {
				return None;
			}
		}

		self.input_points.push_back(point);
		let x_bezier = coordinate_fit(
			last_point.map(|(t, (position, _))| (t, position.map_affine(|p, _| p.x))),
			self.x_points(),
		)?;
		let y_bezier = coordinate_fit(
			last_point.map(|(t, (position, _))| (t, position.map_affine(|p, _| p.y))),
			self.y_points(),
		)?;
		let position_trajectory = x_bezier.zip_affine(y_bezier, |x, y, _| dvec2(x, y));

		let (t0, params0) = if let Some((t, state)) = last_point {
			(t, state.1)
		} else {
			let p = self.input_points.pop_front()?;
			(p.t, p.pressure)
		};
		let InputPoint {
			t: t1,
			pressure: params1,
			..
		} = self.input_points.pop_front()?;
		let duration: Duration = (t1 - t0).try_into().ok()?;

		// TODO: This is probably not the best way to interpolate params.
		let param_trajectory = ParamTrajectory::interpolate(params0, params1, duration.get());

		let trajectory = (position_trajectory, param_trajectory);
		self.last_point = Some((t1, trajectory.evaluate(duration)));
		Some((trajectory, duration.get()))
	}

	pub fn finish(self) -> Option<f64> {
		Some(self.input_points.back()?.t)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use approx::{abs_diff_eq, assert_abs_diff_eq};
	use std::assert_matches::assert_matches;

	const EPSILON: f64 = 1e-2;

	#[test]
	fn test_cubic_interpolator() {
		let interpolator = CubicBezierFit;
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
		let mut interpolator: InputDifferentiator<CubicBezierFit> = Default::default();

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
		let mut spline: InputDifferentiator<CubicBezierFit> = Default::default();

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
