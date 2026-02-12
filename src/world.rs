use crate::shape::{Shape3D, Triangle};

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    shapes: Vec<Shape3D>,
}

impl World {
    pub fn new(shapes: Vec<Shape3D>) -> Self {
        Self {
            shapes,
        }
    }

    pub fn shapes(&self) -> &Vec<Shape3D> {
        &self.shapes
    }

    pub fn triangles(&self) -> impl Iterator<Item = &Triangle> {
        self.shapes().iter().flat_map(|shape| shape.triangles().iter())
    }
}