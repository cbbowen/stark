use std::ffi::OsStr;
use std::fmt::Write;
use std::path::Path;

use wgsl_to_wgpu::*;

fn generate_wgpu_bindings_for(
	input_path: impl AsRef<Path>,
	output_dir: impl AsRef<Path>,
	write_options: WriteOptions,
) {
	let input_path = input_path.as_ref();
	assert!(input_path.is_relative());

	let output_dir = output_dir.as_ref();
	assert!(output_dir.is_dir());

	let input_path_string = input_path.to_string_lossy();
	println!("cargo:rerun-if-changed={input_path_string}");
	let wgsl_source = std::fs::read_to_string(input_path).unwrap();

	let rs_source = &create_shader_module_embedded(&wgsl_source, write_options).unwrap();

	let output_path = output_dir.join(input_path.with_extension("rs"));
	std::fs::create_dir_all(output_path.parent().unwrap()).unwrap();

	println!("output_path = {}", output_path.to_string_lossy());
	std::fs::write( output_path, rs_source.as_bytes()).unwrap();
}

fn generate_wgpu_bindings() {
	let write_options = WriteOptions {
		derive_bytemuck_vertex: true,
		derive_encase_host_shareable: true,
		matrix_vector_types: MatrixVectorTypes::Glam,
		rustfmt: true,
		..Default::default()
	};

	let output_dir = Path::new(&std::env::var("OUT_DIR").unwrap()).join("generate_wgpu_bindings");
	if !output_dir.exists() {
		std::fs::create_dir_all(&output_dir).unwrap();
	}

	let input_dir = Path::new("src/shaders");
	for entry in std::fs::read_dir(input_dir).unwrap() {
		let Ok(entry) = entry else { continue; };
		if entry.path().extension() != Some(OsStr::new("wgsl")) { continue; }
		generate_wgpu_bindings_for(entry.path(), &output_dir, write_options.clone());
		// println!("{:?}", entry.path());
	}
	// panic!("foo");
	// generate_wgpu_bindings_for("src/shaders/canvas.wgsl", &output_dir, write_options.clone());
}

fn main() {
	generate_wgpu_bindings();
}
