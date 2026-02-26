use bvh::{
    aabb::{Aabb, Bounded},
    bounding_hierarchy::BHShape,
};
use glam::{Affine3, Vec3};
use nalgebra::Point3;
use ratatui_core::style::Color;

use crate::plane::{Plane, partition_index};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    points: [Vec3; 3],
    node_index: usize,
}

impl Triangle {
    pub fn new(points: [Vec3; 3]) -> Self {
        Self {
            points,
            node_index: 0,
        }
    }

    pub fn points(&self) -> &[Vec3; 3] {
        &self.points
    }

    pub fn points_mut(&mut self) -> &mut [Vec3; 3] {
        &mut self.points
    }

    pub fn transform(&self, transform: Affine3) -> Triangle {
        Self::new([
            transform.transform_point3(self.points[0]),
            transform.transform_point3(self.points[1]),
            transform.transform_point3(self.points[2]),
        ])
    }

    pub fn min_x(&self) -> Option<f32> {
        self.points()
            .map(|point| point.x)
            .into_iter()
            .min_by(|l, r| l.partial_cmp(r).unwrap())
    }

    pub fn max_x(&self) -> Option<f32> {
        self.points()
            .map(|point| point.x)
            .into_iter()
            .max_by(|l, r| l.partial_cmp(r).unwrap())
    }

    pub fn min_y(&self) -> Option<f32> {
        self.points()
            .map(|point| point.y)
            .into_iter()
            .min_by(|l, r| l.partial_cmp(r).unwrap())
    }

    pub fn max_y(&self) -> Option<f32> {
        self.points()
            .map(|point| point.y)
            .into_iter()
            .max_by(|l, r| l.partial_cmp(r).unwrap())
    }

    pub fn min_z(&self) -> Option<f32> {
        self.points()
            .map(|point| point.z)
            .into_iter()
            .min_by(|l, r| l.partial_cmp(r).unwrap())
    }

    pub fn max_z(&self) -> Option<f32> {
        self.points()
            .map(|point| point.z)
            .into_iter()
            .max_by(|l, r| l.partial_cmp(r).unwrap())
    }
}

impl Bounded<f32, 3> for Triangle {
    fn aabb(&self) -> Aabb<f32, 3> {
        let min = Point3::new(
            self.min_x().unwrap(),
            self.min_y().unwrap(),
            self.min_z().unwrap(),
        );

        let max = Point3::new(
            self.max_x().unwrap(),
            self.max_y().unwrap(),
            self.max_z().unwrap(),
        );

        Aabb::with_bounds(min, max)
    }
}

impl BHShape<f32, 3> for Triangle {
    fn set_bh_node_index(&mut self, index: usize) {
        self.node_index = index;
    }

    fn bh_node_index(&self) -> usize {
        self.node_index
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColoredTriangle {
    triangle: Triangle,
    color: Color,
}

impl ColoredTriangle {
    pub fn new(triangle: Triangle, color: Color) -> Self {
        Self { triangle, color }
    }

    pub fn inner(&self) -> Triangle {
        self.triangle
    }

    pub fn inner_mut(&mut self) -> &mut Triangle {
        &mut self.triangle
    }

    pub fn points(&self) -> &[Vec3; 3] {
        &self.triangle.points()
    }

    pub fn points_mut(&mut self) -> &mut [Vec3; 3] {
        self.triangle.points_mut()
    }

    pub fn color(&self) -> Color {
        self.color
    }

    pub fn transform(&self, transform: Affine3) -> ColoredTriangle {
        Self::new(
            Triangle::new(self.points().map(|point| transform.transform_point3(point))),
            self.color,
        )
    }

    pub fn partition_indices(&self, planes: &[Plane]) -> Option<(usize, usize)> {
        let partition_indices = self
            .points()
            .iter()
            .map(|point| partition_index(planes, *point))
            .collect::<Option<Vec<usize>>>()?;

        let Some(min) = partition_indices.iter().min() else {
            return None;
        };

        let Some(max) = partition_indices.iter().max() else {
            return None;
        };

        Some((*min, *max))
    }
}
