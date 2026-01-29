use std::{f64::consts::TAU, ops::Sub, rc::Rc};

use glam::{DAffine3, DVec3, dvec3};
use ratatui_core::style::Color;
use stl::BinaryStlFile;

#[derive(Clone)]
pub struct Shape3D {
    points: Vec<Rc<DVec3>>,
    faces: Vec<Face>,
}

impl Shape3D {
    pub fn from_stl(stl: BinaryStlFile, transform: DAffine3) -> Self {
        let transformed_triangles = stl
            .triangles
            .iter()
            .map(|triangle| {
                [
                    transform.transform_point3(dvec3(
                        triangle.v1[0] as f64,
                        triangle.v1[1] as f64,
                        triangle.v1[2] as f64,
                    )),
                    transform.transform_point3(dvec3(
                        triangle.v2[0] as f64,
                        triangle.v2[1] as f64,
                        triangle.v2[2] as f64,
                    )),
                    transform.transform_point3(dvec3(
                        triangle.v3[0] as f64,
                        triangle.v3[1] as f64,
                        triangle.v3[2] as f64,
                    )),
                ]
            })
            .collect::<Vec<[DVec3; 3]>>();

        let points = transformed_triangles
            .iter()
            .flatten()
            .copied()
            .map(|point| Rc::new(point))
            .collect::<Vec<Rc<DVec3>>>();

        let faces = transformed_triangles
            .iter()
            .map(|triangle| {
                let point_1 = Rc::clone(
                    points
                        .iter()
                        .find(|point| (point.sub(triangle[0])).length() < 0.01)
                        .unwrap(),
                );

                let point_2 = Rc::clone(
                    points
                        .iter()
                        .find(|point| (point.sub(triangle[1])).length() < 0.01)
                        .unwrap(),
                );

                let point_3 = Rc::clone(
                    points
                        .iter()
                        .find(|point| (point.sub(triangle[2])).length() < 0.01)
                        .unwrap(),
                );

                Face::new([point_1, point_2, point_3], Color::White)
            })
            .collect::<Vec<Face>>();

        Self { points, faces }
    }

    pub fn triangular_pyramid(t: f64) -> Self {
        let points = vec![
            Rc::new(dvec3(t.cos(), -1.0, -10.0 + t.sin())),
            Rc::new(dvec3(
                (t + TAU / 3.0).cos(),
                -1.0,
                -10.0 + (t + TAU / 3.0).sin(),
            )),
            Rc::new(dvec3(
                (t + TAU * 2.0 / 3.0).cos(),
                -1.0,
                -10.0 + (t + TAU * 2.0 / 3.0).sin(),
            )),
            Rc::new(dvec3(0.0, 1.0, -10.0)),
        ];

        let faces = vec![
            Face::new(
                [
                    Rc::clone(&points[0]),
                    Rc::clone(&points[1]),
                    Rc::clone(&points[2]),
                ],
                Color::Red,
            ),
            Face::new(
                [
                    Rc::clone(&points[0]),
                    Rc::clone(&points[1]),
                    Rc::clone(&points[3]),
                ],
                Color::Green,
            ),
            Face::new(
                [
                    Rc::clone(&points[0]),
                    Rc::clone(&points[2]),
                    Rc::clone(&points[3]),
                ],
                Color::Blue,
            ),
            Face::new(
                [
                    Rc::clone(&points[1]),
                    Rc::clone(&points[2]),
                    Rc::clone(&points[3]),
                ],
                Color::White,
            ),
        ];

        Self { points, faces }
    }

    pub fn faces(&self) -> &Vec<Face> {
        &self.faces
    }
}

#[derive(Clone)]
pub struct Face {
    points: [Rc<DVec3>; 3],
    color: Color,
}

impl Face {
    pub fn new(points: [Rc<DVec3>; 3], color: Color) -> Self {
        Self { points, color }
    }

    pub fn points(&self) -> &[Rc<DVec3>; 3] {
        &self.points
    }

    pub fn color(&self) -> Color {
        self.color
    }
}
