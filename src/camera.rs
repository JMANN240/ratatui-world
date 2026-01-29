use std::f64::consts::PI;

use glam::{DMat4, DQuat, DVec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    position: DVec3,
    theta: f64,
    phi: f64,
    projection_matrix: DMat4
}

impl Camera {
    pub fn new(
        position: DVec3,
        theta: f64,
        phi: f64,
    projection_matrix: DMat4
    ) -> Self {
        Self {
            position,
            theta,
            phi,
            projection_matrix,
        }
    }

    pub fn position(&self) -> DVec3 {
        self.position
    }

    pub fn set_position(&mut self, position: DVec3) {
        self.position = position;
    }

    pub fn theta(&self) -> f64 {
        self.theta
    }

    pub fn set_theta(&mut self, theta: f64) {
        self.theta = theta;
    }

    pub fn theta_quaternion(&self) -> DQuat {
        DQuat::from_axis_angle(DVec3::Y, self.theta())
    }

    pub fn phi(&self) -> f64 {
        self.phi
    }

    pub fn set_phi(&mut self, phi: f64) {
        self.phi = phi;
    }

    pub fn phi_quaternion(&self) -> DQuat {
        DQuat::from_axis_angle(DVec3::X, self.phi())
    }

    pub fn quaternion(&self) -> DQuat {
        self.phi_quaternion() * self.theta_quaternion()
    }

    pub fn facing(&self) -> DVec3 {
        DVec3::NEG_Z.rotate_x(self.phi()).rotate_y(self.theta())
    }

    pub fn right(&self) -> DVec3 {
        self.facing().cross(DVec3::Y).normalize()
    }

    pub fn transformation_matrix(&self) -> DMat4 {
        DMat4::from_scale_rotation_translation(DVec3::ONE, self.quaternion(), self.position())
    }

    pub fn transform_world_point(&self, world_point: DVec3) -> DVec3 {
        self.transformation_matrix().inverse().transform_point3(world_point)
    }

    pub fn projection_matrix(&self) -> DMat4 {
        self.projection_matrix
    }

    pub fn project_camera_point(&self, camera_point: DVec3) -> DVec3 {
        self.projection_matrix().project_point3(camera_point)
    }

    pub fn transform_and_project_world_point(&self, world_point: DVec3) -> DVec3 {
        self.project_camera_point(self.transform_world_point(world_point))
    }

    pub fn should_clip(point: DVec3) -> bool {
        point.min_element() <= -1.0 || point.max_element() >= 1.0
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: DVec3::default(),
            theta: f64::default(),
            phi: f64::default(),
            projection_matrix: DMat4::perspective_infinite_rh(PI / 2.0, 1.0, 0.1),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use glam::dvec3;

    use super::*;

    #[test]
    fn test_facing() {
        let mut camera = Camera::default();

        assert!(camera.facing().angle_between(DVec3::NEG_Z) < 0.1);

        camera.set_theta(camera.theta() + PI / 2.0);

        assert!(camera.facing().angle_between(DVec3::NEG_X) < 0.1);

        camera.set_theta(camera.theta() - PI);

        assert!(camera.facing().angle_between(DVec3::X) < 0.1);

        camera.set_theta(camera.theta() + PI / 2.0);
        camera.set_phi(camera.phi() + PI / 2.0);

        assert!(camera.facing().angle_between(DVec3::Y) < 0.1);
    }

    #[test]
    fn test_transform() {
        let mut camera = Camera::default();

        let point = dvec3(0.0, 0.0, -1.0);

        camera.set_theta(camera.theta() + PI / 2.0);

        assert!(camera.transform_world_point(point).angle_between(DVec3::X) < 0.1);
    }

    #[test]
    fn test_project() {
        let camera = Camera::default();

        assert!(Camera::should_clip(camera.transform_and_project_world_point(DVec3::X)));
        assert!(Camera::should_clip(camera.transform_and_project_world_point(DVec3::NEG_X)));
        assert!(Camera::should_clip(camera.transform_and_project_world_point(DVec3::Y)));
        assert!(Camera::should_clip(camera.transform_and_project_world_point(DVec3::NEG_Y)));
        assert!(Camera::should_clip(camera.transform_and_project_world_point(DVec3::Z)));
        assert!(!Camera::should_clip(camera.transform_and_project_world_point(DVec3::NEG_Z)));
    }
}
