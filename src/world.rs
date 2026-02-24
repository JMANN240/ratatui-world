use std::collections::HashMap;

use crate::{shape::Shape3D, triangle::ColoredTriangle};

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    shapes: HashMap<String, Shape3D>,
}

impl World {
    pub fn new(shapes: HashMap<String, Shape3D>) -> Self {
        Self {
            shapes,
        }
    }

    pub fn shapes(&self) -> &HashMap<String, Shape3D> {
        &self.shapes
    }

    pub fn shapes_mut(&mut self) -> &mut HashMap<String, Shape3D> {
        &mut self.shapes
    }

    pub fn triangles(&self) -> impl Iterator<Item = ColoredTriangle> {
        self.shapes().iter().flat_map(|(_name, shape)| shape.triangles())
    }
}