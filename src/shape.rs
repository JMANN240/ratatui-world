use std::{f64::consts::TAU, ops::Sub, process::exit, rc::Rc};

use glam::{DAffine3, DVec3, dvec3};
use gltf::{Document, Gltf, Semantic, buffer::Data, import_buffers, mesh::Mode};
use ratatui_core::style::Color;
use stl::BinaryStlFile;

#[derive(Debug, Clone, PartialEq)]
pub struct Shape3D {
    triangles: Vec<Triangle>,
}

impl Shape3D {
    pub fn from_gltf(document: Document, buffers: Vec<Data>, transform: DAffine3) -> Self {
        let mut points = vec![];
        let mut triangles = vec![];

        if let Some(scene) = document.default_scene() {
            for node in scene.nodes() {
                let transform = node.transform();
                let (translation, rotation, scale) = transform.decomposed();

                if let Some(mesh) = node.mesh() {
                    for primitive in mesh.primitives() {
                        // Only handle triangle lists in this example.
                        if primitive.mode() != Mode::Triangles {
                            continue;
                        }

                        let color_f32 = primitive.material().pbr_metallic_roughness().base_color_factor();
                        let color = Color::Rgb((color_f32[0] * 255.0) as u8, (color_f32[1] * 255.0) as u8, (color_f32[2] * 255.0) as u8);

                        let reader = primitive.reader(|b| Some(&buffers[b.index()]));

                        // Record base index before pushing this primitive's vertices.
                        let base = points.len() as u32;

                        // Positions
                        let Some(positions) = reader.read_positions() else {
                            continue;
                        };
                        for p in positions {
                            let v = DVec3::new(p[0] as f64 + translation[0] as f64, p[1] as f64 + translation[1] as f64, p[2] as f64 + translation[2] as f64);
                            points.push(v);
                        }

                        // Indices (faces)
                        if let Some(indices) = reader.read_indices() {
                            let idx: Vec<u32> = indices.into_u32().collect();
                            for tri in idx.chunks_exact(3) {
                                triangles.push(Triangle::new(
                                    [
                                        points[(base + tri[0]) as usize],
                                        points[(base + tri[1]) as usize],
                                        points[(base + tri[2]) as usize],
                                    ],
                                    color,
                                ));
                            }
                        }

                        if let Some(colors) = reader.read_colors(16) {
                            for color in colors.into_rgb_u8() {
                                println!("{:?}", color);
                            }
                        }
                    }
                }
            }
        }

        Self { triangles }
    }

    pub fn triangles(&self) -> &Vec<Triangle> {
        &self.triangles
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    points: [DVec3; 3],
    color: Color,
}

impl Triangle {
    pub fn new(points: [DVec3; 3], color: Color) -> Self {
        Self { points, color }
    }

    pub fn points(&self) -> &[DVec3; 3] {
        &self.points
    }

    pub fn color(&self) -> Color {
        self.color
    }
}
