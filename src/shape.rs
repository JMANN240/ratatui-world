use crate::triangle::Triangle;
use glam::{Affine3, Vec3};
use gltf::{Document, buffer::Data, mesh::Mode};
use ratatui_core::style::Color;

use crate::triangle::ColoredTriangle;

#[derive(Debug, Clone, PartialEq)]
pub struct Shape3D {
    triangles: Vec<ColoredTriangle>,
    transform: Affine3,
}

impl Shape3D {
    pub fn new(triangles: Vec<ColoredTriangle>, transform: Affine3) -> Self {
        Self {
            triangles,
            transform,
        }
    }

    pub fn from_gltf(document: Document, buffers: Vec<Data>, transform: Affine3) -> Self {
        let mut points = vec![];
        let mut triangles = vec![];

        if let Some(scene) = document.default_scene() {
            for node in scene.nodes() {
                let transform = node.transform();
                let (translation, _rotation, _scale) = transform.decomposed();

                if let Some(mesh) = node.mesh() {
                    for primitive in mesh.primitives() {
                        // Only handle triangle lists in this example.
                        if primitive.mode() != Mode::Triangles {
                            continue;
                        }

                        let color_f32 = primitive
                            .material()
                            .pbr_metallic_roughness()
                            .base_color_factor();
                        let color = Color::Rgb(
                            (color_f32[0] * 255.0) as u8,
                            (color_f32[1] * 255.0) as u8,
                            (color_f32[2] * 255.0) as u8,
                        );

                        let reader = primitive.reader(|b| Some(&buffers[b.index()]));

                        // Record base index before pushing this primitive's vertices.
                        let base = points.len() as u32;

                        // Positions
                        let Some(positions) = reader.read_positions() else {
                            continue;
                        };
                        for p in positions {
                            let v = Vec3::new(
                                p[0] + translation[0],
                                p[1] + translation[1],
                                p[2] + translation[2],
                            );
                            points.push(v);
                        }

                        // Indices (faces)
                        if let Some(indices) = reader.read_indices() {
                            let idx: Vec<u32> = indices.into_u32().collect();
                            for tri in idx.chunks_exact(3) {
                                triangles.push(ColoredTriangle::new(
                                    Triangle::new([
                                        points[(base + tri[0]) as usize],
                                        points[(base + tri[1]) as usize],
                                        points[(base + tri[2]) as usize],
                                    ]),
                                    color,
                                ));
                            }
                        }
                    }
                }
            }
        }

        Self {
            triangles,
            transform,
        }
    }

    pub fn set_transform(&mut self, new_transform: Affine3) {
        self.transform = new_transform;
    }

    pub fn triangles(&self) -> impl Iterator<Item = ColoredTriangle> {
        self.triangles
            .iter()
            .map(|triangle| triangle.transform(self.transform))
    }

    pub fn triangles_mut(&mut self) -> impl Iterator<Item = &mut ColoredTriangle> {
        self.triangles.iter_mut()
    }
}
