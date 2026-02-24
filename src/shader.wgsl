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
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>
) {
    if id.x >= params.width || id.y >= params.height {
        return;
    }

    let index = (id.x + id.y * params.width) as usize;

    let ray = rays[index];

    var least_distance = 1000;

    for (var i = 0; i < arrayLength(triangles); i++) {
        let triangle = triangles[i];

        let maybe_intersection = moller_trumbore_intersection(camera.position, ray, triangle.points);

        if maybe_intersection.valid {
            let intersection_length = length(maybe_intersection.vector);

            if intersection_length < least_distance {
                least_distance = intersection_length;
            }
        }
    }

    output[index] = least_distance;
}

fn moller_trumbore_intersection(
    origin: vec3f,
    direction: vec3f,
    triangle: array<vec3f, 3>,
) -> MaybeVec3f {
    let e1 = triangle[1] - triangle[0];
    let e2 = triangle[2] - triangle[0];

    let ray_cross_e2 = cross(direction, e2);
    let det = dot(e1, ray_cross_e2);

    if det > -0.0001 && det < 0.0001 {
        return MaybeVec3f(vec3f(), false); // This ray is parallel to this triangle.
    }

    let inv_det = 1.0 / det;
    let s = origin - triangle[0];
    let u = inv_det * dot(s, ray_cross_e2);
    if u < 0.0 || u > 1.0 {
        return MaybeVec3f(vec3f(), false);
    }

    let s_cross_e1 = cross(s, e1);
    let v = inv_det * dot(direction, s_cross_e1);
    if v < 0.0 || u + v > 1.0 {
        return MaybeVec3f(vec3f(), false);
    }
    // At this stage we can compute t to find out where the intersection point is on the line.
    let t = inv_det * dot(e2, s_cross_e1);

    if t > 0.0001 {
        // ray intersection
        let intersection_point = origin + direction * t;
        return MaybeVec3f(intersection_point, true);
    } else {
        // This means that there is a line intersection but not a ray intersection.
        return MaybeVec3f(vec3f(), false);
    }
}
