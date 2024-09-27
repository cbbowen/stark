// Implementation based on:
//
// Fast and accurate G^1 fitting of clothoid curves
// by Enrico Bertolazzi and Marco Frego
// University of Trento, Italy

use core::f32;
use std::fmt::Debug;
use glam::*;

#[derive(Clone, Copy)]
pub struct ClothoidState {
	position: Vec2,
	theta: f32,
	curvature: f32,
}

impl Debug for ClothoidState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}, θ = {}, κ = {}",
			self.position, self.theta, self.curvature
		)
	}
}

#[derive(Clone, Copy)]
pub struct Clothoid {
	initial: ClothoidState,
	pinch: f32,
	length: f32,
}

impl Debug for Clothoid {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{:?}, ρ = {}, s = {}",
			self.initial, self.pinch, self.length
		)
	}
}

pub fn normalize_angle(theta: f32) -> f32 {
	(theta + f32::consts::PI).rem_euclid(f32::consts::TAU) - f32::consts::PI
}

impl Clothoid {
	pub fn fit_g1(position0: Vec2, theta0: f32, position1: Vec2, theta1: f32, tol: f32) -> Self {
		let d = position1 - position0;
		let r = d.length();
		let phi = d.to_angle();
		let phi0 = normalize_angle(theta0 - phi);
		let phi1 = normalize_angle(theta1 - phi);
		let delta = phi1 - phi0;

		let mut rate = {
			let x = phi0 / f32::consts::PI;
			let y = phi1 / f32::consts::PI;
			let xy = x * y;
			let xx = x * x;
			let yy = y * y;
			(phi0 + phi1)
				* (2.989696028701907
					+ xy * (0.716228953608281 + xy * -0.458969738821509)
					+ (-0.502821153340377 + xy * 0.261062141752652) * (xx + yy)
					+ -0.045854475238709 * (xx * xx + yy * yy))
		};

		for _ in 0..11 {
			let vs = fresnel_moments::<3>(2.0 * rate, delta - rate, phi0);
			let g = vs[0].y;
			let dg = vs[2].x - vs[1].x;
			rate -= g / (dg + dg.signum() * tol * tol);
			if g.abs() < tol * (vs[0].x).min(1.0) {
				break;
			}
		}

		let [v] = fresnel_moments::<1>(2.0 * rate, delta - rate, phi0);

		debug_assert!(v.y.abs() <= tol, "error |{}| > {tol}", v.y);
		let length = r / v.x;

		debug_assert!(length > 0.0, "length {length} negative");
		let inv_length = length.recip();
		let kappa = (delta - rate) * inv_length;
		let pinch = 2.0 * rate * inv_length * inv_length;

		Self {
			initial: ClothoidState {
				position: position0,
				theta: theta0,
				curvature: kappa,
			},
			pinch,
			length,
		}
	}

	pub fn sample(&self, max_error: f32) -> Vec<ClothoidState> {
		let mut result = Vec::new();
		let mut state = self.initial;
		let mut s = 0f32;
		result.push(state);
		while s < self.length {
			let max_step = self.length - s;
			// |step * (curvature + step * pinch / 2)| < max_error
			// step * (curvature + step * pinch / 2) - max_error
			// step * (curvature + step * pinch / 2) + max_error
			let max_step = least_positive_quadratic_solution(self.pinch / 2.0, state.curvature, max_error, max_step);
			let max_step = least_positive_quadratic_solution(self.pinch / 2.0, state.curvature, -max_error, max_step);
			debug_assert!(max_step > 0.0);

			s = (s + max_step).max(s + 1e-2 * max_error).min(self.length);
			state = self.evaluate(s);
			result.push(state);
		}
		result
	}

	pub fn evaluate(&self, s: f32) -> ClothoidState {
		let sp = s * self.pinch;
		let [v] = fresnel_moments::<1>(sp * s, self.initial.curvature * s, self.initial.theta);
		ClothoidState {
			position: self.initial.position + s * v,
			theta: self.initial.theta + s * (self.initial.curvature + 0.5 * sp),
			curvature: self.initial.curvature + sp,
		}
	}
}

// Unstable near c == 0.
fn least_positive_quadratic_solution(mut a: f32, mut b: f32, mut c: f32, max: f32) -> f32 {
	debug_assert!(c != 0.0);
	debug_assert!(max > 0.0);
	if c < 0.0 {
		a = -a;
		b = -b;
		c = -c;
	}
	let numerator = 2.0 * c;
	let determinant = b * b - 4.0 * a * c;
	if determinant < 0.0 {
		return max;
	}
	let denominator = -b + determinant.sqrt();
	if denominator <= 0.0 || max * denominator <= numerator {
		return max;
	}
	return numerator / denominator;
}

pub fn fresnel_cs(t: f32) -> Vec2 {
	let (s, c) = fresnel::fresnl(t as f64);
	vec2(c as f32, s as f32)
}

struct If<const C: bool>;
trait True {}
impl True for If<true> {}

fn fresnel_moments_t<const K: usize>(t: f32) -> [Vec2; K]
where
	If<{ K <= 3 }>: True,
{
	let mut moments = [Vec2::ZERO; K];
	moments[0] = fresnel_cs(t);
	if K > 1 {
		let p = Vec2::from_angle(f32::consts::PI * t * t / 2.0).perp();
		moments[1] = (Vec2::Y - p) / f32::consts::PI;
		if K > 2 {
			moments[2] = (moments[0].perp() - t * p) / f32::consts::PI;
		}
	}
	moments
}

fn fresnel_moments_large_a<const K: usize>(a: f32, b: f32) -> [Vec2; K]
where
	If<{ K <= 3 }>: True,
{
	let m_1_sqrt_pi = 1.0 / f32::consts::PI.sqrt();

	let s = a.signum();
	let absa = a.abs();
	let z = m_1_sqrt_pi * absa.sqrt();
	let inv_z = z.recip();
	let ell = s * b * m_1_sqrt_pi / absa.sqrt();
	let g = -0.5 * s * b * b / absa;

	let mut vg = Vec2::from_angle(g) * inv_z;

	let minus = fresnel_moments_t::<K>(ell);
	let plus = fresnel_moments_t::<K>(ell + z);

	let mut result = [Vec2::ZERO; K];
	let d0 = vec2(1.0, s) * (plus[0] - minus[0]);
	result[0] = d0.rotate(vg);
	if K > 1 {
		vg *= inv_z;
		let d1 = vec2(1.0, s) * (plus[1] - minus[1]);
		result[1] = (d1 - ell * d0).rotate(vg);
		if K > 2 {
			vg *= inv_z;
			let d2 = vec2(1.0, s) * (plus[2] - minus[2]);
			result[2] = (d2 + ell * (ell * d0 - 2.0 * d1)).rotate(vg);
		}
	}
	result
}

fn lommel(mu: f32, nu: f32, b: f32) -> f32 {
	let mut term = 1.0 / ((mu + nu + 1.0) * (mu - nu + 1.0));
	let mut sum = term;
	for n in 1..=100 {
		term *= (-b / (2.0 * n as f32 + mu - nu + 1.0)) * (b / (2.0 * n as f32 + mu + nu + 1.0));
		sum += term;
		if term.abs() < sum.abs() * (8.0 * f32::EPSILON) {
			break;
		}
	}
	return sum;
}

fn fresnel_moments_zero_a<const K: usize>(b: f32) -> [Vec2; K] {
	let pb = Vec2::from_angle(b).perp();
	let b2 = b * b;

	let mut result = [Vec2::ZERO; K];
	result[0] = if b.abs() < 1e-3 {
		vec2(
			1.0 - (b2 / 6.0) * (1.0 - (b2 / 20.0) * (1.0 - (b2 / 42.0))),
			(b / 2.0) * (1.0 - (b2 / 12.0) * (1.0 - (b2 / 30.0))),
		)
	} else {
		(Vec2::Y - pb) / b
	};

	let m = (2.0 * b).floor().min((K - 1) as f32).max(1.0) as usize;
	for k in 1..m {
		let v = result[k - 1];
		result[k] = (k as f32 * v.perp() - pb) / b;
	}
	if m < K {
		let a0 = -pb.x - b * pb.y;
		let a1 = b * -pb.x;
		let mba0 = -b * a0;
		let mba1 = -b * a1;
		let mut lommel_m_0 = lommel(m as f32 + 0.5, 0.5, b);
		let mut lommel_m_1 = lommel(m as f32 + 0.5, 1.5, b);
		for k in m..K {
			let lommel_k_0 = lommel(k as f32 + 1.5, 0.5, b);
			let lommel_k_1 = lommel(k as f32 + 1.5, 1.5, b);
			result[k] = vec2(
				(k as f32 * a1 * lommel_m_1 - mba0 * lommel_k_0 + pb.y) / (1 + k) as f32,
				(mba1 * lommel_k_1 - pb.x) / (2 + k) as f32 + a0 * lommel_m_0,
			);
			lommel_m_1 = lommel_k_1;
			lommel_m_0 = lommel_k_0;
		}
	}
	result
}

const SMALL_A_SERIES_SIZE: usize = 3;

fn fresnel_moments_small_a<const K: usize>(a: f32, b: f32) -> [Vec2; K]
where
	If<{ K <= 3 }>: True,
	[(); K + 4 * SMALL_A_SERIES_SIZE + 2]: Sized,
{
	let intermediate = fresnel_moments_zero_a::<{ K + 4 * SMALL_A_SERIES_SIZE + 2 }>(b);

	let mut result = [Vec2::ZERO; K];
	for j in 0..K {
		result[j] = intermediate[j] + (a / 2.0) * intermediate[j + 2].perp();
	}

	let mut t = 1.0;
	let aa = -a * a / 4.0; // controllare!
	for n in 1..=SMALL_A_SERIES_SIZE {
		t *= aa / (2 * n * (2 * n - 1)) as f32;
		let bf = a / (4 * n + 2) as f32;
		for j in 0..K {
			let jj = 4 * n + j;
			result[j] += t * (intermediate[jj] + bf * intermediate[jj + 2].perp());
		}
	}
	result
}

fn fresnel_moments<const K: usize>(a: f32, b: f32, c: f32) -> [Vec2; K]
where
	If<{ K <= 3 }>: True,
	[(); K + 4 * SMALL_A_SERIES_SIZE + 2]: Sized,
{
	let rotated_moments = if a.abs() < 1e-2 {
		fresnel_moments_small_a::<K>(a, b)
	} else {
		fresnel_moments_large_a::<K>(a, b)
	};

	let cv = Vec2::from_angle(c);

	let mut result = [Vec2::ZERO; K];
	for k in 0..K {
		result[k] = rotated_moments[k].rotate(cv);
	}
	result
}

#[cfg(test)]
mod tests {
	use super::*;

	fn test_fit_g1_case(x0: f32, y0: f32, theta0: f32, x1: f32, y1: f32, theta1: f32, tol: f32) {
		let curve = Clothoid::fit_g1(vec2(x0, y0), theta0, vec2(x1, y1), theta1, tol);

		let s0 = curve.evaluate(0.0);
		assert!((s0.position.x - x0).abs() < tol, "({s0:?}).x != {x0}");
		assert!((s0.position.y - y0).abs() < tol, "({s0:?}).y != {y0}");
		assert!(
			normalize_angle(s0.theta - theta0).abs() < tol,
			"({s0:?}).theta != {theta0}"
		);

		let s1 = curve.evaluate(curve.length);
		assert!((s1.position.x - x1).abs() < tol, "({s1:?}).x vs {x1}");
		assert!((s1.position.y - y1).abs() < tol, "({s1:?}).y vs {y1}");
		assert!(
			normalize_angle(s1.theta - theta1).abs() < tol,
			"({s1:?}).theta vs {theta1}"
		);
	}

	#[test]
	fn test_fresnel() {
		assert_eq!(fresnel_cs(0.0), vec2(0.0, 0.0));
		assert_eq!(fresnel_cs(0.5), vec2(0.49234423, 0.06473243));
		assert_eq!(fresnel_cs(1.0), vec2(0.77989340, 0.43825915));
		assert_eq!(fresnel_cs(1.5), vec2(0.44526118, 0.69750496));
		assert_eq!(fresnel_cs(2.0), vec2(0.48825341, 0.34341568));
		assert_eq!(fresnel_cs(2.5), vec2(0.45741301, 0.61918176));
	}

	#[test]
	fn test_fit_g1_basic() {
		let tol = 1e-5;
		test_fit_g1_case(0.0, 0.0, 0.0, 1.0, 0.0, 0.0, tol);
		test_fit_g1_case(0.0, 0.0, 0.0, 1.0, 1.0, 0.0, tol);
		test_fit_g1_case(0.0, 0.0, 0.0, 1.0, -1.0, 0.0, tol);
		test_fit_g1_case(0.0, 0.0, 0.0, 1.0, 1.0, f32::consts::PI / 2.0, tol);
		test_fit_g1_case(0.0, 0.0, 0.0, 1.0, 1.0, 0.0, tol);
		test_fit_g1_case(1.0, 0.0, 0.0, 1.0, 1.0, 1.0, tol);
		test_fit_g1_case(1.0, 0.0, 0.0, 1.0, -1.0, 1.0, tol);
		test_fit_g1_case(1.0, 0.0, 1.0, 1.0, -2.0, 1.0, tol);
	}

	#[test]
	fn test_fit_g1_proofed() {
		fastrand::seed(0x13371337);

		let tol = 1e-5;
		for _ in 0..100 {
			let length = 10.0 * fastrand::f32();
			let proof = Clothoid {
				initial: ClothoidState {
					position: 100.0 * (vec2(fastrand::f32(), fastrand::f32()) - 0.5),
					theta: 10.0 * (fastrand::f32() - 0.5),
					curvature: (fastrand::f32() - 0.5) / length.max(1.0),
				},
				pinch: (fastrand::f32() - 0.5) / (length * length).max(1.0),
				length,
			};
			let s0 = proof.evaluate(0.0);
			let s1 = proof.evaluate(proof.length);
			test_fit_g1_case(s0.position.x, s0.position.y, s0.theta, s1.position.x, s1.position.y, s1.theta, tol);
		}
	}

	#[test]
	fn test_fit_g1_random() {
		fastrand::seed(0x13371337);

		let tol = 1e-3;
		for _ in 0..100 {
			test_fit_g1_case(
				0.0, 0.0, f32::consts::TAU * (fastrand::f32() - 0.5), 
				100.0 * (fastrand::f32() - 0.5), 100.0 * (fastrand::f32() - 0.5), f32::consts::TAU * (fastrand::f32() - 0.5), tol);
		}
	}

	#[test]
	fn sample() {
		let clothoid = Clothoid {
			initial: ClothoidState {
				position: Vec2::ZERO,
				theta: 0.0,
				curvature: 1.0,
			},
			pinch: -0.5,
			length: 4.0,
		};
		println!("{:?}", clothoid.sample(0.25));
	}
}
