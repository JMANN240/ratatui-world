use glam::{DVec3, Vec3Swizzles};
use rand::random_bool;
use ratatui_canvas_polygon::triangle::Triangle;
use ratatui_core::{style::Color, symbols::Marker, widgets::Widget};
use ratatui_widgets::{
    block::Block,
    canvas::{Canvas, Points},
};

use crate::{camera::Camera, shape::Shape3D};

pub mod camera;
pub mod shape;

pub struct World<'a> {
    camera: Camera,
    block: Option<Block<'a>>,
    marker: Marker,
    shapes: Vec<Shape3D>,
    render_depth: f64,
}

impl<'a> World<'a> {
    pub fn new(camera: Camera, render_depth: f64) -> Self {
        Self {
            camera,
            block: None,
            marker: Marker::default(),
            shapes: vec![],
            render_depth,
        }
    }

    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    #[must_use = "method moves the value of self and returns the modified value"]
    pub const fn marker(mut self, marker: Marker) -> Self {
        self.marker = marker;
        self
    }

    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn add_shape(mut self, shape: Shape3D) -> Self {
        self.shapes.push(shape);
        self
    }
}

impl<'a> Widget for World<'a> {
    fn render(self, area: ratatui_core::layout::Rect, buf: &mut ratatui_core::buffer::Buffer)
    where
        Self: Sized,
    {
        Widget::render(&self, area, buf);
    }
}

impl<'a> Widget for &World<'a> {
    fn render(self, area: ratatui_core::layout::Rect, buf: &mut ratatui_core::buffer::Buffer)
    where
        Self: Sized,
    {
        let bounds_width = 1.0;
        let aspect_ratio = area.width as f64 / area.height as f64 / 2.0;
        let bounds_height = bounds_width / aspect_ratio;

        let character_x_resolution = 2;
        let character_y_resolution = 4;

        let mut canvas = Canvas::default()
            .x_bounds([-bounds_width, bounds_width])
            .y_bounds([-bounds_height, bounds_height])
            .paint(|context| {
                for shape in self.shapes.iter() {
                    let mut sorted_faces = shape.faces().clone();
                    sorted_faces.sort_by_key(|face| {
                        (self
                            .camera
                            .transform_world_point(*face.points()[0])
                            .midpoint(self.camera.transform_world_point(*face.points()[1]))
                            .midpoint(self.camera.transform_world_point(*face.points()[2]))
                            .length()
                            * 1000.0) as i32
                    });
                    sorted_faces.reverse();

                    for face in sorted_faces {
                        let midpoint = self
                            .camera
                            .transform_world_point(*face.points()[0])
                            .midpoint(self.camera.transform_world_point(*face.points()[1]))
                            .midpoint(self.camera.transform_world_point(*face.points()[2]));

                        let chance = if Camera::should_clip(self.camera.project_camera_point(midpoint)) {
                            0.0
                        } else {
                            (midpoint.length() / self.render_depth).clamp(0.0, 1.0) * -1.0 + 1.0
                        };

                        context.draw(&Triangle::new(
                            [
                                self.camera
                                    .transform_and_project_world_point((*face.points()[0]).clone())
                                    .xy()
                                    .to_array()
                                    .into(),
                                self.camera
                                    .transform_and_project_world_point((*face.points()[1]).clone())
                                    .xy()
                                    .to_array()
                                    .into(),
                                self.camera
                                    .transform_and_project_world_point((*face.points()[2]).clone())
                                    .xy()
                                    .to_array()
                                    .into(),
                            ],
                            face.color(),
                            Some(Box::new(move |_, _| random_bool(chance.powi(2)))),
                        ));
                    }
                }
            });

        canvas = if let Some(block) = &self.block {
            canvas.block(block.clone())
        } else {
            canvas
        };

        canvas.render(area, buf);
    }
}
