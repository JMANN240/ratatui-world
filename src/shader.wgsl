struct Ray {
    origin: vec3f,
    direction: vec3f,
    inverse_direction: vec3f,
    signs: vec3<bool>,
}

fn ray_from_origin_and_direction(origin: vec3f, direction: vec3f) -> Ray {
    let inverse_direction = 1.0 / direction;

    let signs = vec3<bool>(
        inverse_direction.x < 0.0,
        inverse_direction.y < 0.0,
        inverse_direction.z < 0.0,
    );

    return Ray(origin, direction, inverse_direction, signs);
}

struct AABB {
    min: vec3f,
    max: vec3f,
}

struct BVHNode {
    aabb: AABB,
    entry_index: u32,
    exit_index: u32,
    shape_index: u32,
}

struct MaybeT {
    t: f32,
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
    color: Color,
}

struct Intersection {
    distance: f32,
    color: Color,
}

struct Color {
    red: u32,
    green: u32,
    blue: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(2) var<storage, read> rays: array<vec3f>;
@group(0) @binding(3) var<storage, read> bvh: array<BVHNode>;
@group(0) @binding(4) var<storage, read> triangles: array<Triangle>;
@group(0) @binding(5) var<storage, read_write> intersections: array<Intersection>;

@compute
@workgroup_size(16, 16)
fn main(
    @builtin(global_invocation_id) id: vec3<u32>
) {
    if id.x >= params.width || id.y >= params.height {
        return;
    }

    let index = id.x + id.y * params.width;

    let ray = ray_from_origin_and_direction(camera.position, rays[index]);

    var color = Color(0, 0, 0);
    var min_t = 1000f;

    var node_index = 0u;
    let n_nodes = arrayLength(&bvh);

    let max = 4294967295u;

    loop {
        if node_index >= n_nodes {
            break;
        }

        let node = bvh[node_index];

        if node.entry_index < max {
            let intersects = aabb_intersection(node.aabb, ray, 0, 1000);

            if intersects {
                node_index = node.entry_index;
            } else {
                node_index = node.exit_index;
            }
        } else {
            // Leaf node, check the shape and exit index
            let triangle = triangles[node.shape_index];

            let maybe_t = moller_trumbore_intersection(ray, triangle.points);

            if maybe_t.valid {
                if maybe_t.t < min_t {
                    color = triangle.color;
                    min_t = maybe_t.t;
                }
            }

            node_index = node.exit_index;
        }
    }

    intersections[index] = Intersection(min_t * length(ray.direction), color);
}

fn moller_trumbore_intersection(
    ray: Ray,
    triangle: array<vec3f, 3>,
) -> MaybeT {
    let e1 = triangle[1] - triangle[0];
    let e2 = triangle[2] - triangle[0];

    let ray_cross_e2 = cross(ray.direction, e2);
    let det = dot(e1, ray_cross_e2);

    if det > -0.0000001 && det < 0.0000001 {
        return MaybeT(0f, false); // This ray is parallel to this triangle.
    }

    let inv_det = 1.0 / det;
    let s = ray.origin - triangle[0];
    let u = inv_det * dot(s, ray_cross_e2);
    if u < 0.0 || u > 1.0 {
        return MaybeT(0f, false);
    }

    let s_cross_e1 = cross(s, e1);
    let v = inv_det * dot(ray.direction, s_cross_e1);
    if v < 0.0 || u + v > 1.0 {
        return MaybeT(0f, false);
    }
    // At this stage we can compute t to find out where the intersection point is on the line.
    let t = inv_det * dot(e2, s_cross_e1);

    if t > 0.0000001 {
        // ray intersection
        return MaybeT(t, true);
    } else {
        // This means that there is a line intersection but not a ray intersection.
        return MaybeT(0f, false);
    }
}

fn aabb_intersection(aabb: AABB, ray: Ray, t0: f32, t1: f32) -> bool {
    let bounds_min = select(aabb.min, aabb.max, ray.signs.x);
    let bounds_max = select(aabb.max, aabb.min, ray.signs.x);

    var t_min = (bounds_min.x - ray.origin.x) * ray.inverse_direction.x;
    var t_max = (bounds_max.x - ray.origin.x) * ray.inverse_direction.x;

    let bounds_y_min = select(aabb.min, aabb.max, ray.signs.y);
    let bounds_y_max = select(aabb.max, aabb.min, ray.signs.y);

    let t_y_min = (bounds_y_min.y - ray.origin.y) * ray.inverse_direction.y;
    let t_y_max = (bounds_y_max.y - ray.origin.y) * ray.inverse_direction.y;

    if (t_min > t_y_max) || (t_y_min > t_max) {
        return false;
    }

    if t_y_min > t_min {
        t_min = t_y_min;
    }

    if t_y_max < t_max {
        t_max = t_y_max;
    }

    let bounds_z_min = select(aabb.min, aabb.max, ray.signs.z);
    let bounds_z_max = select(aabb.max, aabb.min, ray.signs.z);

    let t_z_min = (bounds_z_min.z - ray.origin.z) * ray.inverse_direction.z;
    let t_z_max = (bounds_z_max.z - ray.origin.z) * ray.inverse_direction.z;

    if (t_min > t_z_max) || (t_z_min > t_max) {
        return false;
    }

    if t_z_min > t_min {
        t_min = t_z_min;
    }

    if t_z_max < t_max {
        t_max = t_z_max;
    }

    return ((t_min < t1) && (t_max > t0));
}
