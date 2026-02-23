use bytemuck::{Pod, Zeroable};
use spirv_std::glam::Vec3;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Plane {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
}

impl Plane {
    pub fn new(a: f32, b: f32, c: f32, d: f32) -> Self {
        Self { a, b, c, d }
    }

    #[cfg(not(target_arch = "spirv"))]
    pub fn between_vectors(n: usize, point: Vec3, v1: Vec3, v2: Vec3) -> Vec<Self> {
        let angle = v1.angle_between(v2);
        let cross = v1.cross(v2);

        (0..n)
            .map(|i| {
                Self::from_point_and_vectors(
                    point,
                    cross,
                    v1.rotate_towards(v2, angle * i as f32 / (n - 1) as f32),
                )
            })
            .collect()
    }

    pub fn from_normal_and_offset(normal: Vec3, offset: f32) -> Self {
        Self::new(normal.x, normal.y, normal.z, -offset)
    }

    pub fn from_point_and_vectors(point: Vec3, v1: Vec3, v2: Vec3) -> Self {
        let normal = v1.cross(v2).normalize();

        let offset = point.dot(normal);

        Self::from_normal_and_offset(normal, offset)
    }

    pub fn from_points(p1: Vec3, p2: Vec3, p3: Vec3) -> Self {
        Self::from_point_and_vectors(p1, p2 - p1, p3 - p1)
    }

    pub fn a(&self) -> f32 {
        self.a
    }

    pub fn b(&self) -> f32 {
        self.b
    }

    pub fn c(&self) -> f32 {
        self.c
    }

    pub fn d(&self) -> f32 {
        self.d
    }

    pub fn side(&self, point: Vec3) -> Option<PlaneSide> {
        let value = self.a() * point.x + self.b() * point.y + self.c() * point.z + self.d();

        if value > 0.0 {
            Some(PlaneSide::Positive)
        } else if value < 0.0 {
            Some(PlaneSide::Negative)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaneSide {
    Positive,
    Negative,
}

pub fn partition_index(planes: &[Plane], vector: Vec3) -> Option<usize> {
    let mut planes = planes.iter();

    if let Some(first_plane) = planes.next()
        && matches!(first_plane.side(vector), None | Some(PlaneSide::Negative))
    {
        return None;
    }

    planes.enumerate().find_map(|(plane_index, plane)| {
        matches!(plane.side(vector), None | Some(PlaneSide::Negative)).then_some(plane_index)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plane_constructors() {
        let offset_x_y_plane =
            Plane::from_points(Vec3::Z, Vec3::X + Vec3::Z, Vec3::Y + Vec3::Z);

        assert_eq!(
            Plane::from_point_and_vectors(Vec3::ONE, Vec3::X, Vec3::Y),
            offset_x_y_plane
        );
        assert_eq!(
            Plane::from_normal_and_offset(Vec3::Z, 1.0),
            offset_x_y_plane
        );
        assert_eq!(Plane::new(0.0, 0.0, 1.0, -1.0), offset_x_y_plane);
    }

    #[test]
    fn test_plane_side() {
        let x_y_plane = Plane::from_points(Vec3::ZERO, Vec3::X, Vec3::Y);

        assert_eq!(x_y_plane.side(Vec3::X), None);
        assert_eq!(x_y_plane.side(Vec3::NEG_X), None);
        assert_eq!(x_y_plane.side(Vec3::Y), None);
        assert_eq!(x_y_plane.side(Vec3::NEG_Y), None);
        assert_eq!(x_y_plane.side(Vec3::Z), Some(PlaneSide::Positive));
        assert_eq!(x_y_plane.side(Vec3::NEG_Z), Some(PlaneSide::Negative));

        let tilted_x_y_plane = Plane::from_points(Vec3::ZERO, Vec3::X, Vec3::Y + Vec3::Z);

        assert_eq!(tilted_x_y_plane.side(Vec3::X), None);
        assert_eq!(tilted_x_y_plane.side(Vec3::NEG_X), None);
        assert_eq!(tilted_x_y_plane.side(Vec3::Y), Some(PlaneSide::Negative));
        assert_eq!(
            tilted_x_y_plane.side(Vec3::NEG_Y),
            Some(PlaneSide::Positive)
        );
        assert_eq!(tilted_x_y_plane.side(Vec3::Z), Some(PlaneSide::Positive));
        assert_eq!(
            tilted_x_y_plane.side(Vec3::NEG_Z),
            Some(PlaneSide::Negative)
        );

        let offset_tilted_x_y_plane = Plane::from_normal_and_offset(Vec3::NEG_Y + Vec3::Z, 100.0);

        assert_eq!(
            offset_tilted_x_y_plane.side(Vec3::X),
            Some(PlaneSide::Negative)
        );
        assert_eq!(
            offset_tilted_x_y_plane.side(Vec3::NEG_X),
            Some(PlaneSide::Negative)
        );
        assert_eq!(
            offset_tilted_x_y_plane.side(Vec3::Y),
            Some(PlaneSide::Negative)
        );
        assert_eq!(
            offset_tilted_x_y_plane.side(Vec3::NEG_Y),
            Some(PlaneSide::Negative)
        );
        assert_eq!(
            offset_tilted_x_y_plane.side(Vec3::Z),
            Some(PlaneSide::Negative)
        );
        assert_eq!(
            offset_tilted_x_y_plane.side(Vec3::NEG_Z),
            Some(PlaneSide::Negative)
        );
    }

    #[test]
    fn test_partition_index() {
        let planes =
            Plane::between_vectors(5, Vec3::ZERO, Vec3::NEG_X + Vec3::Z, Vec3::X + Vec3::Z);

        assert_eq!(partition_index(&planes, Vec3::NEG_X), None);
        assert_eq!(
            partition_index(&planes, Vec3::NEG_X + Vec3::Z * 1.1),
            Some(0)
        );
        assert_eq!(
            partition_index(&planes, Vec3::NEG_X * 0.1 + Vec3::Z),
            Some(1)
        );
        assert_eq!(partition_index(&planes, Vec3::X * 0.1 + Vec3::Z), Some(2));
        assert_eq!(partition_index(&planes, Vec3::X + Vec3::Z * 1.1), Some(3));
        assert_eq!(partition_index(&planes, Vec3::X), None);

        assert_eq!(partition_index(&[], Vec3::X), None);
    }
}
