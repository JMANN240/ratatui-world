use glam::{Affine3, Vec3};
use ratatui_core::style::Color;

use lib::{plane::{Plane, partition_index}, triangle::Triangle};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColoredTriangle {
    triangle: Triangle,
    color: Color,
}

impl ColoredTriangle {
    pub fn new(triangle: Triangle, color: Color) -> Self {
        Self { triangle, color }
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
