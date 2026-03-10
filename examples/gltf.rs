use std::{
    collections::HashMap,
    f32::consts::{PI, TAU},
    fs::File,
    time::Duration,
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use glam::{Affine3, Quat, Vec3};
use ratatui::{DefaultTerminal, Frame, widgets::Widget};
use ratatui_world::{ray_trace::RayTrace, shape::Shape3D, world::World};
use tracing_subscriber::{Registry, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let file = File::create("debug.log").unwrap();

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file) // Direct output to the file
        .with_ansi(false); // Set the maximum log level for this writer

    Registry::default().with(file_layer).init();

    let mut app = App::new().await;

    ratatui::run(|terminal| app.run(terminal))
}

pub struct App {
    t: f32,
    camera: RayTrace,
    exit: bool,
    target_position: Vec3,
    target_theta: f32,
    target_phi: f32,
}

impl App {
    pub async fn new() -> Self {
        let mut shapes = HashMap::new();

        let (monkey_document, monkey_buffers, _monkey_images) = gltf::import("monkey.glb").unwrap();
        let (room_document, room_buffers, _room_images) = gltf::import("room.glb").unwrap();

        let monkey = Shape3D::from_gltf(
            monkey_document,
            monkey_buffers,
            Affine3::from_scale_rotation_translation(Vec3::ONE, Quat::IDENTITY, Vec3::NEG_Z * 4.0),
        );

        let room = Shape3D::from_gltf(
            room_document,
            room_buffers,
            Affine3::from_scale_rotation_translation(Vec3::ONE, Quat::IDENTITY, Vec3::ZERO),
        );

        shapes.insert(String::from("monkey"), monkey);
        shapes.insert(String::from("room"), room);

        Self {
            t: f32::default(),
            camera: RayTrace::new(
                World::new(shapes),
                1.0,
                PI / 4.0,
                Vec3::default(),
                f32::default(),
                f32::default(),
            )
            .await,
            exit: bool::default(),
            target_position: Vec3::default(),
            target_theta: f32::default(),
            target_phi: f32::default(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        while !self.exit {
            self.camera.first_render = (self.t % 1.0) < 0.01;
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
            self.t += 1.0 / 120.0;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> std::io::Result<()> {
        if event::poll(Duration::from_secs_f64(1.0 / 120.0))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event)
                }
                _ => {}
            };
        }

        let world = self.camera.world_mut();

        world
            .shapes_mut()
            .entry(String::from("monkey"))
            .and_modify(|shape| {
                shape.set_transform(Affine3::from_rotation_translation(
                    Quat::from_rotation_y(self.t * 4.0),
                    Vec3::NEG_Z * 4.0 + 0.1 * Vec3::Y * (self.t * 7.0).sin(),
                ))
            });

        self.camera.set_position(self.camera.position() + (self.target_position - self.camera.position()) * 0.1);
        self.camera.set_theta(self.camera.theta() + (self.target_theta - self.camera.theta()) * 0.1);
        self.camera.set_phi(self.camera.phi() + (self.target_phi - self.camera.phi()) * 0.1);

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => self.exit(),
            KeyCode::Left => self.target_theta = self.target_theta + TAU / 120.0,
            KeyCode::Right => self.target_theta = self.target_theta - TAU / 120.0,
            KeyCode::Up => self.target_phi = self.target_phi + TAU / 240.0,
            KeyCode::Down => self.target_phi = self.target_phi - TAU / 240.0,
            KeyCode::Char('w') => self.target_position = self.target_position + self.camera.facing(),
            KeyCode::Char('a') => self.target_position = self.target_position - self.camera.right(),
            KeyCode::Char('s') => self.target_position = self.target_position - self.camera.facing(),
            KeyCode::Char('d') => self.target_position = self.target_position + self.camera.right(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &App {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        self.camera.render(area, buf);
    }
}
