fn dither(co: vec2<f32>) -> f32 {
	let a = sin(dot(co.xy, vec2(12.9898, 78.233)));
	return fract(a * 43758.5453) - 0.5;
}

fn dither2(co: vec2<f32>) -> vec2<f32> {
	let a = sin(dot(co.xy, vec2(12.9898, 78.233)));
	let b = (co.xy + vec2(43758.5453, 29443.5016));
	return fract(a * b) - 0.5;
}

fn dither3(co: vec2<f32>) -> vec3<f32> {
	let a = vec3(dither(co), dither2(co));
	return fract(a * vec3(43758.5453, 29443.5016, 57984.4129)) - 0.5;
}
