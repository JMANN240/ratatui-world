use glam::Vec3;

pub mod plane;
pub mod ray;
pub mod ray_trace;
pub mod shape;
pub mod triangle;
pub mod world;

// https://en.wikipedia.org/wiki/M%C3%B6ller%E2%80%93Trumbore_intersection_algorithm#Rust_implementation
pub fn moller_trumbore_intersection(
    origin: Vec3,
    direction: Vec3,
    triangle: [Vec3; 3],
) -> Option<Vec3> {
    let e1 = triangle[1] - triangle[0];
    let e2 = triangle[2] - triangle[0];

    let ray_cross_e2 = direction.cross(e2);
    let det = e1.dot(ray_cross_e2);

    if det > -f32::EPSILON && det < f32::EPSILON {
        return None; // This ray is parallel to this triangle.
    }

    let inv_det = 1.0 / det;
    let s = origin - triangle[0];
    let u = inv_det * s.dot(ray_cross_e2);
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let s_cross_e1 = s.cross(e1);
    let v = inv_det * direction.dot(s_cross_e1);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    // At this stage we can compute t to find out where the intersection point is on the line.
    let t = inv_det * e2.dot(s_cross_e1);

    (t > f32::EPSILON).then_some(origin + direction * t)
}
