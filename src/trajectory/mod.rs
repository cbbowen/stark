use std::{marker::PhantomData, ops::*};
use thiserror::Error;

mod state;
pub use state::{CubicState, QuadraticState};

mod bezier;
pub use bezier::*;

mod reparameterize;

#[derive(Debug, Error)]
pub enum Error {
	#[error("duration out of range")]
	DurationOutOfRange,
	#[error("error solving quadratic program")]
	SolveQPFailed,
	#[error("quadratic program had a non-finite solution")]
	SolveQPNotFinite,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct Duration(f64);

impl TryFrom<f64> for Duration {
	type Error = Error;
	fn try_from(value: f64) -> Result<Self> {
		(value >= 0.0)
			.then_some(Self(value))
			.ok_or(Error::DurationOutOfRange)
	}
}

impl Duration {
	pub fn clamp(value: f64) -> Self {
		Duration(value.max(0.0))
	}

	pub fn get(self) -> f64 {
		self.0
	}

	pub fn try_sub(self, rhs: Duration) -> Result<Self> {
		(self.get() - rhs.get()).try_into()
	}
}

impl Add for Duration {
	type Output = Self;
	fn add(self, rhs: Self) -> Self::Output {
		(self.get() + rhs.get()).try_into().unwrap()
	}
}

impl AddAssign for Duration {
	fn add_assign(&mut self, rhs: Self) {
		self.0 += rhs.get()
	}
}

impl Eq for Duration {}

impl Ord for Duration {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.partial_cmp(other).unwrap()
	}
}

pub trait Trajectory {
	type State: Clone;
	type Control;

	fn evaluate(&self, duration: Duration) -> Self::State;

	fn control(self) -> Self::Control;
	fn from_state_and_control(state: Self::State, control: Self::Control) -> Self;
}

impl<A: Trajectory, B: Trajectory> Trajectory for (A, B) {
	type State = (A::State, B::State);
	type Control = (A::Control, B::Control);

	fn evaluate(&self, duration: Duration) -> Self::State {
		(self.0.evaluate(duration), self.1.evaluate(duration))
	}

	fn control(self) -> Self::Control {
		(self.0.control(), self.1.control())
	}

	fn from_state_and_control(state: Self::State, control: Self::Control) -> Self {
		(
			A::from_state_and_control(state.0, control.0),
			B::from_state_and_control(state.1, control.1),
		)
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Linear<X> {
	state: X,
	velocity: X,
}

impl<X: Default> Linear<X> {
	pub fn constant(state: X) -> Self {
		let velocity = X::default();
		Linear { state, velocity }
	}
}

impl<X> Trajectory for Linear<X>
where
	X: Clone + Add<Output = X> + Mul<f64, Output = X>,
{
	type State = X;
	type Control = X;

	fn evaluate(&self, duration: Duration) -> Self::State {
		self.state.clone() + self.velocity.clone() * duration.get()
	}

	fn control(self) -> Self::Control {
		self.velocity
	}

	fn from_state_and_control(state: Self::State, control: Self::Control) -> Self {
		Self {
			state,
			velocity: control,
		}
	}
}

impl<X> Linear<X>
where
	X: Clone + Sub<Output = X> + Mul<f64, Output = X>,
{
	pub fn interpolate(p_initial: X, p_final: X, t_final: f64) -> Self {
		let velocity = (p_final - p_initial.clone()) * t_final.recip();
		Linear {
			state: p_initial,
			velocity,
		}
	}
}

impl<X> Linear<X> {
	pub fn map_affine<Y>(self, f: impl Fn(X, f64) -> Y) -> Linear<Y> {
		Linear {
			state: f(self.state, 1.0),
			velocity: f(self.velocity, 0.0),
		}
	}

	pub fn zip_affine<Y, Z>(self, other: Linear<Y>, f: impl Fn(X, Y, f64) -> Z) -> Linear<Z> {
		Linear {
			state: f(self.state, other.state, 1.0),
			velocity: f(self.velocity, other.velocity, 0.0),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quadratic<X> {
	state: QuadraticState<X>,
	acceleration: X,
}

impl<X> Trajectory for Quadratic<X>
where
	X: Clone + Add<Output = X> + Mul<f64, Output = X>,
{
	type State = QuadraticState<X>;
	type Control = X;

	fn evaluate(&self, duration: Duration) -> Self::State {
		let duration = duration.get();
		let [position, velocity] = self.state.clone().get();
		let acceleration = self.acceleration.clone();
		QuadraticState::new(
			position + (velocity.clone() + acceleration.clone() * (duration / 2.0)) * duration,
			velocity + acceleration * duration,
		)
	}

	fn control(self) -> Self::Control {
		self.acceleration
	}

	fn from_state_and_control(state: Self::State, control: Self::Control) -> Self {
		Self {
			state,
			acceleration: control,
		}
	}
}

impl<X> Quadratic<X> {
	pub fn map_affine<Y>(self, f: impl Fn(X, f64) -> Y) -> Quadratic<Y> {
		let acceleration = f(self.acceleration, 0.0);
		Quadratic {
			state: self.state.map_affine(f),
			acceleration,
		}
	}

	pub fn zip_affine<Y, Z>(self, other: Quadratic<Y>, f: impl Fn(X, Y, f64) -> Z) -> Quadratic<Z> {
		let acceleration = f(self.acceleration, other.acceleration, 0.0);
		Quadratic {
			state: self.state.zip_affine(other.state, f),
			acceleration,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cubic<X> {
	state: CubicState<X>,
	jerk: X,
}

impl<X> Trajectory for Cubic<X>
where
	X: Clone + Add<Output = X> + Mul<f64, Output = X>,
{
	type State = CubicState<X>;
	type Control = X;

	fn evaluate(&self, duration: Duration) -> Self::State {
		let duration = duration.get();
		let [position, velocity, acceleration] = self.state.clone().get();
		let jerk = self.jerk.clone();
		CubicState::new(
			position
				+ (velocity.clone()
					+ (acceleration.clone() + jerk.clone() * (duration / 3.0)) * (duration / 2.0))
					* duration,
			velocity + (acceleration.clone() + jerk.clone() * (duration / 2.0)) * duration,
			acceleration + jerk * duration,
		)
	}

	fn control(self) -> Self::Control {
		self.jerk
	}

	fn from_state_and_control(state: Self::State, control: Self::Control) -> Self {
		Self {
			state,
			jerk: control,
		}
	}
}

impl<X> Cubic<X>
where
	X: Clone + Add<Output = X> + Sub<Output = X> + Mul<f64, Output = X>,
{
	pub fn interpolate(
		p_initial: QuadraticState<X>,
		p_final: QuadraticState<X>,
		t_final: f64,
	) -> Self {
		let w = t_final / 3.0;

		let [initial_position, initial_velocity] = p_initial.get();
		let [final_position, final_velocity] = p_final.get();
		let (p0, p1, p2, p3) = (
			initial_position.clone(),
			initial_position.clone() + initial_velocity.clone() * w,
			final_position.clone() - final_velocity * w,
			final_position,
		);
		let (q0, q1, q2) = (p1.clone() - p0, p2.clone() - p1, p3 - p2);
		let (r0, r1) = (q1.clone() - q0, q2 - q1);

		let t_inv = t_final.recip();
		let t_inv_2_6 = 6.0 * t_inv.powi(2);
		let acceleration = r0.clone() * t_inv_2_6;
		let jerk = (r1 - r0) * (t_inv * t_inv_2_6);

		Cubic {
			state: CubicState::new(initial_position, initial_velocity, acceleration),
			jerk,
		}
	}
}

impl<X> Cubic<X> {
	pub fn map_affine<Y>(self, f: impl Fn(X, f64) -> Y) -> Cubic<Y> {
		let jerk = f(self.jerk, 0.0);
		Cubic {
			state: self.state.map_affine(f),
			jerk,
		}
	}

	pub fn zip_affine<Y, Z>(self, other: Cubic<Y>, f: impl Fn(X, Y, f64) -> Z) -> Cubic<Z> {
		let jerk = f(self.jerk, other.jerk, 0.0);
		Cubic {
			state: self.state.zip_affine(other.state, f),
			jerk,
		}
	}
}

/// A spline is a trajectory defined piecewise.
#[derive(Debug, Clone, PartialEq)]
pub struct Spline<Piece> {
	// Invariant: non-empty and ordered by duration.
	pieces: Vec<(Duration, Piece)>,
}

impl<Piece> Spline<Piece> {
	pub fn new(piece: Piece) -> Self {
		Self {
			pieces: vec![(Duration::default(), piece)],
		}
	}

	pub fn segments(&self) -> impl Iterator<Item = (&Piece, Duration)> + '_ {
		self
			.pieces
			.iter()
			.scan(Duration::default(), |prev_time, (time, piece)| {
				let duration = time.try_sub(*prev_time).unwrap();
				*prev_time = *time;
				Some((piece, duration))
			})
	}

	pub fn get_piece(&self, index: usize) -> Option<&Piece> {
		let (_, piece) = self.pieces.get(index)?;
		Some(piece)
	}

	pub fn get_time(&self, index: usize) -> Option<Duration> {
		let (duration, _) = self.pieces.get(index)?;
		Some(duration.clone())
	}

	pub fn time_to_index(&self, time: Duration) -> usize {
		self.pieces[1..].partition_point(|(t, _)| t <= &time)
	}

	pub fn single_piece(self) -> Option<Piece> {
		(self.pieces.len() == 1).then_some(self.pieces.into_iter().next()?.1)
	}

	pub fn trim(&mut self, time: Duration) {
		self.pieces.shrink_to(self.time_to_index(time));
	}
}

impl<Piece: Trajectory> Spline<Piece> {
	pub fn add_control(
		&mut self,
		time: Duration,
		control: Piece::Control,
	) -> Result<(Duration, &Piece)> {
		let (last_time, last_piece) = self.pieces.last().unwrap();
		let duration = time.try_sub(*last_time)?;
		let state = last_piece.evaluate(duration);
		self
			.pieces
			.push((time, Piece::from_state_and_control(state, control)));
		Ok((duration, &self.pieces.last().unwrap().1))
	}
}

impl<Piece: Trajectory> Trajectory for Spline<Piece> {
	type State = Piece::State;
	type Control = Vec<(Duration, Piece::Control)>;

	fn evaluate(&self, duration: Duration) -> Self::State {
		let index = self.time_to_index(duration);
		let (time, piece) = &self.pieces[index];
		let piece_duration = duration.try_sub(*time).unwrap();
		piece.evaluate(piece_duration)
	}

	fn control(self) -> Self::Control {
		self
			.pieces
			.into_iter()
			.map(|(t, p)| (t, p.control()))
			.collect()
	}

	fn from_state_and_control(state: Self::State, control: Self::Control) -> Self {
		let mut pieces = Vec::with_capacity(control.len());
		control.into_iter().fold(
			(Duration::default(), state),
			|(last_time, last_state), (time, control)| {
				let piece = Piece::from_state_and_control(last_state, control);
				let state = piece.evaluate(time.try_sub(last_time).unwrap());
				pieces.push((time, piece));
				(time, state)
			},
		);
		Self { pieces }
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use approx::assert_abs_diff_eq;

	#[test]
	fn test_quadratic() -> anyhow::Result<()> {
		let quadratic = Quadratic::from_state_and_control(QuadraticState::new(0.0, 1.0), -0.5);

		let s = quadratic.evaluate(0.0.try_into()?);
		assert_abs_diff_eq!(*s.position(), 0.0);
		assert_abs_diff_eq!(*s.velocity(), 1.0);

		let s = quadratic.evaluate(1.0.try_into()?);
		assert_abs_diff_eq!(*s.position(), 0.75);
		assert_abs_diff_eq!(*s.velocity(), 0.5);

		let s = quadratic.evaluate(2.0.try_into()?);
		assert_abs_diff_eq!(*s.position(), 1.0);
		assert_abs_diff_eq!(*s.velocity(), 0.0);

		Ok(())
	}

	#[test]
	fn test_cubic() {
		let p_initial = QuadraticState::new(1.0, 2.0);
		let p_final = QuadraticState::new(3.0, 4.0);
		let t_final: Duration = 5.0.try_into().unwrap();
		let trajectory = Cubic::interpolate(p_initial, p_final, t_final.get());

		let evaluated_p_initial = trajectory.evaluate(Duration::default());
		let evaluated_p_final = trajectory.evaluate(t_final);
		assert_abs_diff_eq!(
			evaluated_p_initial.position(),
			p_initial.position(),
			epsilon = 1e-8
		);
		assert_abs_diff_eq!(
			evaluated_p_initial.velocity(),
			p_initial.velocity(),
			epsilon = 1e-8
		);
		assert_abs_diff_eq!(
			evaluated_p_final.position(),
			p_final.position(),
			epsilon = 1e-8
		);
		assert_abs_diff_eq!(
			evaluated_p_final.velocity(),
			p_final.velocity(),
			epsilon = 1e-8
		);
	}

	#[test]
	fn test_spline() {
		let mut spline = Spline::new(Linear::constant(1.0));
		spline.add_control(Duration::clamp(2.0), 1.0).unwrap();
		spline.add_control(Duration::clamp(4.0), -1.0).unwrap();
		assert_eq!(spline.evaluate(Duration::clamp(1.0)), 1.0);
		assert_eq!(spline.evaluate(Duration::clamp(3.0)), 2.0);
		assert_eq!(spline.evaluate(Duration::clamp(5.0)), 2.0);
	}
}
