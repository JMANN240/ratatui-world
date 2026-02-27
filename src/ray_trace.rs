use std::f32::consts::E;

use crate::{moller_trumbore_intersection, plane::Plane};
use bvh::bvh::Bvh;
use bytemuck::{Pod, Zeroable};
use chrono::Utc;
use flume::bounded;
use glam::{Mat4, Quat, U16Vec2, Vec3, Vec3A, vec2, vec3a};
use rand::random_bool;
use ratatui_core::{buffer::Buffer, layout::Rect, style::Color, symbols::Marker, widgets::Widget};
use ratatui_widgets::canvas::{Canvas, Points};
use rayon::prelude::*;
use tracing::debug;
use wgpu::{
    ComputePipeline, Device, Queue,
    util::{BufferInitDescriptor, DeviceExt},
};

use crate::{ray::Ray, triangle::ColoredTriangle, world::World};

#[derive(Debug, Clone)]
pub struct RayTrace {
    world: World,
    aspect_ratio: f32,
    fov_y: f32,
    position: Vec3,
    theta: f32,
    phi: f32,
    device: Device,
    queue: Queue,
    pipeline: ComputePipeline,
    pub bvh: Bvh<f32, 3>,
    pub first_render: bool,
}

impl RayTrace {
    pub async fn new(
        mut world: World,
        aspect_ratio: f32,
        fov_y: f32,
        position: Vec3,
        theta: f32,
        phi: f32,
    ) -> Self {
        let instance = wgpu::Instance::default();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();

        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ratatui world pipeline"),
            layout: None,
            module: &shader,
            entry_point: None,
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        let mut shapes = world
            .triangles_mut()
            .map(|triangle| triangle.inner_mut())
            .collect::<Vec<_>>();

        let bvh = Bvh::build(shapes.as_mut_slice());

        Self {
            world,
            aspect_ratio,
            fov_y,
            position,
            theta,
            phi,
            device,
            queue,
            pipeline,
            bvh,
            first_render: true,
        }
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn fov_x(&self) -> f32 {
        self.fov_y * self.aspect_ratio
    }

    pub fn untransformed_left_side_vector(&self) -> Vec3 {
        Vec3::NEG_Z.rotate_y(self.fov_x() / 2.0)
    }

    pub fn untransformed_right_side_vector(&self) -> Vec3 {
        Vec3::NEG_Z.rotate_y(-self.fov_x() / 2.0)
    }

    pub fn untransformed_top_side_vector(&self) -> Vec3 {
        Vec3::NEG_Z.rotate_x(self.fov_y / 2.0)
    }

    pub fn untransformed_bottom_side_vector(&self) -> Vec3 {
        Vec3::NEG_Z.rotate_x(-self.fov_y / 2.0)
    }

    pub fn left_side_vector(&self) -> Vec3 {
        self.untransformed_left_side_vector()
            .rotate_x(self.phi())
            .rotate_y(self.theta())
    }

    pub fn right_side_vector(&self) -> Vec3 {
        self.untransformed_right_side_vector()
            .rotate_x(self.phi())
            .rotate_y(self.theta())
    }

    pub fn top_side_vector(&self) -> Vec3 {
        self.untransformed_top_side_vector()
            .rotate_x(self.phi())
            .rotate_y(self.theta())
    }

    pub fn bottom_side_vector(&self) -> Vec3 {
        self.untransformed_bottom_side_vector()
            .rotate_x(self.phi())
            .rotate_y(self.theta())
    }

    pub fn depth(&self, resolution_y: usize) -> f32 {
        let scale = (resolution_y as f32 / 2.0) / (self.fov_y / 2.0).sin();

        (self.fov_y / 2.0).cos() * scale
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

    pub fn vertical_planes(&self) -> Vec<Plane> {
        Plane::between_vectors(
            5,
            self.position(),
            self.left_side_vector(),
            self.right_side_vector(),
        )
    }

    pub fn horizontal_planes(&self) -> Vec<Plane> {
        Plane::between_vectors(
            5,
            self.position(),
            self.bottom_side_vector(),
            self.top_side_vector(),
        )
    }
}

impl Widget for &RayTrace {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        debug!("start {:?}", Utc::now());
        let character_x_resolution = 2; // how many subpixels there are along the width of a character
        let character_y_resolution = 4; // how many subpixels there are along the height of a character

        // lets say that the canvas is 60 characters wide and 60 characters tall

        let resolution_x = area.width * character_x_resolution; // 60 * 2 = 120 for quadrant
        let resolution_y = area.height * character_y_resolution; // 60 * 2 = 120 for quadrant

        // so we really have 120 pixels to work with both ways with quadrant

        let cell_aspect_ratio = 1.0 / 2.0;

        // but the characters are twice as tall as they are width, meaning that the whole "square" canvas is actually a rectangle

        let aspect_ratio =
            cell_aspect_ratio * character_y_resolution as f32 / character_x_resolution as f32;

        // So we get the aspect ratio of each pixel

        // 0.5 * 1 / 1 = 0.5 in the case of a block
        // 0.5 * 2 / 2 = 0.5 in the case of a quadrant
        // 0.5 * 4 / 2 = 1.0 in the case of braille

        let width = resolution_x as f32; // The space on the screen is as wide as there are pixels. 120 for quadrant
        let height = (resolution_y as f32) / aspect_ratio; // The space on the screen is as tall as there are pixels divided by the AR. 120 / 0.5 = 240 for quadrant

        let cell_spacing_x = width / (area.width as f32); // For quadrant it is 120 / 60 = 2, so each cell should be 2 apart width-wise
        let cell_spacing_y = height / (area.height as f32); // For quadrant it is 240 / 60 = 4, so each cell should be 4 apart height-wise

        let pixel_spacing_x = cell_spacing_x / (character_x_resolution as f32); // For quadrant it is 2 / 2 = 1, so each pixel should be 1 apart width-wise
        let pixel_spacing_y = cell_spacing_y / (character_y_resolution as f32); // For quadrant it is 4 / 2 = 2, so each pixel should be 2 apart height-wise

        let left = -width / 2.0; // -120 / 2 = -60
        let right = width / 2.0; // 120 / 2 = 60
        let up = height / 2.0; // 240 / 2 = 120
        let down = -height / 2.0; // -240 / 2 = -120

        let depth = self.depth(resolution_y as usize);

        let transformation_matrix =
            Mat4::from_scale_rotation_translation(Vec3::ONE, self.quaternion(), Vec3::ZERO);

        debug!("start buffer stuff {:?}", Utc::now());
        let params = Params {
            width: resolution_x as u32,
            height: resolution_y as u32,
        };

        let params_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("params buffer"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        debug!("params made {:?}", Utc::now());
        let camera = Camera {
            position: self.position(),
            theta: self.theta(),
            phi: self.phi(),
            _pad: Vec3::ZERO,
        };

        let camera_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("camera buffer"),
            contents: bytemuck::cast_slice(&[camera]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        debug!("camera made {:?}", Utc::now());
        let mut rays_data = vec![];
        let mut screen_vecs = vec![];

        for cell_y_index in 0..area.height {
            let ray_y_base = down + cell_y_index as f32 * cell_spacing_y + pixel_spacing_y / 2.0;

            for ray_y_index in 0..character_y_resolution {
                let ray_y_offset = ray_y_index as f32 * pixel_spacing_y;

                for cell_x_index in 0..area.width {
                    let ray_x_base =
                        left + cell_x_index as f32 * cell_spacing_x + pixel_spacing_x / 2.0;

                    for ray_x_index in 0..character_x_resolution {
                        let ray_x_offset = ray_x_index as f32 * pixel_spacing_x;

                        let ray_x = ray_x_base + ray_x_offset;
                        let ray_y = ray_y_base + ray_y_offset;

                        rays_data.push(
                            transformation_matrix.transform_vector3a(vec3a(ray_x, ray_y, -depth)),
                        );

                        screen_vecs.push(vec2(ray_x, ray_y));
                    }
                }
            }
        }

        let rays_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("rays"),
            contents: bytemuck::cast_slice(&rays_data),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        });

        debug!("rays made {:?}", Utc::now());

        let flat_bvh = self
            .bvh
            .flatten()
            .into_iter()
            .map(|node| FlatBVHNode {
                aabb: AABB {
                    min: vec3a(node.aabb.min.x, node.aabb.min.y, node.aabb.min.z),
                    max: vec3a(node.aabb.max.x, node.aabb.max.y, node.aabb.max.z),
                },
                entry_index: node.entry_index,
                exit_index: node.exit_index,
                shape_index: node.shape_index,
                _pad: 0,
            })
            .collect::<Vec<_>>();

        let bvh_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("bvh"),
            contents: bytemuck::cast_slice(&flat_bvh),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        });

        let triangles_data = self
            .world
            .triangles()
            .filter_map(|colored_triangle| {
                if let Color::Rgb(r, g, b) = colored_triangle.color() {
                    Some(Triangle {
                        points: colored_triangle.points().map(Vec3::to_vec3a),
                        color: Rgb {
                            red: r as u32,
                            green: g as u32,
                            blue: b as u32,
                        },
                        _pad: 0,
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let triangles_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("triangles"),
            contents: bytemuck::cast_slice(&triangles_data),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        });

        debug!("truiangles made {:?}", Utc::now());
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("output"),
            size: rays_data.len() as u64 * 16,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        debug!("output made {:?}", Utc::now());
        let temp_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("temp"),
            size: rays_data.len() as u64 * 16,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        debug!("temp made {:?}", Utc::now());
        debug!("start bind group {:?}", Utc::now());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: rays_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: bvh_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: triangles_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        debug!("start encoder {:?}", Utc::now());
        let mut encoder = self.device.create_command_encoder(&Default::default());

        debug!("dispatch {:?}", Utc::now());
        {
            let num_x_dispatches = resolution_x.div_ceil(16) as u32;
            let num_y_dispatches = resolution_y.div_ceil(16) as u32;

            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(num_x_dispatches, num_y_dispatches, 1);
        }
        debug!("done? {:?}", Utc::now());

        encoder.copy_buffer_to_buffer(&output_buffer, 0, &temp_buffer, 0, output_buffer.size());
        debug!("copied {:?}", Utc::now());

        self.queue.submit([encoder.finish()]);
        debug!("submitted {:?}", Utc::now());

        self.queue.on_submitted_work_done(|| {
            debug!("on_submitted_work_done {:?}", Utc::now());
        });

        debug!("mapping {:?}", Utc::now());
        {
            // The mapping process is async, so we'll need to create a channel to get
            // the success flag for our mapping
            let (tx, rx) = bounded(1);

            debug!("mapping2 {:?}", Utc::now());
            // We send the success or failure of our mapping via a callback
            temp_buffer.map_async(wgpu::MapMode::Read, .., move |result| {
                tx.send(result).unwrap()
            });

            debug!("mapping3 {:?}", Utc::now());
            // The callback we submitted to map async will only get called after the
            // device is polled or the queue submitted
            self.device
                .poll(wgpu::PollType::wait_indefinitely())
                .unwrap();

            debug!("mapping4 {:?}", Utc::now());
            // We check if the mapping was successful here
            rx.recv().unwrap().unwrap();

            debug!("mapping5 {:?}", Utc::now());
            // We then get the bytes that were stored in the buffer
            let output_data = temp_buffer.get_mapped_range(..);

            debug!("mapping6 {:?}", Utc::now());
            let intersections: &[DumbIntersection] = bytemuck::cast_slice(&output_data);

            debug!("mapped {:?}", Utc::now());

            let canvas = Canvas::default()
                .marker(Marker::Braille)
                .x_bounds([left as f64, right as f64])
                .y_bounds([down as f64, up as f64])
                .paint(|context| {
                    for (intersection, screen_vec) in intersections.iter().zip(screen_vecs.iter()) {
                        let normalized_distance = E.powf(-0.01 * intersection.distance.powi(2));

                        if random_bool(normalized_distance as f64) {
                            context.draw(&Points::new(
                                &[(screen_vec.x as f64, screen_vec.y as f64)],
                                Color::Rgb(
                                    (intersection.color.red as f32 * normalized_distance) as u8,
                                    (intersection.color.green as f32 * normalized_distance) as u8,
                                    (intersection.color.blue as f32 * normalized_distance) as u8,
                                ),
                            ));
                        }
                    }
                });

            debug!("drawn {:?}", Utc::now());
            canvas.render(area, buf);
            debug!("rendered {:?}", Utc::now());
        }

        // We need to unmap the buffer to be able to use it again
        temp_buffer.unmap();
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Params {
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Camera {
    pub position: Vec3,
    pub theta: f32,
    pub phi: f32,
    _pad: Vec3,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Rgb {
    pub red: u32,
    pub green: u32,
    pub blue: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Triangle {
    pub points: [Vec3A; 3],
    pub color: Rgb,
    _pad: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct DumbIntersection {
    distance: f32,
    color: Rgb,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct AABB {
    min: Vec3A,
    max: Vec3A,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct FlatBVHNode {
    aabb: AABB,
    entry_index: u32,
    exit_index: u32,
    shape_index: u32,
    _pad: u32,
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
        _screen: &Screen,
    ) -> Vec<Intersection> {
        let vertical_planes = ray_trace.vertical_planes();
        let horizontal_planes = ray_trace.horizontal_planes();

        let triangle_vertical_partition_indices = world
            .triangles()
            .par_bridge()
            .map(|triangle| triangle.partition_indices(&vertical_planes))
            .collect::<Vec<_>>();

        let triangle_horizontal_partition_indices = world
            .triangles()
            .par_bridge()
            .map(|triangle| triangle.partition_indices(&horizontal_planes))
            .collect::<Vec<_>>();

        let mut ray_triangle_intersections = self
            .rays
            .iter()
            .filter_map(|ray| {
                let maybe_ray_vertical_plane_partition_index =
                    ray.partition_index(&vertical_planes);
                let maybe_ray_horizontal_plane_partition_index =
                    ray.partition_index(&horizontal_planes);

                world
                    .triangles()
                    .enumerate()
                    .filter_map(|(index, triangle)| {
                        let maybe_triangle_vertical_plane_partition_indices =
                            triangle_vertical_partition_indices.get(index).unwrap();
                        let maybe_triangle_horizontal_plane_partition_indices =
                            triangle_horizontal_partition_indices.get(index).unwrap();

                        let ray_might_vertically_intersect_triangle =
                            maybe_ray_vertical_plane_partition_index.is_none_or(
                                |ray_vertical_plane_partition_index| {
                                    maybe_triangle_vertical_plane_partition_indices.is_none_or(
                                        |(
                                            lower_triangle_vertical_plane_partition_index,
                                            upper_triangle_vertical_plane_partition_index,
                                        )| {
                                            ray_vertical_plane_partition_index
                                                >= lower_triangle_vertical_plane_partition_index
                                                && ray_vertical_plane_partition_index
                                                    <= upper_triangle_vertical_plane_partition_index
                                        },
                                    )
                                },
                            );

                        let ray_might_horizontally_intersect_triangle =
                            maybe_ray_horizontal_plane_partition_index.is_none_or(
                                |ray_horizontal_plane_partition_index| {
                                    maybe_triangle_horizontal_plane_partition_indices.is_none_or(
                                        |(
                                            lower_triangle_horizontal_plane_partition_index,
                                            upper_triangle_horizontal_plane_partition_index,
                                        )| {
                                            ray_horizontal_plane_partition_index
                                                >= lower_triangle_horizontal_plane_partition_index
                                                && ray_horizontal_plane_partition_index
                                                    <= upper_triangle_horizontal_plane_partition_index
                                        },
                                    )
                                },
                            );

                        let ray_might_intersect_triangle = ray_might_vertically_intersect_triangle && ray_might_horizontally_intersect_triangle;

                        if ray_might_intersect_triangle {
                            moller_trumbore_intersection(
                                ray_trace.position(),
                                ray.world_vector(),
                                *triangle.points(),
                            )
                            .map(|intersection| Intersection {
                                ray: *ray,
                                triangle,
                                intersection,
                            })
                        } else {
                            None
                        }
                    })
                    .min_by(|l, r| {
                        (ray_trace.position() - l.intersection())
                            .length()
                            .partial_cmp(&(ray_trace.position() - r.intersection()).length())
                            .unwrap()
                    })
            })
            .collect::<Vec<_>>();

        ray_triangle_intersections.sort_by(|l, r| {
            (ray_trace.position() - r.intersection())
                .length()
                .partial_cmp(&(ray_trace.position() - l.intersection()).length())
                .unwrap()
        });

        ray_triangle_intersections
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Intersection {
    ray: Ray,
    triangle: ColoredTriangle,
    intersection: Vec3,
}

impl Intersection {
    pub fn ray(&self) -> Ray {
        self.ray
    }

    pub fn triangle(&self) -> ColoredTriangle {
        self.triangle
    }

    pub fn intersection(&self) -> Vec3 {
        self.intersection
    }
}

pub struct Screen {
    left: f32,
    right: f32,
    up: f32,
    down: f32,
    width: f32,
    height: f32,
}

impl Screen {
    pub fn new(left: f32, right: f32, up: f32, down: f32) -> Self {
        Screen {
            left,
            right,
            up,
            down,
            width: right - left,
            height: up - down,
        }
    }

    pub fn left(&self) -> f32 {
        self.left
    }

    pub fn right(&self) -> f32 {
        self.right
    }

    pub fn up(&self) -> f32 {
        self.up
    }

    pub fn down(&self) -> f32 {
        self.down
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }
}

#[cfg(test)]
mod tests {
    use glam::vec3;

    use super::*;

    #[test]
    fn test_moller_trumbore() {
        let position = Vec3::ZERO;
        let direction = Vec3::NEG_Z;
        let triangle = [
            vec3(-1.0, -1.0, -2.0),
            vec3(0.0, 1.0, -2.0),
            vec3(1.0, -1.0, -2.0),
        ];

        assert_eq!(
            moller_trumbore_intersection(position, direction, triangle),
            Some(vec3(0.0, 0.0, -2.0))
        );

        let position = vec3(0.0, 0.0, 10.0);

        assert_eq!(
            moller_trumbore_intersection(position, direction, triangle),
            Some(vec3(0.0, 0.0, -2.0))
        );

        let position = vec3(0.0, 0.0, -10.0);

        assert_eq!(
            moller_trumbore_intersection(position, direction, triangle),
            None
        );
    }
}
