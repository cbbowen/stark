use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

type Scalar = f32;

pub trait VectorSpace:
	Sized
	+ Clone
	+ Copy
	+ Add<Self, Output = Self>
	+ Sub<Self, Output = Self>
	+ Mul<Scalar, Output = Self>
	+ Div<Scalar, Output = Self>
{
	fn lerp(self, other: Self, t: Scalar) -> Self {
		self + (other - self) * t
	}
}
impl<Vector> VectorSpace for Vector where
	Vector: Sized
		+ Clone
		+ Copy
		+ Add<Self, Output = Self>
		+ Sub<Self, Output = Self>
		+ Mul<Scalar, Output = Self>
		+ Div<Scalar, Output = Self>
{
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Product<A, B>(A, B);

impl<A: Add<A, Output = A>, B: Add<B, Output = B>> Add for Product<A, B> {
	type Output = Self;
	fn add(self, rhs: Self) -> Self::Output {
		Product(self.0 + rhs.0, self.1 + rhs.1)
	}
}

impl<A: AddAssign<A>, B: AddAssign<B>> AddAssign for Product<A, B> {
	fn add_assign(&mut self, rhs: Self) {
		self.0 += rhs.0;
		self.1 += rhs.1;
	}
}

impl<A: Sub<A, Output = A>, B: Sub<B, Output = B>> Sub for Product<A, B> {
	type Output = Self;
	fn sub(self, rhs: Self) -> Self::Output {
		Product(self.0 - rhs.0, self.1 - rhs.1)
	}
}

impl<A: SubAssign<A>, B: SubAssign<B>> SubAssign for Product<A, B> {
	fn sub_assign(&mut self, rhs: Self) {
		self.0 -= rhs.0;
		self.1 -= rhs.1;
	}
}

impl<A: Neg<Output = A>, B: Neg<Output = B>> Neg for Product<A, B> {
	type Output = Self;
	fn neg(self) -> Self::Output {
		Product(-self.0, -self.1)
	}
}

impl<Scalar: Clone, A: Mul<Scalar, Output = A>, B: Mul<Scalar, Output = B>> Mul<Scalar>
	for Product<A, B>
{
	type Output = Self;
	fn mul(self, rhs: Scalar) -> Self::Output {
		Product(self.0 * rhs.clone(), self.1 * rhs)
	}
}

impl<Scalar: Clone, A: MulAssign<Scalar>, B: MulAssign<Scalar>> MulAssign<Scalar>
	for Product<A, B>
{
	fn mul_assign(&mut self, rhs: Scalar) {
		self.0 *= rhs.clone();
		self.1 *= rhs;
	}
}

impl<Scalar: Clone, A: Div<Scalar, Output = A>, B: Div<Scalar, Output = B>> Div<Scalar>
	for Product<A, B>
{
	type Output = Self;
	fn div(self, rhs: Scalar) -> Self::Output {
		Product(self.0 / rhs.clone(), self.1 / rhs)
	}
}

impl<Scalar: Clone, A: DivAssign<Scalar>, B: DivAssign<Scalar>> DivAssign<Scalar>
	for Product<A, B>
{
	fn div_assign(&mut self, rhs: Scalar) {
		self.0 /= rhs.clone();
		self.1 /= rhs;
	}
}
