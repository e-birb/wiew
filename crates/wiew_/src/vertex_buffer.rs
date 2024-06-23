use std::{marker::PhantomData, ops::RangeBounds, sync::Arc};

use wgpu::util::DeviceExt;

pub mod instance;

pub trait VertexRawRepr: bytemuck::Pod {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

pub struct VertexBuffer<T: VertexRawRepr> {
    buffer: Arc<wgpu::Buffer>,
    len: u32,
    _phantom: PhantomData<T>,
}

impl<T: VertexRawRepr> VertexBuffer<T>
{
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn single(device: &wgpu::Device, instance: T, label: Option<&'static str>) -> Self {
        Self::from_slice(device, &[instance], label)
    }

    pub fn from_iter<I>(device: &wgpu::Device, iter: I, label: Option<&'static str>) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let instances: Vec<T> = iter.into_iter().collect();
        Self::from_slice(device, &instances, label)
    }

    pub fn from_slice(device: &wgpu::Device, slice: &[T], label: Option<&'static str>) -> Self {
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label,
                contents: bytemuck::cast_slice(slice),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }
        );

        Self {
            buffer: Arc::new(instance_buffer),
            len: slice.len() as u32,
            _phantom: PhantomData,
        }
    }

    pub fn update_single(&self, queue: &wgpu::Queue, instance: T) {
        self.update_from_slice(queue, &[instance])
    }

    pub fn update_from_iterator<I>(&self, queue: &wgpu::Queue, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        let instances: Vec<T> = iter.into_iter().collect();
        self.update_from_slice(queue, &instances);
    }

    pub fn update_from_slice(&self, queue: &wgpu::Queue, slice: &[T]) {
        queue.write_buffer(&self.buffer(), 0, bytemuck::cast_slice(slice));
    }

    pub fn slice(&self, range: impl RangeBounds<u32>) -> VertexBufferSlice<T> {
        VertexBufferSlice::new(self, range)
    }
}

pub struct VertexBufferSlice<T: VertexRawRepr> {
    pub buffer: Arc<wgpu::Buffer>,
    pub range: std::ops::Range<u32>,
    _phantom: PhantomData<T>,
}

impl<T: VertexRawRepr> VertexBufferSlice<T> {
    pub fn new(buffer: &VertexBuffer<T>, range: impl RangeBounds<u32>) -> Self {
        // TODO bound checks!!!
        let range = {
            let lower = match range.start_bound() {
                std::ops::Bound::Included(&n) => n,
                std::ops::Bound::Excluded(&n) => n + 1,
                std::ops::Bound::Unbounded => 0,
            };
            let upper = match range.end_bound() {
                std::ops::Bound::Included(&n) => n + 1,
                std::ops::Bound::Excluded(&n) => n,
                std::ops::Bound::Unbounded => buffer.len(),
            };
            lower..upper
        };
        Self {
            buffer: buffer.buffer.clone(),
            range,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T: VertexRawRepr> Into<VertexBufferSlice<T>> for &'a VertexBuffer<T> {
    fn into(self) -> VertexBufferSlice<T> {
        self.slice(..)
    }
}

#[macro_export]
macro_rules! decl_vertex_raw_repr {
    (
        $(#[$meta:meta])*
        struct $name:ident ($step_mode:ident step mode) {
            $(
                $(#[$field_meta:meta])*
                pub $field:ident : $type:ty as [
                    $($n:literal => $repr:ident),*$(,)?
                ]
            ),*$(,)?
        }
    ) => {
        $(#[$meta])*
        #[repr(C)]
        #[derive(Copy, Clone, $crate::external::bytemuck::Pod, $crate::external::bytemuck::Zeroable)]
        pub struct $name {
            $(
                $(#[$field_meta])*
                pub $field: $type,
            )*
        }

        impl $name {
            /// The WGPU vertex attributes for this type
            pub const ATTRIBUTES: [
                $crate::external::wgpu::VertexAttribute;
                $crate::decl_vertex_raw_repr!(count $($($n),*),*)
            ] = $crate::external::wgpu::vertex_attr_array![
                $($($n => $repr),*),*
            ];

            /// The WGPU step mode for this type
            pub const STEP_MODE: $crate::external::wgpu::VertexStepMode = $crate::external::wgpu::VertexStepMode::$step_mode;
        }

        impl $crate::VertexRawRepr for $name {
            fn desc() -> $crate::external::wgpu::VertexBufferLayout<'static> {
                use std::mem;
                $crate::external::wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<$name>() as $crate::external::wgpu::BufferAddress,
                    // We need to switch from using a step mode of Vertex to Instance
                    // This means that our shaders will only change to use the next
                    // instance when the shader starts processing a new instance
                    step_mode: $crate::external::wgpu::VertexStepMode::$step_mode,
                    attributes: &Self::ATTRIBUTES,
                }
            }
        }
    };
    (count ) => { 0 };
    (count $t:tt $(,$tts:tt)*) => {
        $crate::decl_vertex_raw_repr!(count $($tts),*) + 1
    };
}