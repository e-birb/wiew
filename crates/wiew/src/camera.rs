use std::fmt::Debug;

mod trackball;
pub use trackball::*;
mod identity; pub use identity::*;
use wgpu::util::DeviceExt;

use crate::{SingletonResource, RenderContext};

pub trait ProjectionCamera/*: Debug*/ {
    /// The view matrix of the camera.
    fn view_matrix(&self) -> cgmath::Matrix4<f32>;
    /// The projection matrix of the camera.
    fn projection_matrix(&self, aspect: f32) -> cgmath::Matrix4<f32>;
    /// Returns a modified camera after a mouse rotation event.
    fn mouse_rotation(
        &mut self,
        from: (f32, f32),
        to: (f32, f32),
        width: f32, height: f32,
    );
    /// Returns a modified camera after a mouse pan event.
    fn mouse_pan(
        &mut self,
        from: (f32, f32),
        to: (f32, f32),
        width: f32, height: f32,
    );
    /// Returns a modified camera after a mouse zoom event.
    fn mouse_zoom(
        &mut self,
        delta: f32,
    );
    fn mouse_zoom_r(
        &mut self,
        delta: f32,
    );
    fn mouse_roll(
        &mut self,
        delta: f32,
    );
    /// Returns a modified camera that returned to its home position.
    fn at_home(
        &mut self,
    );

    fn view_point(&self) -> nalgebra::Point3<f32>;

    fn light_dir(&self) -> nalgebra::Vector3<f32>;
}

pub struct ProjectionCameraCommon {
    bind_group_layout: wgpu::BindGroupLayout,
}

impl SingletonResource for ProjectionCameraCommon {
    fn init(ctx: &mut RenderContext) -> Self {
        let layout = ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });

        Self {
            bind_group_layout: layout,
        }
    }
}

impl ProjectionCameraCommon {
    pub fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
}

pub struct ProjectionCameraBuffer {
    pub uniform: CameraUniform, // TODO maybe remove this
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl ProjectionCameraBuffer {
    pub fn new(
        cx: &mut RenderContext,
    ) -> Self {
        let uniform = CameraUniform::new();

        let common = cx.singleton::<ProjectionCameraCommon>();

        let buffer = cx.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let camera_bind_group = cx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &common.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });

        ProjectionCameraBuffer {
            uniform,
            buffer,
            bind_group: camera_bind_group,
        }
    }

    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        camera: &impl ProjectionCamera,
        aspect: f32,
    ) {
        self.uniform.update_view_proj(camera, aspect);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view: [[f32; 4]; 4],
    proj: [[f32; 4]; 4],
    view_point: [f32; 3],
    _padding0: [f32; 1],
    light_dir: [f32; 3],
    _padding1: [f32; 1],
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view: cgmath::Matrix4::identity().into(),
            proj: cgmath::Matrix4::identity().into(),
            view_point: [0.0; 3],
            _padding0: [0.0; 1],
            light_dir: [0.0; 3],
            _padding1: [0.0; 1],
        }
    }

    fn update_view_proj(&mut self, camera: &impl ProjectionCamera, aspect: f32) {
        self.view = camera.view_matrix().into();
        self.proj = camera.projection_matrix(aspect).into();
        self.view_point = camera.view_point().into();
        self.light_dir = camera.light_dir().into();
        //self.light_dir = nalgebra::Vector3::new(0.0, 1.0, 0.0).normalize().into();
    }
}

/// OpenGL to wgpu matrix.
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);