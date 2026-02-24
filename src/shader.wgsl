struct MaybeVec3f {
    vector: vec3f,
    valid: bool,
}

struct Params {
    width: u32,
    height: u32,
}

struct Camera {
    position: vec3f,
    theta: f32,
    phi: f32,
}

struct Triangle {
    points: array<vec3f, 3>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(2) var<storage, read> rays: array<vec3f>;
@group(0) @binding(3) var<storage, read> triangles: array<Triangle>;
@group(0) @binding(4) var<storage, read_write> intersection_distances: array<f32>;

@compute
@workgroup_size(16, 16)
fn main(
    @builtin(global_invocation_id) id: vec3<u32>
) {
    let index = id.x + id.y * params.width;

    if id.x >= params.width || id.y >= params.height {
        return;
    }

    let ray = rays[index];

    var least_distance = 1000f;

    let origin = camera.position;

    let n_triangles = arrayLength(&triangles);

    for (var i = 0u; i < n_triangles; i++) {
        let triangle = triangles[i];

        let maybe_intersection = moller_trumbore_intersection(origin, ray, triangle.points);

        if maybe_intersection.valid {
            let intersection_distance = length(maybe_intersection.vector - origin);

            if intersection_distance < least_distance {
                least_distance = intersection_distance;
            }
        }
    }

    intersection_distances[index] = least_distance;
}

fn moller_trumbore_intersection(
    origin: vec3f, // (0, 0, 0)
    direction: vec3f, // (0, 0, -1)
    triangle: array<vec3f, 3>,
) -> MaybeVec3f {
    // vec3(-10.0, -10.0, -10.0),
    // vec3(100.0, 0.0, -10.0),
    // vec3(0.0, 100.0, -10.0),

    let e1 = triangle[1] - triangle[0]; // (110, 10, 0)
    let e2 = triangle[2] - triangle[0]; // (10, 110, 0)

    let ray_cross_e2 = cross(direction, e2); // (110, -10, 0)
    let det = dot(e1, ray_cross_e2); // 12000

    if det > -0.0000001 && det < 0.0000001 { // false
        return MaybeVec3f(vec3f(), false); // This ray is parallel to this triangle.
    }

    let inv_det = 1.0 / det; // 0.00008333333
    let s = origin - triangle[0]; // (10.0, 10.0, 10.0)
    let u = inv_det * dot(s, ray_cross_e2); // 0.00008333333 * 1000 = 0.08333333
    if u < 0.0 || u > 1.0 { // false
        return MaybeVec3f(vec3f(), false);
    }

    let s_cross_e1 = cross(s, e1); // (-100, 1100, -1000)
    let v = inv_det * dot(direction, s_cross_e1); // 0.00008333333 * 1000 = 0.08333333
    if v < 0.0 || u + v > 1.0 { // false
        return MaybeVec3f(vec3f(), false);
    }
    // At this stage we can compute t to find out where the intersection point is on the line.
    let t = inv_det * dot(e2, s_cross_e1); // 0.00008333333 * 120000 = 9.9999996

    if t > 0.0000001 {
        // ray intersection
        let intersection_point = origin + direction * t; // (0, 0, 0) + (0, 0, -1) * 9.9999996 = (0, 0, -9.9999996)
        return MaybeVec3f(intersection_point, true);
    } else {
        // This means that there is a line intersection but not a ray intersection.
        return MaybeVec3f(vec3f(), false);
    }
}
