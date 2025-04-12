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
	// Invariant: `distances.len() == spline.num_pieces()`
	distances: Vec<Distance>,
	_reparameterization: PhantomData<R>,
}

/// Returns `x` such that the integral of `dy_dx` from 0 to `x` equals `y`.
///
/// Implementation: Newton's method with bracketing.
fn solve_positive_integral(dy_dx: impl Fn(f64) -> f64, mut y: f64, x_0: f64) -> f64 {
	const EPS: f64 = 1e-5;
	let mut a = 0.0;
	let mut b = None;
	let mut x_i = x_0;
	for i in 0..50 {
		let dy_dx_i = dy_dx(x_i) + (-i as f64).exp2();
		let y_i = quadrature::integrate(&dy_dx, a, x_i, EPS);
		let delta_y = y_i.integral - y;

		// This is an optimization, not necessary for correctness.
		if delta_y < 0.0 {
			y = -delta_y;
			a = x_i;
		} else {
			b = Some(x_i);
		}

		x_i -= delta_y / (dy_dx_i + y_i.error_estimate);
		if let Some(b) = b {
			let d = 0.25 * (b - a);
			x_i = x_i.clamp(a + d, b - d);
		}
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

	pub fn num_pieces(&self) -> usize {
		self.spline.num_pieces()
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

	pub fn set_control_after_time(
		&mut self,
		time: Duration,
		control: Piece::Control,
	) -> Option<(usize, Duration)> {
		let Some((index, duration)) = self.spline.set_control_after(time, control) else {
			self.distances.truncate(self.spline.num_pieces());
			return None;
		};
		self.distances.truncate(index + 1);
		let piece = self.spline.get_piece(index).unwrap();
		let length = Self::piece_length(piece, duration);
		self
			.distances
			.push(*self.distances.last().unwrap() + length);
		debug_assert_eq!(self.distances.len(), self.spline.num_pieces());
		Some((index, length))
	}

	fn distance_to_piece_index_and_time(&self, distance: Distance) -> (usize, Duration) {
		let i = self.distances[1..].partition_point(move |&d_i| !(d_i > distance));
		let piece_distance = self.distances[i];
		let piece: &Piece = self.spline.get_piece(i).unwrap();

		let distance_on_piece = distance.get() - piece_distance.get();

		// TODO: If we're not on the last piece, we can linearly interpolate to get a better estimate
		// here.
		let approximate_time = 0.0;

		(
			i,
			Duration::clamp(solve_positive_integral(
				|t| R::ds_dt(&piece.evaluate(Duration::clamp(t))),
				distance_on_piece,
				approximate_time,
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

impl<R: Reparameterization<Piece::State>, Piece: Trajectory> From<Spline<Piece>>
	for ReparameterizedSpline<R, Piece>
{
	fn from(value: Spline<Piece>) -> Self {
		ReparameterizedSpline::from_spline(value)
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

	fn into_control(self) -> Self::Control {
		self.spline.into_control()
	}

	fn from_state_and_control((state, speed): Self::State, control: Self::Control) -> Self {
		let temporal_state = R::scale_domain(state, speed);
		Self::from_spline(Spline::from_state_and_control(temporal_state, control))
	}

	fn into_tail(self, distance: Duration) -> Self {
		// TODO: This could be done more efficiently.
		let time = self.distance_to_time(distance);
		self.spline.into_tail(time).into()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use approx::assert_abs_diff_eq;

	#[derive(Debug)]
	struct TestReparameterization;
	impl Reparameterization<QuadraticState<f64>> for TestReparameterization {
		fn ds_dt(state: &QuadraticState<f64>) -> f64 {
			state.velocity().abs()
		}
		fn scale_domain(state: QuadraticState<f64>, factor: f64) -> QuadraticState<f64> {
			state.domain_scaled(factor)
		}
	}

	#[test]
	fn test_set_control_after_time() -> anyhow::Result<()> {
		let mut spline = ReparameterizedSpline::<TestReparameterization, _>::new(
			Quadratic::from_state_and_control(QuadraticState::new(0.0, 1.0), 0.0),
		);

		assert_eq!(spline.num_pieces(), 1);
		spline.set_control_after_time(1.0.try_into()?, 0.0);
		assert_eq!(spline.num_pieces(), 2);
		spline.set_control_after_time(2.0.try_into()?, 0.0);
		assert_eq!(spline.num_pieces(), 3);
		spline.set_control_after_time(1.0.try_into()?, 0.0);
		assert_eq!(spline.num_pieces(), 2);
		spline.set_control_after_time(0.0.try_into()?, 0.0);
		assert_eq!(spline.num_pieces(), 1);

		Ok(())
	}

	#[test]
	fn test_reparameterize_spline() -> anyhow::Result<()> {
		let mut spline = ReparameterizedSpline::<TestReparameterization, _>::new(
			Quadratic::from_state_and_control(QuadraticState::new(0.0, 1.0), 0.0),
		);
		spline.set_control_after_time(1.0.try_into()?, 1.0);
		spline.set_control_after_time(2.0.try_into()?, -1.0);
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
