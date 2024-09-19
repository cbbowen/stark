mod resources;
use std::{borrow::Borrow, mem::MaybeUninit, num::NonZero, ops::Deref};

use bon::{bon, builder};
pub use resources::*;
use thiserror::Error;
use wgpu::util::DeviceExt;

#[derive(Debug)]
pub struct Shader {
	pub module: wgpu::ShaderModule,
	pub layout: wgpu::PipelineLayout,
}

#[builder(finish_fn = create)]
pub fn render_pipeline<'a>(
	#[builder(finish_fn)] device: &wgpu::Device,
	label: Option<&str>,
	layout: Option<&wgpu::PipelineLayout>,
	vertex: wgpu::VertexState<'a>,
	fragment: Option<wgpu::FragmentState<'a>>,
	depth_stencil: Option<wgpu::DepthStencilState>,
	#[builder(default)] multisample: wgpu::MultisampleState,
	multiview: Option<NonZero<u32>>,
	cache: Option<&wgpu::PipelineCache>,
) -> wgpu::RenderPipeline {
	device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label,
		layout,
		vertex,
		fragment,
		primitive: wgpu::PrimitiveState {
			topology: wgpu::PrimitiveTopology::TriangleStrip,
			strip_index_format: None,
			front_face: wgpu::FrontFace::Ccw,
			cull_mode: None,
			polygon_mode: wgpu::PolygonMode::Fill,
			unclipped_depth: false,
			conservative: false,
		},
		depth_stencil,
		multisample,
		multiview,
		cache,
	})
}

#[derive(Debug, Error)]
enum TextureError {
	#[error("texture cannot have both depth and array layers")]
	DepthAndArrayLayers,
	#[error("textures with depth must have height")]
	MissingHeight,
}

fn to_extent_and_dimension(
	width: u32,
	height: Option<u32>,
	depth: Option<u32>,
	array_layers: Option<u32>,
) -> Result<(wgpu::Extent3d, wgpu::TextureDimension), TextureError> {
	use TextureError::*;
	let dimension = if depth.is_some() {
		wgpu::TextureDimension::D3
	} else if height.is_some() {
		wgpu::TextureDimension::D2
	} else {
		wgpu::TextureDimension::D1
	};
	if dimension == wgpu::TextureDimension::D3 {
		if array_layers.is_some() {
			Err(DepthAndArrayLayers)?;
		}
		if height.is_none() {
			Err(MissingHeight)?;
		}
	}
	let depth_or_array_layers = depth.or(array_layers).unwrap_or(1);
	let height = height.unwrap_or(1);
	Ok((
		wgpu::Extent3d {
			width,
			height,
			depth_or_array_layers,
		},
		dimension,
	))
}

#[builder(finish_fn = create)]
pub fn texture(
	#[builder(finish_fn)] device: &wgpu::Device,
	label: Option<&str>,
	width: u32,
	height: Option<u32>,
	depth: Option<u32>,
	array_layers: Option<u32>,
	#[builder(default = 1)] mip_level_count: u32,
	#[builder(default = 1)] sample_count: u32,
	#[builder(default = wgpu::TextureUsages::all())] usage: wgpu::TextureUsages,
	format: wgpu::TextureFormat,
	#[builder(default = &[])] view_formats: &[wgpu::TextureFormat],
	with_data: Option<(&wgpu::Queue, &[u8])>,
) -> wgpu::Texture {
	let (size, dimension) = to_extent_and_dimension(width, height, depth, array_layers).unwrap();
	let descriptor = wgpu::TextureDescriptor {
		label,
		size,
		mip_level_count,
		sample_count,
		usage,
		dimension,
		format,
		view_formats,
	};
	if let Some((queue, data)) = with_data {
		device.create_texture_with_data(queue, &descriptor, Default::default(), data)
	} else {
		device.create_texture(&descriptor)
	}
}

/// Thin wrapper around a `wgpu::Buffer` that stores the a type `T` in a format suitable for binding
/// to a uniform or storage.
pub struct BindingBuffer<T: ?Sized> {
	buffer: wgpu::Buffer,
	_t: std::marker::PhantomData<T>,
}

impl<T: ?Sized> Deref for BindingBuffer<T> {
	type Target = wgpu::Buffer;
	fn deref(&self) -> &Self::Target {
		&self.buffer
	}
}

impl<T: ?Sized> BindingBuffer<T> {
	/// Returns the underlying `wgpu::Buffer`.
	pub fn into_raw(self) -> wgpu::Buffer {
		self.buffer
	}

	fn from_buffer(buffer: wgpu::Buffer) -> Self {
		Self {
			buffer,
			_t: Default::default(),
		}
	}

	fn default_usages() -> wgpu::BufferUsages {
		wgpu::BufferUsages::COPY_DST
			| wgpu::BufferUsages::COPY_SRC
			| wgpu::BufferUsages::STORAGE
			| wgpu::BufferUsages::UNIFORM
	}
}

#[bon]
impl<T: ?Sized + encase::ShaderType + encase::internal::WriteInto> BindingBuffer<T> {
	fn value_to_data(values: &T) -> impl Borrow<[u8]> {
		let size = T::size(values).get() as usize;
		let data = Vec::<u8>::with_capacity(size);
		let mut data = encase::StorageBuffer::new(data);
		data.write(values).unwrap();
		data.into_inner()
	}

	/// Builds a buffer with the given initial `value`.
	#[builder(finish_fn = "create")]
	pub fn init<'a>(
		#[builder(start_fn)] value: &T,
		#[builder(finish_fn)] device: &'a wgpu::Device,
		label: Option<&str>,
		usage: Option<wgpu::BufferUsages>,
	) -> Self {
		let usage = usage.unwrap_or(Self::default_usages());
		let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label,
			contents: Self::value_to_data(value).borrow(),
			usage,
		});
		Self::from_buffer(buffer)
	}

	/// Writes the given `value` to the buffer.
	pub fn write(&self, queue: &wgpu::Queue, value: impl Borrow<T>) {
		queue.write_buffer(
			&self.buffer,
			0,
			Self::value_to_data(value.borrow()).borrow(),
		)
	}
}

#[bon]
impl<T: encase::ShaderSize + encase::internal::WriteInto> BindingBuffer<T>
where
	[u8; T::SHADER_SIZE.get() as usize]: Sized,
{
	fn sized_value_to_data(value: &T) -> impl Borrow<[u8]> {
		let data = MaybeUninit::uninit_array::<{ T::SHADER_SIZE.get() as usize }>();
		let mut data = encase::StorageBuffer::new(data);
		data.write(value).unwrap();
		let data = data.into_inner();

		// SAFETY: The guarantee of `encase::ShaderSize` is that `write` writes exactly that many
		// bytes.
		unsafe { MaybeUninit::array_assume_init(data) }
	}

	/// Builds an uninitialized buffer for a type which implements `encase::ShaderSize`.
	#[builder(finish_fn = "create")]
	pub fn new_sized(
		#[builder(finish_fn)] device: &wgpu::Device,
		label: Option<&str>,
		usage: Option<wgpu::BufferUsages>,
		#[builder(default)] mapped_at_creation: bool,
	) -> Self {
		let usage = usage.unwrap_or(Self::default_usages());
		let buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label,
			size: T::SHADER_SIZE.get(),
			usage,
			mapped_at_creation,
		});
		Self::from_buffer(buffer)
	}

	/// Builds a buffer with the given initial `value`. The value must implement
	/// `encase::ShaderSize`. Functionally, this is equivalent to `init` but avoids an extra
	/// allocation.
	#[builder(finish_fn = "create")]
	pub fn init_sized<'a>(
		#[builder(start_fn)] value: &T,
		#[builder(finish_fn)] device: &'a wgpu::Device,
		label: Option<&str>,
		usage: Option<wgpu::BufferUsages>,
	) -> Self {
		let usage = usage.unwrap_or(Self::default_usages());
		let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label,
			contents: Self::sized_value_to_data(value).borrow(),
			usage,
		});
		Self::from_buffer(buffer)
	}

	/// Writes the given `value` to the buffer. The value must implement `encase::ShaderSize`.
	/// Functionally, this is equivalent to `write` but avoids an extra allocation.
	pub fn write_sized(&self, queue: &wgpu::Queue, value: impl Borrow<T>) {
		queue.write_buffer(
			&self.buffer,
			0,
			Self::sized_value_to_data(value.borrow()).borrow(),
		)
	}
}

#[bon]
impl<T: ?Sized + encase::CalculateSizeFor> BindingBuffer<T> {
	/// Builds an uninitialized buffer for a type which implements `encase::CalculateSizeFor` with
	/// `capacity` elements.
	#[builder(finish_fn = "create")]
	pub fn with_capacity<'a>(
		#[builder(start_fn)] capacity: u64,
		#[builder(finish_fn)] device: &'a wgpu::Device,
		label: Option<&str>,
		usage: Option<wgpu::BufferUsages>,
		#[builder(default)] mapped_at_creation: bool,
	) -> Self {
		let usage = usage.unwrap_or(Self::default_usages());
		let buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label,
			size: T::calculate_size_for(capacity).get(),
			usage,
			mapped_at_creation,
		});
		Self::from_buffer(buffer)
	}
}

impl<T: ?Sized + encase::ShaderType + encase::CalculateSizeFor + encase::internal::WriteInto>
	BindingBuffer<T>
{
	pub fn raw_offset(offset: u64) -> u64 {
		// `calculate_size_for` has surprising semantics for zero-length arrays, so we have to
		// special-case the zero index.
		if offset == 0 {
			0
		} else {
			T::calculate_size_for(offset).get()
		}
	}

	// TODO: It would be nice to support `impl std::ops::RangeBounds<u64>`.
	pub fn slice(&self, range: std::ops::Range<u64>) -> wgpu::BufferSlice<'_> {
		self
			.buffer
			.slice(Self::raw_offset(range.start)..Self::raw_offset(range.end))
	}

	pub fn slice_binding(&self, range: std::ops::Range<u64>) -> wgpu::BufferBinding<'_> {
		let size = T::calculate_size_for(range.end - range.start);
		let offset = Self::raw_offset(range.start);
		wgpu::BufferBinding {
			buffer: &self.buffer,
			offset,
			size: Some(size),
		}
	}

	pub fn write_slice(&self, queue: &wgpu::Queue, offset: u64, values: impl Borrow<T>) {
		queue.write_buffer(
			&self.buffer,
			Self::raw_offset(offset),
			Self::value_to_data(values.borrow()).borrow(),
		)
	}
}
