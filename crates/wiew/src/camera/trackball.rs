use cgmath::num_traits::Pow;
use rotation3::*;

use crate::{instance::Instance3d, pipelines::flat::{self, FlatPipeline}, Pass, RenderContext, Resource, VertexBuffer};

use self::movement::{MouseMovement, NewMouseMovement};

use super::{ProjectionCamera, OPENGL_TO_WGPU_MATRIX};

use wgpu::{CompareFunction, PrimitiveTopology};

mod movement;


/// Simple trackball camera.
//#[derive(Debug, Clone, PartialEq)]
pub struct TrackballCamera {
    target: nalgebra::Point3<f32>,
    distance: f32,
    trackball_relative_radius: f32,
    rotation: Rotation<f32>, // TODO rename rotation to rotation3
    fovy_deg: f32,
    znear: f32,
    zfar: f32,
}

impl TrackballCamera {
    pub fn new() -> Self {
        const D: f32 = 5.0;
        Self {
            target: nalgebra::Point3::origin(),
            distance: D,
            trackball_relative_radius: 1.0 / D,
            rotation: Rotation::from_euler_angles(
                45f32.to_radians(),
                -30f32.to_radians(),
                0.0
            ),
            fovy_deg: 30.0,
            znear: 0.05,
            zfar: 1e3,
        }
    }

    pub fn render(
        &self,
        cx: &mut RenderContext,
        pass: &mut Pass,
        trackball: &Trackball,
    ) {
        let res = cx.resource(&trackball.res);

        res.update_instance(
            cx,
            self.target,
            self.distance,
            self.trackball_relative_radius,
        );

        res.flat_pipeline.render(
            cx,
            pass,
            res.vertex_buffer.slice(..),
            res.instance_buffer.slice(..),
        );

        res.flat_pipeline_2.render(
            cx,
            pass,
            res.vertex_buffer_2.slice(..),
            res.instance_buffer.slice(..),
        );
    }
}

impl ProjectionCamera for TrackballCamera {
    fn view_matrix(&self) -> cgmath::Matrix4<f32> {
        let eye = self.view_point();
        let up = self.rotation.rotate_vector(
            nalgebra::Vector3::new(0.0, 1.0, 0.0)
        );
        cgmath::Matrix4::look_at_rh(
            cgmath::Point3::new(eye.x, eye.y, eye.z),
            cgmath::Point3::new(self.target.x, self.target.y, self.target.z),
            cgmath::Vector3::new(up.x, up.y, up.z)
        )
    }

    fn projection_matrix(&self, aspect: f32) -> cgmath::Matrix4<f32> {
        let proj = cgmath::perspective(cgmath::Deg(self.fovy_deg), aspect, self.znear, self.zfar);
        OPENGL_TO_WGPU_MATRIX * proj
    }

    fn mouse_rotation(
        &mut self,
        from: (f32, f32),
        to: (f32, f32),
        width: f32,
        height: f32,
    ) {
        self.rotation = NewMouseMovement::mouse_rotation(
            from,
            to,
            width,
            height,
            self.fovy_deg,
            self.trackball_relative_radius,
            self.rotation,
        );
    }

    fn mouse_pan(
        &mut self,
        from: (f32, f32),
        to: (f32, f32),
        width: f32,
        height: f32,
    ) {
        self.target = NewMouseMovement::mouse_translation(
            from,
            to,
            width,
            height,
            self.fovy_deg,
            self.distance,
            self.rotation,
            self.target,
        );
    }

    fn mouse_zoom(
        &mut self,
        delta: f32,
    ) {
        self.distance *= 1.25f32.pow(delta);
        self.distance = self.distance.clamp(self.znear / self.trackball_relative_radius, self.zfar);
    }

    fn mouse_roll(
        &mut self,
        delta: f32,
    ) {
        let axis = self.rotation.rotate_vector(
            nalgebra::Vector3::new(0.0, 0.0, 1.0)
        );
        self.rotation = Rotation::from_vector(axis * delta) * self.rotation;
    }

    fn at_home(
        &mut self,
    ) {
        self.target = nalgebra::Point3::origin();
        self.distance = 5.0;
        self.rotation = Rotation::from_euler_angles(
            45f32.to_radians(),
            -30f32.to_radians(),
            0.0
        );
    }

    fn view_point(&self) -> nalgebra::Point3<f32> {
        self.target + self.rotation.rotate_vector(
            nalgebra::Vector3::new(0.0, 0.0, self.distance)
        )
    }

    fn light_dir(&self) -> nalgebra::Vector3<f32> {
        self.rotation.rotate_vector(nalgebra::Vector3::new(-0.5, 0.5, 2.0).normalize())
    }
}

//#[derive(Debug, Clone, Copy)]
pub struct Trackball {
    res: Resource<TrackballRes>,
}

impl Trackball {
    pub fn new(
    ) -> Self {
        Self {
            res: Resource::new(|cx: &mut RenderContext| TrackballRes::new(cx)),
        }
    }
}

pub struct TrackballRes {
    vertex_buffer: VertexBuffer<flat::Vertex>,
    vertex_buffer_2: VertexBuffer<flat::Vertex>,
    instance_buffer: VertexBuffer<Instance3d>,
    flat_pipeline: FlatPipeline,
    flat_pipeline_2: FlatPipeline,
}

impl TrackballRes {
    pub fn new(
        cx: &mut RenderContext,
    ) -> Self {
        const N: usize = 100;
        const L: f32 = 0.25;

        const A: f32 = 0.25;
        const TMP: f32 = 0.25;
        const R: [f32; 4] = [1.0, TMP, TMP, A];
        const G: [f32; 4] = [TMP, 1.0, TMP, A];
        const B: [f32; 4] = [TMP, TMP, 1.0, A];

        let mut vertices: Vec<flat::Vertex> = Vec::new();
        let mut v = |pos: [f32; 3], color: [f32; 4]| {
            vertices.push(flat::Vertex {
                position: pos,
                color,
            });
        };

        for i in 0..N {
            let angle = 2.0 * std::f32::consts::PI * (i as f32) / (N as f32);
            let angle2 = 2.0 * std::f32::consts::PI * ((i + 1) as f32) / (N as f32);
            v([0.0, angle.cos(), angle.sin()], R);
            v([0.0, angle2.cos(), angle2.sin()], R);
            v([angle.cos(), 0.0, angle.sin()], G);
            v([angle2.cos(), 0.0, angle2.sin()], G);
            v([angle.cos(), angle.sin(), 0.0], B);
            v([angle2.cos(), angle2.sin(), 0.0], B);
        }
        v([-L * 0.5, 0.0, 0.0], R);
        v([L, 0.0, 0.0], R);
        v([0.0, -L * 0.5, 0.0], G);
        v([0.0, L, 0.0], G);
        v([0.0, 0.0, -L * 0.5], B);
        v([0.0, 0.0, L], B);

        let vertex_buffer = VertexBuffer::<flat::Vertex>::from_slice(
            cx.device,
            &vertices,
            None,
        );

        for v in vertices.iter_mut() {
            v.color[3] = 0.5;
        }

        let vertex_buffer_2 = VertexBuffer::<flat::Vertex>::from_slice(
            cx.device,
            &vertices,
            None,
        );

        let instance_buffer = VertexBuffer::<Instance3d>::single(
            cx.device,
            Instance3d::from_placement(&Default::default()),
            None,
        );

        let flat_pipeline = FlatPipeline::new(
            PrimitiveTopology::LineList,
            CompareFunction::Always,
            false,
        );
        let flat_pipeline_2 = FlatPipeline::new(
            PrimitiveTopology::LineList,
            CompareFunction::LessEqual,
            true,
        );

        Self {
            vertex_buffer,
            vertex_buffer_2,
            instance_buffer,
            flat_pipeline,
            flat_pipeline_2,
        }
    }

    pub fn update_instance(
        &self,
        cx: &mut RenderContext,
        position: nalgebra::Point3<f32>,
        camera_distance: f32,
        trackball_relative_radius: f32,
    ) {
        self.instance_buffer.update_single(
            cx.queue,
            Instance3d::from_placement_and_scale(
                &Placement3 {
                    position: nalgebra::Vector3::new(position.x, position.y, position.z),
                    rotation: Default::default(),
                },
                &(nalgebra::Vector3::new(1.0, 1.0, 1.0) * camera_distance * trackball_relative_radius),
            ),
        );
    }
}