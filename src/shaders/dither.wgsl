fn dither1(co: vec2<f32>) -> f32 {
	let A = vec2(63.6574, -41.2570);
	let B = vec3(-57.4096, 62.2291, 10448.7115);
	let a = sin(dot(A, co));
	let b = dot(B, vec3(co, 1.0));
	return fract(a * b);
}

fn dither2(co: vec2<f32>) -> vec2<f32> {
	let A = mat2x2(76.7223, -38.3171,
	               93.1957, 114.9507);
	let B = mat3x2(96.5832, 18.5166, 17033.1114,
	               61.2907, -114.9439, 62222.1600);
	let a = sin(A * co);
	let b = B * vec3(co, 1.0);
	return fract(a * b);
}

fn dither3(co: vec2<f32>) -> vec3<f32> {
	let A = mat2x3(6.3841, 91.6403,
	               75.5221, 17.0909,
						-97.6091, 20.9826);
	let B = mat3x3(-66.3084, -73.0280, 50941.4172,
	               -113.8825, 26.8574, 13387.8386,
						118.5056, 13.7056, 20853.5495);
	let a = sin(A * co);
	let b = B * vec3(co, 1.0);
	return fract(a * b);
}
