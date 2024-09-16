mod resources;
use std::{borrow::Borrow, mem::MaybeUninit, num::NonZero, ops::Deref};

use bon::{bon, builder};
pub use resources::*;
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

/// Thin wrapper around a `wgpu::Buffer` that stores the a type `T` in a format suitable for binding
/// to a uniform or storage.
pub struct BindingBuffer<T: ?Sized> {
	buffer: wgpu::Buffer,
	_t: std::marker::PhantomData<T>,
}

impl<T> Deref for BindingBuffer<T> {
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
		let mut data = encase::UniformBuffer::new(data);
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
		let mut data = encase::UniformBuffer::new(data);
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
	pub fn write_slice(&self, queue: &wgpu::Queue, offset: u64, values: impl Borrow<T>) {
		let offset = if offset == 0 {
			0
		} else {
			T::calculate_size_for(offset).get()
		};
		queue.write_buffer(
			&self.buffer,
			offset,
			Self::value_to_data(values.borrow()).borrow(),
		)
	}
}
