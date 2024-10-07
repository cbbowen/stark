use std::ops::Mul;

#[doc(hidden)]
pub trait IsTrue {}

#[doc(hidden)]
pub struct If<const B: bool>;
impl IsTrue for If<true> {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PositionAndDerivatives<X, const N: usize>([X; N]);

impl<X, const N: usize> PositionAndDerivatives<X, N> {
	pub fn get(self) -> [X; N] {
		self.0
	}

	pub fn map_affine<Y>(self, f: impl Fn(X, f64) -> Y) -> PositionAndDerivatives<Y, N> {
		let mut w = 1.0;
		PositionAndDerivatives(self.0.map(move |x| {
			let y = f(x, w);
			w = 0.0;
			y
		}))
	}

	pub fn zip_affine<Y, Z>(
		self,
		other: PositionAndDerivatives<Y, N>,
		f: impl Fn(X, Y, f64) -> Z,
	) -> PositionAndDerivatives<Z, N> {
		let mut w = 1.0;
		let mut y_iter = other.get().into_iter();
		PositionAndDerivatives(self.0.map(move |x| {
			let y = y_iter.next().unwrap();
			let z = f(x, y, w);
			w = 0.0;
			z
		}))
	}
}

impl<X, const N: usize> Default for PositionAndDerivatives<X, N>
where
	[X; N]: Default,
{
	fn default() -> Self {
		Self(Default::default())
	}
}

impl<X, const N: usize> From<[X; N]> for PositionAndDerivatives<X, N> {
	fn from(s: [X; N]) -> Self {
		Self(s)
	}
}

impl<X> PositionAndDerivatives<X, 1> {
	pub fn new(position: X) -> Self {
		Self([position])
	}
}

impl<X> PositionAndDerivatives<X, 2> {
	pub fn new(position: X, velocity: X) -> Self {
		Self([position, velocity])
	}
}

impl<X> PositionAndDerivatives<X, 3> {
	pub fn new(position: X, velocity: X, acceleration: X) -> Self {
		Self([position, velocity, acceleration])
	}
}

impl<X, const N: usize> PositionAndDerivatives<X, N>
where
	If<{ N >= 1 }>: IsTrue,
{
	pub fn position(&self) -> &X {
		&self.0[0]
	}
}

impl<X, const N: usize> PositionAndDerivatives<X, N>
where
	If<{ N >= 2 }>: IsTrue,
{
	pub fn velocity(&self) -> &X {
		&self.0[1]
	}
}

impl<X, const N: usize> PositionAndDerivatives<X, N>
where
	X: Mul<f64>,
{
	pub fn domain_scaled(self, scale: f64) -> PositionAndDerivatives<X::Output, N> {
		let mut factor = 1.0;
		PositionAndDerivatives(self.0.map(move |x| {
			let y = x * factor;
			factor *= scale;
			y
		}))
	}
}

pub type QuadraticState<X> = PositionAndDerivatives<X, 2>;

pub type CubicState<X> = PositionAndDerivatives<X, 3>;
