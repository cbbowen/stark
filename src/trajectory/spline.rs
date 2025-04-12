use super::*;

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

	pub fn num_pieces(&self) -> usize {
		self.pieces.len()
	}

	// Returns the index of the piece that contains the given time.
	pub fn time_to_index(&self, time: Duration) -> usize {
		self.pieces[1..].partition_point(|(t, _)| t <= &time)
	}

	fn time_to_piece_info(&self, time: Duration) -> (usize, Duration, &Piece) {
		let index = self.time_to_index(time);
		let (piece_time, piece) = &self.pieces[index];
		(index, *piece_time, piece)
	}

	pub fn time_to_index_and_duration(&self, time: Duration) -> (usize, Duration) {
		let (index, piece_time, _) = self.time_to_piece_info(time);
		(index, time.try_sub(piece_time).unwrap())
	}

	pub fn try_into_piece(self) -> std::result::Result<Piece, Self> {
		if self.pieces.len() == 1 {
			return Ok(self.pieces.into_iter().next().unwrap().1);
		}
		Err(self)
	}

	// Reduces the memory footprint of the spline without changing the result up to and including
	// `time`.
	//
	// Returns the index of the last piece, the duration on it of `time`, and the piece itself.
	pub fn trim(&mut self, time: Duration) -> (usize, Duration, &Piece) {
		let index = self.time_to_index(time);
		self.pieces.truncate(index + 1);
		let (last_time, last_piece) = self.last_time_and_piece();
		(index, time.try_sub(last_time).unwrap(), last_piece)
	}

	pub fn last_time_and_piece(&self) -> (Duration, &Piece) {
		// This is guaranteed to succeed because `pieces` is non-empty.
		let (time, piece) = self.pieces.last().unwrap();
		(*time, piece)
	}
}

impl<Piece: Trajectory> Spline<Piece> {
	// Removes all pieces that start at or after `time`.
	//
	// This function can leave the spline in an invalid state. The caller is responsible for
	// reestablishing the non-empty invariant on `pieces`.
	//
	// Returns the index of the piece at `time` and the duration on it of `time`, if that piece was
	// not removed.
	fn trim_internal(&mut self, time: Duration) -> (Option<(usize, Duration)>, Piece::State) {
		let (index, duration, piece) = self.trim(time);
		if duration.is_zero() {
			(None, self.pieces.pop().unwrap().1.initial_state())
		} else {
			(Some((index, duration)), piece.evaluate(duration))
		}
	}

	// Sets the control at and after the given time.
	pub fn set_control_after(
		&mut self,
		time: Duration,
		control: Piece::Control,
	) -> Option<(usize, Duration)> {
		let (index_and_duration, state) = self.trim_internal(time);
		self
			.pieces
			.push((time, Piece::from_state_and_control(state, control)));
		index_and_duration
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

	fn into_control(self) -> Self::Control {
		self
			.pieces
			.into_iter()
			.map(|(t, p)| (t, p.into_control()))
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

	fn into_tail(self, time: Duration) -> Self {
		let (index, duration) = self.time_to_index_and_duration(time);
		let mut iter = self.pieces.into_iter().skip(index);
		let head = (Duration::ZERO, iter.next().unwrap().1.into_tail(duration));
		let mut pieces = vec![head];
		pieces.extend(iter.map(|(t, p)| (t.try_sub(time).unwrap(), p)));
		Self { pieces }
	}
}

impl<X: Default + Clone + Add<Output = X> + Sub<Output = X> + Mul<f64, Output = X>>
	Spline<Linear<X>>
{
	pub fn interpolate(points: impl IntoIterator<Item = (f64, X)>) -> Option<(f64, Self)> {
		let mut points = points.into_iter();
		let (t0, x0) = points.next()?;
		let mut prev = (Duration::ZERO, x0.clone());
		let mut spline = Self::new(Linear::constant(x0));
		while let Some((t, x)) = points.next() {
			let Ok(d) = (t - t0).try_into() else { continue };
			let (d_prev, x_prev) = std::mem::replace(&mut prev, (d, x.clone()));
			let diff = d - d_prev;
			if diff <= 0.0 {
				continue;
			}
			let control = Linear::interpolate(x_prev, x, diff).into_control();
			spline.set_control_after(d, control);
		}
		Some((t0, spline))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_set_control_after() -> anyhow::Result<()> {
		let mut spline = Spline::new(Linear::constant(0.0));
		assert_eq!(spline.num_pieces(), 1);
		assert!(spline.set_control_after(1.0.try_into()?, 0.0).is_some());
		assert_eq!(spline.num_pieces(), 2);
		assert!(spline.set_control_after(3.0.try_into()?, 0.0).is_some());
		assert_eq!(spline.num_pieces(), 3);
		assert!(spline.set_control_after(2.0.try_into()?, 0.0).is_some());
		assert_eq!(spline.num_pieces(), 3);
		assert_eq!(spline.set_control_after(2.0.try_into()?, 0.0), None);
		assert_eq!(spline.num_pieces(), 3);
		assert_eq!(spline.set_control_after(1.0.try_into()?, 0.0), None);
		assert_eq!(spline.num_pieces(), 2);
		assert_eq!(spline.set_control_after(0.0.try_into()?, 0.0), None);
		assert_eq!(spline.num_pieces(), 1);
		Ok(())
	}

	#[test]
	fn test_spline() -> anyhow::Result<()> {
		let mut spline = Spline::new(Linear::constant(1.0));
		spline.set_control_after(2.0.try_into()?, 1.0);
		spline.set_control_after(4.0.try_into()?, -1.0);
		assert_eq!(spline.evaluate(1.0.try_into()?), 1.0);
		assert_eq!(spline.evaluate(3.0.try_into()?), 2.0);
		assert_eq!(spline.evaluate(5.0.try_into()?), 2.0);
		Ok(())
	}

	#[test]
	fn test_into_tail() -> anyhow::Result<()> {
		let mut spline = Spline::new(Linear::constant(1.0));
		spline.set_control_after(2.0.try_into()?, 1.0);
		spline.set_control_after(4.0.try_into()?, -1.0);
		let offset = 3.0.try_into()?;
		let tail = spline.clone().into_tail(offset);

		let t0 = 0.5.try_into()?;
		assert_eq!(tail.evaluate(t0), spline.evaluate(offset + t0));
		let t1 = 1.5.try_into()?;
		assert_eq!(tail.evaluate(t1), spline.evaluate(offset + t1));
		Ok(())
	}
}
