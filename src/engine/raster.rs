use glam::{vec2, Vec2};
use itertools::Itertools;

fn floor_as_i32(x: f32) -> i32 {
	x.floor() as i32
}

pub fn max_line_along_x(p0: Vec2, p1: Vec2) -> impl Iterator<Item = i32> {
	let d = p1 - p0;
	debug_assert!(d.x >= 0f32);
	let slope = d.y / d.x;
	let intercept = p0.y - slope * p0.x;
	let xi0 = floor_as_i32(p0.x);
	let xi1 = floor_as_i32(p1.x);

	let positive = d.y >= 0f32;
	let (start, end) = if positive {
		(None, Some(floor_as_i32(p1.y)))
	} else {
		(Some(floor_as_i32(p0.y)), None)
	};
	start
		.into_iter()
		.chain((xi0 + 1..=xi1).map(move |xi| floor_as_i32(intercept + slope * xi as f32)))
		.chain(end)
}

pub fn min_line_along_x(p0: Vec2, p1: Vec2) -> impl Iterator<Item = i32> {
	max_line_along_x(vec2(p0.x, -p0.y), vec2(p1.x, -p1.y)).map(|yi| -(yi + 1))
}

pub fn conservative_wedge(a: Vec2, b: Vec2, c: Vec2) -> impl Iterator<Item = (i32, i32)> {
	debug_assert!(b.x >= a.x);
	debug_assert!(c.x >= b.x);
	(floor_as_i32(a.x)..)
		.zip(min_line_along_x(a, c))
		.zip(max_line_along_x(a, b))
		.flat_map(move |((x, y_min), y_max)| (y_min..=y_max).map(move |y| (x, y)))
}

fn conservative_clockwise_triangle(a: Vec2, b: Vec2, c: Vec2) -> impl Iterator<Item = (i32, i32)> {
	debug_assert!(b.x >= a.x);
	debug_assert!(c.x >= b.x);
	conservative_wedge(a, b, c).chain(
		conservative_wedge(vec2(-c.x, c.y), vec2(-b.x, b.y), vec2(-a.x, a.y))
			.map(|(x, y)| (-(x + 1), y)),
	)
}

pub fn conservative_triangle(a: Vec2, b: Vec2, c: Vec2) -> impl Iterator<Item = (i32, i32)> {
	let mut points = [a, b, c];
	points.sort_by(|a, b| a.x.total_cmp(&b.x));
	let [a, b, c] = points;
	let det = (c - a).perp_dot(b - a);
	let mut result: Vec<_> = if det >= 0f32 {
		conservative_clockwise_triangle(a, b, c).collect()
	} else {
		conservative_clockwise_triangle(vec2(a.x, -a.y), vec2(b.x, -b.y), vec2(c.x, -c.y))
			.map(|(x, y)| (x, -(y + 1)))
			.collect()
	};
	result.sort();
	result
		.into_iter()
		.coalesce(|a, b| if a == b { Ok(a) } else { Err((a, b)) })
}

#[cfg(test)]
mod tests {
	use super::*;
	use glam::vec2;
	use itertools::Itertools;

	#[test]
	fn test_max_positive_line_along_x() {
		assert_eq!(
			max_line_along_x(vec2(1.4, 2.7), vec2(1.4, 4.7)).collect_vec(),
			vec![4],
		);
		assert_eq!(
			max_line_along_x(vec2(1.6, 2.9), vec2(6.6, 2.9)).collect_vec(),
			vec![2, 2, 2, 2, 2, 2],
		);
		assert_eq!(
			max_line_along_x(vec2(1.8, 2.1), vec2(6.8, 5.2)).collect_vec(),
			vec![2, 2, 3, 4, 4, 5],
		);
		assert_eq!(
			max_line_along_x(vec2(1.4, 4.7), vec2(1.4, 2.7)).collect_vec(),
			vec![4],
		);
		assert_eq!(
			max_line_along_x(vec2(1.6, 2.9), vec2(6.6, 2.9)).collect_vec(),
			vec![2, 2, 2, 2, 2, 2],
		);
		assert_eq!(
			max_line_along_x(vec2(1.8, 5.2), vec2(6.8, 2.1)).collect_vec(),
			vec![5, 5, 4, 3, 3, 2],
		);
	}

	#[test]
	fn test_min_line_along_x() {
		assert_eq!(
			min_line_along_x(vec2(1.4, 2.7), vec2(1.4, 4.7)).collect_vec(),
			vec![2],
		);
		assert_eq!(
			min_line_along_x(vec2(1.6, 2.9), vec2(6.6, 2.9)).collect_vec(),
			vec![2, 2, 2, 2, 2, 2],
		);
		assert_eq!(
			min_line_along_x(vec2(1.8, 2.1), vec2(6.8, 5.2)).collect_vec(),
			vec![2, 2, 2, 3, 4, 4],
		);
		assert_eq!(
			min_line_along_x(vec2(1.4, 4.7), vec2(1.4, 2.7)).collect_vec(),
			vec![2],
		);
		assert_eq!(
			min_line_along_x(vec2(1.6, 2.9), vec2(6.6, 2.9)).collect_vec(),
			vec![2, 2, 2, 2, 2, 2],
		);
		assert_eq!(
			min_line_along_x(vec2(1.8, 5.2), vec2(6.8, 2.1)).collect_vec(),
			vec![5, 4, 3, 3, 2, 2],
		);
	}

	#[test]
	fn test_conservative_wedge() {
		println!(
			"{:?}",
			conservative_wedge(vec2(0.0, 0.0), vec2(3.0, 2.0), vec2(5.0, -1.0)).collect_vec()
		);
	}

	#[test]
	fn test_conservative_triangle() {
		println!(
			"{:?}",
			conservative_triangle(vec2(0.0, 0.0), vec2(3.0, 2.0), vec2(5.0, -1.0)).collect_vec()
		);
		println!(
			"{:?}",
			conservative_triangle(vec2(0.0, 0.0), vec2(3.0, -2.0), vec2(5.0, 1.0)).collect_vec()
		);
	}
}
