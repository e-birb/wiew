use cgmath::SquareMatrix;

use crate::ProjectionCamera;



pub struct IdentityCamera;

impl ProjectionCamera for IdentityCamera {
    fn view_matrix(&self) -> cgmath::Matrix4<f32> {
        cgmath::Matrix4::identity()
    }

    fn projection_matrix(&self, _aspect: f32) -> cgmath::Matrix4<f32> {
        cgmath::Matrix4::identity()
    }

    fn mouse_rotation(
        &mut self,
        _from: (f32, f32),
        _to: (f32, f32),
        _width: f32,
        _height: f32,
    ) {}

    fn mouse_pan(
        &mut self,
        _from: (f32, f32),
        _to: (f32, f32),
        _width: f32,
        _height: f32,
    ) {}

    fn mouse_zoom(
        &mut self,
        _delta: f32,
    ) {}

    fn mouse_zoom_r(
        &mut self,
        _delta: f32,
    ) {}

    fn mouse_roll(
        &mut self,
        _delta: f32,
    ) {}

    fn at_home(
        &mut self,
    ) {}

    fn light_dir(&self) -> nalgebra::Vector3<f32> {
        nalgebra::Vector3::new(0.0, 1.0, 0.0)
    }

    fn view_point(&self) -> nalgebra::Point3<f32> {
        nalgebra::Point3::new(0.0, 0.0, 0.0)
    }
}