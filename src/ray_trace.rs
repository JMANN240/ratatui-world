use std::{f32::consts::E, rc::Rc};

use bvh::bvh::Bvh;
use bytemuck::{Pod, Zeroable};
use chrono::Utc;
use flume::bounded;
use glam::{Mat4, Quat, U16Vec2, Vec3, Vec3A, vec2, vec3a};
use rand::random_bool;
use ratatui_core::{buffer::Buffer, layout::Rect, style::Color, symbols::Marker, widgets::Widget};
use ratatui_widgets::canvas::{Canvas, Points};
use tracing::debug;
use wgpu::{
    ComputePipeline, Device, Queue,
    util::{BufferInitDescriptor, DeviceExt},
};

use crate::world::World;

#[derive(Clone)]
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
    distance_normalization_function: Rc<dyn Fn(f32) -> f32>,
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
            .triangles()
            .map(|triangle| triangle.inner())
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
            distance_normalization_function: Rc::new(|distance| 1.0 - E.powf(-0.01 * distance.powi(2)))
        }
    }

    pub fn update_bvh(&mut self) {
        let mut shapes = self.world
            .triangles()
            .map(|triangle| triangle.inner())
            .collect::<Vec<_>>();

        self.bvh = Bvh::build(shapes.as_mut_slice());
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn fov_x(&self) -> f32 {
        self.fov_y * self.aspect_ratio
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
}

impl Widget for &RayTrace {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let character_x_resolution = 2;
        let character_y_resolution = 4;

        let resolution_x = area.width * character_x_resolution;
        let resolution_y = area.height * character_y_resolution;

        let cell_aspect_ratio = 1.0 / 2.0;

        let aspect_ratio =
            cell_aspect_ratio * character_y_resolution as f32 / character_x_resolution as f32;

        let width = resolution_x as f32;
        let height = (resolution_y as f32) / aspect_ratio;

        let cell_spacing_x = width / (area.width as f32);
        let cell_spacing_y = height / (area.height as f32);

        let pixel_spacing_x = cell_spacing_x / (character_x_resolution as f32);
        let pixel_spacing_y = cell_spacing_y / (character_y_resolution as f32);

        let left = -width / 2.0;
        let right = width / 2.0;
        let up = height / 2.0;
        let down = -height / 2.0;

        let depth = self.depth(resolution_y as usize);

        let transformation_matrix =
            Mat4::from_scale_rotation_translation(Vec3::ONE, self.quaternion(), Vec3::ZERO);

        let params = Params {
            width: resolution_x as u32,
            height: resolution_y as u32,
        };

        let params_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("params buffer"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

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
            .map(|colored_triangle| {
                    Triangle {
                        points: colored_triangle.points().map(Vec3::to_vec3a),
                        color: Rgb {
                            red: colored_triangle.color().r as u32,
                            green: colored_triangle.color().g as u32,
                            blue: colored_triangle.color().b as u32,
                        },
                        _pad: 0,
                    }
            })
            .collect::<Vec<_>>();

        let triangles_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("triangles"),
            contents: bytemuck::cast_slice(&triangles_data),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        });

        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("output"),
            size: rays_data.len() as u64 * 16,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let temp_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("temp"),
            size: rays_data.len() as u64 * 16,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

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

        let mut encoder = self.device.create_command_encoder(&Default::default());

        {
            let num_x_dispatches = resolution_x.div_ceil(16) as u32;
            let num_y_dispatches = resolution_y.div_ceil(16) as u32;

            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(num_x_dispatches, num_y_dispatches, 1);
        }

        encoder.copy_buffer_to_buffer(&output_buffer, 0, &temp_buffer, 0, output_buffer.size());

        self.queue.submit([encoder.finish()]);

        {
            let (tx, rx) = bounded(1);

            temp_buffer.map_async(wgpu::MapMode::Read, .., move |result| {
                tx.send(result).unwrap()
            });

            self.device
                .poll(wgpu::PollType::wait_indefinitely())
                .unwrap();

            rx.recv().unwrap().unwrap();

            let output_data = temp_buffer.get_mapped_range(..);

            let intersections: &[Intersection] = bytemuck::cast_slice(&output_data);


            let canvas = Canvas::default()
                .marker(Marker::Braille)
                .x_bounds([left as f64, right as f64])
                .y_bounds([down as f64, up as f64])
                .paint(|context| {
                    for (intersection, screen_vec) in intersections.iter().zip(screen_vecs.iter()) {
                        let normalized_distance = (self.distance_normalization_function)(intersection.distance);
                        let draw_probability = 1.0 - normalized_distance;

                        if random_bool(draw_probability as f64) {
                            context.draw(&Points::new(
                                &[(screen_vec.x as f64, screen_vec.y as f64)],
                                Color::Rgb(
                                    (intersection.color.red as f32 * draw_probability) as u8,
                                    (intersection.color.green as f32 * draw_probability) as u8,
                                    (intersection.color.blue as f32 * draw_probability) as u8,
                                ),
                            ));
                        }
                    }
                });

            canvas.render(area, buf);
        }

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
pub struct Intersection {
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
