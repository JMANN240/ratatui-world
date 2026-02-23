use bytemuck::{Pod, Zeroable};
use spirv_std::glam::{Affine3, Vec3};

use crate::plane::{Plane, partition_index};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Triangle {
    points: [Vec3; 3],
}

impl Triangle {
    pub fn new(points: [Vec3; 3]) -> Self {
        Self { points }
    }

    pub fn points(&self) -> &[Vec3; 3] {
        &self.points
    }

    pub fn points_mut(&mut self) -> &mut [Vec3; 3] {
        &mut self.points
    }

    pub fn transform(&self, transform: Affine3) -> Triangle {
        Self::new(
            [
                transform.transform_point3(self.points[0]),
                transform.transform_point3(self.points[1]),
                transform.transform_point3(self.points[2]),
            ]
        )
    }

    #[cfg(not(target_arch = "spirv"))]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_indices() {
        let triangle = Triangle::new(
            [
                Vec3::NEG_X * 0.1 + Vec3::Z,
                Vec3::NEG_X * 0.1 + Vec3::Y + Vec3::Z,
                Vec3::X * 0.1 + Vec3::Z,
            ],
        );

        let planes =
            Plane::between_vectors(5, Vec3::ZERO, Vec3::NEG_X + Vec3::Z, Vec3::X + Vec3::Z);

        assert_eq!(triangle.partition_indices(&planes), Some((1, 2)));

        let triangle = Triangle::new(
            [
                Vec3::NEG_X + Vec3::NEG_Z,
                Vec3::NEG_X + Vec3::Z,
                Vec3::X + Vec3::Z,
            ],
        );

        let planes = Plane::between_vectors(
            5,
            Vec3::ZERO,
            Vec3::NEG_Y + Vec3::NEG_Z,
            Vec3::Y + Vec3::NEG_Z,
        );

        assert_eq!(triangle.partition_indices(&planes), None)
    }
}
