use std::{f64::consts::E, sync::{Arc, Mutex}};

use chrono::Utc;
use glam::{DMat4, DQuat, DVec2, DVec3, U16Vec2, dvec2, dvec3, u16vec2};
use rand::{fill, random_bool};
use ratatui_core::{buffer::Buffer, layout::Rect, style::Color, symbols::Marker, widgets::Widget};
use ratatui_widgets::canvas::{Canvas, Context, Points};
use rayon::prelude::*;
use tracing::debug;

use crate::{shape::Triangle, world::World};

#[derive(Debug, Clone, PartialEq)]
pub struct RayTrace {
    world: World,
    aspect_ratio: f64,
    fov_y: f64,
    position: DVec3,
    theta: f64,
    phi: f64,
}

impl RayTrace {
    pub fn new(
        world: World,
        aspect_ratio: f64,
        fov_y: f64,
        position: DVec3,
        theta: f64,
        phi: f64,
    ) -> Self {
        Self {
            world,
            aspect_ratio,
            fov_y,
            position,
            theta,
            phi,
        }
    }

    pub fn fov_x(&self) -> f64 {
        self.fov_y * self.aspect_ratio
    }

    pub fn depth(&self, resolution_y: usize) -> f64 {
        let scale = (resolution_y as f64 / 2.0) / (self.fov_y / 2.0).sin();

        (self.fov_y / 2.0).cos() * scale
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
        self.theta_quaternion() * self.phi_quaternion()
    }

    pub fn facing(&self) -> DVec3 {
        DVec3::NEG_Z.rotate_x(self.phi()).rotate_y(self.theta())
    }

    pub fn right(&self) -> DVec3 {
        self.facing().cross(DVec3::Y).normalize()
    }

    pub fn transformation_matrix(&self) -> DMat4 {
        DMat4::from_scale_rotation_translation(DVec3::ONE, self.quaternion(), DVec3::ZERO)
    }

    pub fn transform_ray(&self, world_vector: DVec3) -> DVec3 {
        self.transformation_matrix().transform_vector3(world_vector)
    }
}

impl Widget for &RayTrace {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let character_x_resolution = 2; // how many subpixels there are along the width of a character
        let character_y_resolution = 4; // how many subpixels there are along the height of a character

        // lets say that the canvas is 60 characters wide and 60 characters tall

        let resolution_x = area.width * character_x_resolution; // 60 * 2 = 120 for quadrant
        let resolution_y = area.height * character_y_resolution; // 60 * 2 = 120 for quadrant

        // so we really have 120 pixels to work with both ways with quadrant

        let cell_aspect_ratio = 1.0 / 2.0;

        // but the characters are twice as tall as they are width, meaning that the whole "square" canvas is actually a rectangle

        let aspect_ratio =
            cell_aspect_ratio * character_y_resolution as f64 / character_x_resolution as f64;

        // So we get the aspect ratio of each pixel

        // 0.5 * 1 / 1 = 0.5 in the case of a block
        // 0.5 * 2 / 2 = 0.5 in the case of a quadrant
        // 0.5 * 4 / 2 = 1.0 in the case of braille

        let width = resolution_x as f64; // The space on the screen is as wide as there are pixels. 120 for quadrant
        let height = (resolution_y as f64) / aspect_ratio; // The space on the screen is as tall as there are pixels divided by the AR. 120 / 0.5 = 240 for quadrant

        let cell_spacing_x = width / (area.width as f64); // For quadrant it is 120 / 60 = 2, so each cell should be 2 apart width-wise
        let cell_spacing_y = height / (area.height as f64); // For quadrant it is 240 / 60 = 4, so each cell should be 4 apart height-wise

        let pixel_spacing_x = cell_spacing_x / (character_x_resolution as f64); // For quadrant it is 2 / 2 = 1, so each pixel should be 1 apart width-wise
        let pixel_spacing_y = cell_spacing_y / (character_y_resolution as f64); // For quadrant it is 4 / 2 = 2, so each pixel should be 2 apart height-wise

        let left = -width / 2.0; // -120 / 2 = -60
        let right = width / 2.0; // 120 / 2 = 60
        let up = height / 2.0; // 240 / 2 = 120
        let down = -height / 2.0; // -240 / 2 = -120

        // eprintln!("WIDTH {width} LEFT {left} RIGHT {right} HEIGHT {height} UP {up} DOWN {down}");

        let depth = self.depth(resolution_y as usize);

        let canvas = Canvas::default()
            .marker(Marker::Braille)
            .x_bounds([left, right])
            .y_bounds([down, up])
            .paint(|context| {
                debug!("start");
                let cells = (0..area.width)
                    .into_par_iter()
                    .flat_map(|cell_x_index| {
                        let ray_x_base =
                            left + cell_x_index as f64 * cell_spacing_x + pixel_spacing_x / 2.0;

                        // -60 + 0 * 2 + 0.5 = -59.5
                        // -60 + 1 * 2 + 0.5 = -57.5
                        // -60 + 2 * 2 + 0.5 = -55.5

                        (0..area.height).into_par_iter().map(move |cell_y_index| {
                            let ray_y_base =
                                down + cell_y_index as f64 * cell_spacing_y + pixel_spacing_y / 2.0;

                            // -120 + 0 * 4 + 1 = -119
                            // -120 + 1 * 4 + 1 = -115
                            // -120 + 2 * 4 + 1 = -111

                            let mut rays = Vec::new();

                            for ray_x_index in 0..character_x_resolution {
                                let ray_x_offset = ray_x_index as f64 * pixel_spacing_x;

                                for ray_y_index in 0..character_y_resolution {
                                    let ray_y_offset = ray_y_index as f64 * pixel_spacing_y;

                                    rays.push(Ray::new(
                                        self.transform_ray(dvec3(
                                            ray_x_base + ray_x_offset as f64,
                                            ray_y_base + ray_y_offset as f64,
                                            -depth,
                                        )),
                                        dvec2(
                                            ray_x_base + ray_x_offset as f64,
                                            ray_y_base + ray_y_offset as f64,
                                        ),
                                    ));
                                }
                            }

                            Cell::new(u16vec2(cell_x_index, cell_y_index), rays)
                        })
                    })
                    .collect_vec_list();

                debug!("2");
                let intersections = cells
                    .par_iter()
                    .flatten()
                    .flat_map(|cell| cell.get_intersections(self, &self.world))
                    .collect::<Vec<_>>();

                debug!("3");
                let mut bools = vec![false; (area.width * area.height) as usize];

                intersections
                    .par_iter()
                    .map(|(_ray, _triangle, intersection)| {
                        let distance = (self.position() - intersection).length();
                        random_bool(E.powf(-0.03 * distance.powi(2)))
                    })
                    .collect_into_vec(&mut bools);

                debug!("4");
                for (index, (ray, triangle, intersection)) in intersections.iter().enumerate() {
                    if *bools.get(index).unwrap_or(&false) {
                        let distance = (self.position() - intersection).length();
                        let normalized_distance = E.powf(-0.03 * distance.powi(2));
                        context.draw(&Points::new(
                            &[(ray.screen_vector().x, ray.screen_vector().y)],
                            if let Color::Rgb(r, g, b) = triangle.color() {
                                Color::Rgb((r as f64 * normalized_distance) as u8, (g as f64 * normalized_distance) as u8, (b as f64 * normalized_distance) as u8)
                            } else {
                                triangle.color()
                            },
                        ));
                    }
                }
                debug!("5");
            });

        canvas.render(area, buf);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    coords: U16Vec2,
    rays: Vec<Ray>,
}

impl Cell {
    pub fn new(coords: U16Vec2, rays: Vec<Ray>) -> Self {
        Self { coords, rays }
    }

    pub fn get_intersections(
        &self,
        ray_trace: &RayTrace,
        world: &World,
    ) -> Vec<(Ray, Triangle, DVec3)> {
        let mut ray_triangle_intersections = self
            .rays
            .par_iter()
            .copied()
            .filter_map(|ray| {
                world
                    .triangles()
                    .copied()
                    .filter_map(|triangle| {
                        moller_trumbore_intersection(
                            ray_trace.position(),
                            ray.world_vector(),
                            *triangle.points(),
                        )
                        .map(|intersection| (ray, triangle, intersection))
                    })
                    .min_by(|(_, _, l), (_, _, r)| {
                        (ray_trace.position() - l)
                            .length()
                            .partial_cmp(&(ray_trace.position() - r).length())
                            .unwrap()
                    })
            })
            .collect::<Vec<_>>();

        ray_triangle_intersections.sort_by(|(_, _, l), (_, _, r)| {
            (ray_trace.position() - l)
                .length()
                .partial_cmp(&(ray_trace.position() - r).length())
                .unwrap()
        });
        // ray_triangle_intersections.reverse();

        ray_triangle_intersections
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    world_vector: DVec3,
    screen_vector: DVec2,
}

impl Ray {
    pub fn new(world_vector: DVec3, screen_vector: DVec2) -> Self {
        Self {
            world_vector,
            screen_vector,
        }
    }

    pub fn world_vector(&self) -> DVec3 {
        self.world_vector
    }

    pub fn screen_vector(&self) -> DVec2 {
        self.screen_vector
    }
}

// https://en.wikipedia.org/wiki/M%C3%B6ller%E2%80%93Trumbore_intersection_algorithm#Rust_implementation
pub fn moller_trumbore_intersection(
    origin: DVec3,
    direction: DVec3,
    triangle: [DVec3; 3],
) -> Option<DVec3> {
    let e1 = triangle[1] - triangle[0];
    let e2 = triangle[2] - triangle[0];

    let ray_cross_e2 = direction.cross(e2);
    let det = e1.dot(ray_cross_e2);

    if det > -f64::EPSILON && det < f64::EPSILON {
        return None; // This ray is parallel to this triangle.
    }

    let inv_det = 1.0 / det;
    let s = origin - triangle[0];
    let u = inv_det * s.dot(ray_cross_e2);
    if u < 0.0 || u > 1.0 {
        return None;
    }

    let s_cross_e1 = s.cross(e1);
    let v = inv_det * direction.dot(s_cross_e1);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    // At this stage we can compute t to find out where the intersection point is on the line.
    let t = inv_det * e2.dot(s_cross_e1);

    if t > f64::EPSILON {
        // ray intersection
        let intersection_point = origin + direction * t;
        return Some(intersection_point);
    } else {
        // This means that there is a line intersection but not a ray intersection.
        return None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moller_trumbore() {
        let position = DVec3::ZERO;
        let direction = DVec3::NEG_Z;
        let triangle = [
            dvec3(-1.0, -1.0, -2.0),
            dvec3(0.0, 1.0, -2.0),
            dvec3(1.0, -1.0, -2.0),
        ];

        assert_eq!(
            moller_trumbore_intersection(position, direction, triangle),
            Some(dvec3(0.0, 0.0, -2.0))
        );

        let position = dvec3(0.0, 0.0, 10.0);

        assert_eq!(
            moller_trumbore_intersection(position, direction, triangle),
            Some(dvec3(0.0, 0.0, -2.0))
        );

        let position = dvec3(0.0, 0.0, -10.0);

        assert_eq!(
            moller_trumbore_intersection(position, direction, triangle),
            None
        );
    }
}
