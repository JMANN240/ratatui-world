use std::f32::consts::PI;

use glam::{Mat4, Quat, Vec3, Vec3Swizzles};
use rand::random_bool;
use ratatui_canvas_polygon::triangle::Triangle;
use ratatui_widgets::canvas::Context;

use crate::world::{Renderer, World};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    position: Vec3,
    theta: f32,
    phi: f32,
    projection_matrix: Mat4,
}

impl Camera {
    pub fn new(position: Vec3, theta: f32, phi: f32, projection_matrix: Mat4) -> Self {
        Self {
            position,
            theta,
            phi,
            projection_matrix,
        }
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    pub fn theta(&self) -> f32 {
        self.theta
    }

    pub fn set_theta(&mut self, theta: f32) {
        self.theta = theta;
    }

    pub fn theta_quaternion(&self) -> Quat {
        Quat::from_axis_angle(Vec3::Y, self.theta())
    }

    pub fn phi(&self) -> f32 {
        self.phi
    }

    pub fn set_phi(&mut self, phi: f32) {
        self.phi = phi;
    }

    pub fn phi_quaternion(&self) -> Quat {
        Quat::from_axis_angle(Vec3::X, self.phi())
    }

    pub fn quaternion(&self) -> Quat {
        self.theta_quaternion() * self.phi_quaternion()
    }

    pub fn facing(&self) -> Vec3 {
        Vec3::NEG_Z.rotate_x(self.phi()).rotate_y(self.theta())
    }

    pub fn right(&self) -> Vec3 {
        self.facing().cross(Vec3::Y).normalize()
    }

    pub fn transformation_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(Vec3::ONE, self.quaternion(), self.position())
    }

    pub fn transform_world_point(&self, world_point: Vec3) -> Vec3 {
        self.transformation_matrix()
            .inverse()
            .transform_point3(world_point)
    }

    pub fn projection_matrix(&self) -> Mat4 {
        self.projection_matrix
    }

    pub fn project_camera_point(&self, camera_point: Vec3) -> Vec3 {
        self.projection_matrix().project_point3(camera_point)
    }

    pub fn transform_and_project_world_point(&self, world_point: Vec3) -> Vec3 {
        self.project_camera_point(self.transform_world_point(world_point))
    }

    pub fn should_clip(point: Vec3) -> bool {
        point.min_element() <= -1.0 || point.max_element() >= 1.0
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::default(),
            theta: f32::default(),
            phi: f32::default(),
            projection_matrix: Mat4::perspective_infinite_rh(PI / 2.0, 1.0, 0.1),
        }
    }
}

impl Renderer for Camera {
    fn render(&self, world: &World, context: &mut Context, _width: usize, _height: usize) {
        for shape in world.shapes() {
            let mut sorted_faces = shape.faces().clone();
            sorted_faces.sort_by_key(|face| {
                (self
                    .transform_world_point(*face.points()[0])
                    .midpoint(self.transform_world_point(*face.points()[1]))
                    .midpoint(self.transform_world_point(*face.points()[2]))
                    .length()
                    * 1000.0) as i32
            });
            sorted_faces.reverse();

            for face in sorted_faces {
                let midpoint = self
                    .transform_world_point(*face.points()[0])
                    .midpoint(self.transform_world_point(*face.points()[1]))
                    .midpoint(self.transform_world_point(*face.points()[2]));

                let chance = if Camera::should_clip(self.project_camera_point(midpoint)) {
                    0.0
                } else {
                    (midpoint.length() / 10.0).clamp(0.0, 1.0) * -1.0 + 1.0
                };

                context.draw(&Triangle::new(
                    [
                        self.transform_and_project_world_point((*face.points()[0]).clone())
                            .xy()
                            .to_array()
                            .into(),
                        self.transform_and_project_world_point((*face.points()[1]).clone())
                            .xy()
                            .to_array()
                            .into(),
                        self.transform_and_project_world_point((*face.points()[2]).clone())
                            .xy()
                            .to_array()
                            .into(),
                    ],
                    face.color(),
                    Some(Box::new(move |_, _| random_bool(chance.powi(2)))),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use glam::vec3;

    use super::*;

    #[test]
    fn test_facing() {
        let mut camera = Camera::default();

        assert!(camera.facing().angle_between(Vec3::NEG_Z) < 0.1);

        camera.set_theta(camera.theta() + PI / 2.0);

        assert!(camera.facing().angle_between(Vec3::NEG_X) < 0.1);

        camera.set_theta(camera.theta() - PI);

        assert!(camera.facing().angle_between(Vec3::X) < 0.1);

        camera.set_theta(camera.theta() + PI / 2.0);
        camera.set_phi(camera.phi() + PI / 2.0);

        assert!(camera.facing().angle_between(Vec3::Y) < 0.1);
    }

    #[test]
    fn test_transform() {
        let mut camera = Camera::default();

        let point = vec3(0.0, 0.0, -1.0);

        camera.set_theta(camera.theta() + PI / 2.0);

        assert!(camera.transform_world_point(point).angle_between(Vec3::X) < 0.1);
    }

    #[test]
    fn test_project() {
        let camera = Camera::default();

        assert!(Camera::should_clip(
            camera.transform_and_project_world_point(Vec3::X)
        ));
        assert!(Camera::should_clip(
            camera.transform_and_project_world_point(Vec3::NEG_X)
        ));
        assert!(Camera::should_clip(
            camera.transform_and_project_world_point(Vec3::Y)
        ));
        assert!(Camera::should_clip(
            camera.transform_and_project_world_point(Vec3::NEG_Y)
        ));
        assert!(Camera::should_clip(
            camera.transform_and_project_world_point(Vec3::Z)
        ));
        assert!(!Camera::should_clip(
            camera.transform_and_project_world_point(Vec3::NEG_Z)
        ));
    }
}
