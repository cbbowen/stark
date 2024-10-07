use super::*;

const EPSILON: f64 = 1e-5;

pub type Distance = Duration;

trait Reparameterization<State> {
	fn ds_dt(state: &State) -> f64;
	fn scale_domain(state: State, factor: f64) -> State;
}

#[derive(Clone, Debug)]
struct ReparameterizedSpline<R, Piece> {
	spline: Spline<Piece>,
	distances: Vec<Distance>,
	_reparameterization: PhantomData<R>,
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

impl<R, Piece> ReparameterizedSpline<R, Piece> {
	pub fn into_spline(self) -> Spline<Piece> {
		self.spline
	}
}

impl<R: Reparameterization<Piece::State>, Piece: Trajectory> ReparameterizedSpline<R, Piece> {
	pub fn new(piece: Piece) -> Self {
		Self::from_spline(Spline::new(piece))
	}

	fn piece_length(piece: &Piece, duration: Duration) -> Distance {
		Distance::clamp(
			quadrature::integrate(
				|t| R::ds_dt(&piece.evaluate(Duration::clamp(t))),
				0.0,
				duration.get(),
				EPSILON,
			)
			.integral,
		)
	}

	pub fn from_spline(spline: Spline<Piece>) -> Self {
		let distances = spline
			.segments()
			.map(|(piece, duration)| Self::piece_length(piece, duration))
			.scan(Distance::default(), |d, l| {
				*d += l;
				Some(*d)
			})
			.collect();
		Self {
			spline,
			distances,
			_reparameterization: Default::default(),
		}
	}

	pub fn add_control(&mut self, time: Duration, control: Piece::Control) -> Result<()> {
		let (duration, _) = self.spline.add_control(time, control)?;
		let piece = self.spline.get_piece(self.distances.len() - 1).unwrap();
		let length = Self::piece_length(piece, duration);
		self
			.distances
			.push(*self.distances.last().unwrap() + length);
		Ok(())
	}

	fn distance_to_piece_index_and_time(&self, distance: Distance) -> (usize, Duration) {
		let i = self
			.distances
			.partition_point(move |&d_i| !(d_i > distance))
			- 1;
		let piece_distance = self.distances[i];
		let piece: &Piece = self.spline.get_piece(i).unwrap();

		(
			i,
			Duration::clamp(solve_positive_integral(
				|t| R::ds_dt(&piece.evaluate(Duration::clamp(t))),
				distance.get() - piece_distance.get(),
				0.0,
			)),
		)
	}

	pub fn distance_to_time(&self, distance: Distance) -> Duration {
		let (i, t) = self.distance_to_piece_index_and_time(distance);
		self.spline.get_time(i).unwrap() + t
	}

	pub fn time_to_distance(&self, time: Duration) -> Distance {
		let i = self.spline.time_to_index(time);
		let piece = self.spline.get_piece(i).unwrap();
		let duration = time.try_sub(self.spline.get_time(i).unwrap()).unwrap();
		let length = Self::piece_length(piece, duration);
		self.distances[i] + length
	}
}

impl<R: Reparameterization<Piece::State>, Piece: Trajectory> Trajectory
	for ReparameterizedSpline<R, Piece>
{
	type State = (Piece::State, f64);
	type Control = <Spline<Piece> as Trajectory>::Control;

	fn evaluate(&self, distance: Duration) -> Self::State {
		let (i, piece_time) = self.distance_to_piece_index_and_time(distance);
		let piece = self.spline.get_piece(i).unwrap();

		let temporal_state = piece.evaluate(piece_time);
		let speed = R::ds_dt(&temporal_state);
		(R::scale_domain(temporal_state, speed.recip()), speed)
	}

	fn control(self) -> Self::Control {
		self.spline.control()
	}

	fn from_state_and_control((state, speed): Self::State, control: Self::Control) -> Self {
		let temporal_state = R::scale_domain(state, speed);
		Self::from_spline(Spline::from_state_and_control(temporal_state, control))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use approx::assert_abs_diff_eq;

	#[test]
	fn test_reparameterize_spline() -> anyhow::Result<()> {
		#[derive(Debug)]
		struct R;

		impl Reparameterization<QuadraticState<f64>> for R {
			fn ds_dt(state: &QuadraticState<f64>) -> f64 {
				state.velocity().abs()
			}

			fn scale_domain(state: QuadraticState<f64>, factor: f64) -> QuadraticState<f64> {
				state.domain_scaled(factor)
			}
		}

		let mut spline = ReparameterizedSpline::<R, _>::new(Quadratic::from_state_and_control(
			QuadraticState::new(0.0, 1.0),
			0.0,
		));
		spline.add_control(1.0.try_into()?, 1.0)?;
		spline.add_control(2.0.try_into()?, -1.0)?;
		println!("{spline:?}");

		for x in [0.0, 1.0, 2.0, 3.0] {
			assert_abs_diff_eq!(
				*spline.evaluate(x.try_into()?).0.position(),
				x,
				epsilon = 1e-3
			);
		}
		Ok(())
	}
}
