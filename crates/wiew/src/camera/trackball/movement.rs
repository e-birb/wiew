use rotation3::*;
use nalgebra::Point3;


pub trait MouseMovement {
    fn mouse_rotation(
        from: (f32, f32),
        to: (f32, f32),
        width: f32, height: f32,
        fovy_deg: f32,
        trackball_relative_radius: f32,
        current_rotation: Rotation<f32>,
    ) -> Rotation<f32>;

    fn mouse_translation(
        from: (f32, f32),
        to: (f32, f32),
        _width: f32, height: f32,
        fovy_deg: f32,
        camera_distance: f32,
        current_rotation: Rotation<f32>,
        current_target: Point3<f32>,
    ) -> Point3<f32> {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);

        let factor = fovy_deg.to_radians() / height as f32 * camera_distance;

        let i = current_rotation.rotate_vector(
            nalgebra::Vector3::new(1.0, 0.0, 0.0)
        );
        let j = current_rotation.rotate_vector(
            nalgebra::Vector3::new(0.0, 1.0, 0.0)
        );

        current_target - dx * factor * i + dy * factor * j
    }
}

pub struct OldMouseMovement;
impl MouseMovement for OldMouseMovement {
    fn mouse_rotation(
        from: (f32, f32),
        to: (f32, f32),
        width: f32, height: f32,
        fovy_deg: f32,
        trackball_relative_radius: f32,
        current_rotation: Rotation<f32>,
    ) -> Rotation<f32> {
        let (x, y) = from;
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);

        let factor = fovy_deg.to_radians() / height as f32 * (1.0 - trackball_relative_radius);

        let i = current_rotation.rotate_vector(
            nalgebra::Vector3::new(1.0, 0.0, 0.0)
        );
        let j = -current_rotation.rotate_vector(
            nalgebra::Vector3::new(0.0, 1.0, 0.0)
        );

        let r = current_rotation.rotate_vector(
            nalgebra::Vector3::new(0.0, 0.0, 1.0)
        );

        let p = r * trackball_relative_radius;

        //console::log!(format!("i = {:?}, j = {:?}, r = {:?}", i, j, r));
        let x1 = x - dx;
        let y1 = y - dy;
        let x2 = x;
        let y2 = y;
        let p1 = i * (x1 - width as f32 / 2.0) * factor + j * (y1 - height as f32 / 2.0) * factor + p;
        let p2 = i * (x2 - width as f32 / 2.0) * factor + j * (y2 - height as f32 / 2.0) * factor + p;

        let r = -Rotation::between(p1, p2);

        r * current_rotation
    }
}

pub struct NewMouseMovement;
impl MouseMovement for NewMouseMovement {
    fn mouse_rotation(
        from: (f32, f32),
        to: (f32, f32),
        width: f32, height: f32,
        fovy_deg: f32,
        trackball_relative_radius: f32,
        current_rotation: Rotation<f32>,
    ) -> Rotation<f32> {
        let (x, y) = from;
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);

        let factor = fovy_deg.to_radians().tan() * (1.0 - trackball_relative_radius) / height as f32;
        //console::log!(format!("factor = {:?}", factor));
        let mouse_pos_to_3d = |x, y| {
            let x = (x - width as f32 / 2.0) * factor;
            let y = (height as f32 / 2.0 - y) * factor;
            let z = trackball_relative_radius;
            nalgebra::Vector3::new(x, y, z)
        };
        let p1 = mouse_pos_to_3d(x, y);
        let p0 = mouse_pos_to_3d(x - dx, y - dy);
        let r = Rotation::between(p1, p0);
        //console::log!(format!("r = {:?}", r));
        current_rotation * r
    }
}