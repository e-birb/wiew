use cgmath::{Matrix, SquareMatrix};
use nalgebra::Vector3;
use rotation3::*;
//use nalgebra::Vector3;

use crate::decl_vertex_raw_repr;

use super::VertexBuffer;

pub type Instance3dBuffer = VertexBuffer<Instance3d>;

decl_vertex_raw_repr! {
    struct Instance3d (Instance step mode) {
        pub model: [[f32; 4]; 4] as [
            0 => Float32x4,
            1 => Float32x4,
            2 => Float32x4,
            3 => Float32x4,
        ],
        pub model_inv_tr: [[f32; 3]; 3] as [
            4 => Float32x3,
            5 => Float32x3,
            6 => Float32x3,
        ],
    }
}

impl Instance3d {
    pub fn from_matrix(model: cgmath::Matrix4<f32>) -> Self {
        let model_3x3 = cgmath::Matrix3::from_cols(model.x.truncate(), model.y.truncate(), model.z.truncate());
        let model_3x3_inv_tr = model_3x3.invert().unwrap().transpose();
        Self {
            model: model.into(),
            model_inv_tr: model_3x3_inv_tr.into(),
        }
    }

    /// Identity instance
    pub fn id() -> Self {
        Self {
            model: cgmath::Matrix4::identity().into(),
            model_inv_tr: cgmath::Matrix3::identity().into(),
        }
    }

    pub fn translated_x_y_z(self, x: f32, y: f32, z: f32) -> Self {
        let translation = cgmath::Matrix4::from_translation(cgmath::Vector3::new(x, y, z));
        Self::from_matrix(translation * cgmath::Matrix4::from(self.model))
    }

    pub fn rotated_quaternion(self, q: cgmath::Quaternion<f32>) -> Self {
        let rotation = cgmath::Matrix4::from(q);
        Self::from_matrix(rotation * cgmath::Matrix4::from(self.model))
    }

    pub fn from_placement(placement: &Placement3<f32>) -> Self {
        Self::from_placement_and_scale(placement, &Vector3::new(1.0, 1.0, 1.0))
    }

    pub fn from_placement64(placement: &Placement3<f64>) -> Self {
        let mut p_f32: Placement3<f32> = Default::default();
        p_f32.rotation[0] = placement.rotation[0] as f32;
        p_f32.rotation[1] = placement.rotation[1] as f32;
        p_f32.rotation[2] = placement.rotation[2] as f32;
        p_f32.position[0] = placement.position[0] as f32;
        p_f32.position[1] = placement.position[1] as f32;
        p_f32.position[2] = placement.position[2] as f32;
        Self::from_placement_and_scale(&p_f32, &Vector3::new(1.0, 1.0, 1.0))
    }

    pub fn from_placement_and_scale(placement: &Placement3<f32>, scales: &Vector3<f32>) -> Self {
        let q = placement.rotation.to_quaternion();

        let model =
            cgmath::Matrix4::from_translation(cgmath::Vector3::new(
                placement.position.x,
                placement.position.y,
                placement.position.z,
            )) *
            cgmath::Matrix4::from(cgmath::Quaternion::new(
                q[0],
                q[1],
                q[2],
                q[3],
            )) *
            cgmath::Matrix4::from_nonuniform_scale(
                scales.x,
                scales.y,
                scales.z,
            );

        let model_3x3 = cgmath::Matrix3::from_cols(
            model.x.truncate(),
            model.y.truncate(),
            model.z.truncate(),
        );
        let model_3x3_inv_tr = model_3x3.invert().unwrap().transpose();

        Self {
            model: model.into(),
            model_inv_tr: model_3x3_inv_tr.into(),
        }
    }
}