use std::{
    f64::consts::{PI, TAU},
    fs::File,
    process::exit,
    time::Duration,
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use glam::{DAffine3, DVec3, dvec3};
use gltf::{Document, Gltf, buffer::Data};
use ratatui::{
    DefaultTerminal, Frame,
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, Widget},
};
use ratatui_world::{ray_trace::RayTrace, shape::Shape3D, world::World};
use tracing_subscriber::{Registry, layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> std::io::Result<()> {
    let file = File::create("debug.log").unwrap();

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file) // Direct output to the file
        .with_ansi(false); // Set the maximum log level for this writer

    Registry::default()
        .with(file_layer)
        .init();

    let (document, buffers, _images) = gltf::import("monkey.glb").unwrap();

    ratatui::run(|terminal| App::new(document, buffers).run(terminal))
}

pub struct App {
    t: f64,
    camera: RayTrace,
    exit: bool,
    render_depth: f64,
}

impl App {
    pub fn new(document: Document, buffers: Vec<Data>) -> Self {
        Self {
            t: f64::default(),
            camera: RayTrace::new(
                World::new(vec![Shape3D::from_gltf(
                    document,
                    buffers,
                    DAffine3::from_axis_angle(DVec3::Y, -PI / 2.0),
                )]),
                1.0,
                PI / 4.0,
                DVec3::default(),
                f64::default(),
                f64::default(),
            ),
            exit: bool::default(),
            render_depth: 10.0,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        while !self.exit {
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
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => self.exit(),
            KeyCode::Left => self.camera.set_theta(self.camera.theta() + TAU / 12.0),
            KeyCode::Right => self.camera.set_theta(self.camera.theta() - TAU / 12.0),
            KeyCode::Up => self.camera.set_phi(self.camera.phi() + TAU / 24.0),
            KeyCode::Down => self.camera.set_phi(self.camera.phi() - TAU / 24.0),
            KeyCode::Char('w') => self
                .camera
                .set_position(self.camera.position() + self.camera.facing()),
            KeyCode::Char('a') => self
                .camera
                .set_position(self.camera.position() - self.camera.right()),
            KeyCode::Char('s') => self
                .camera
                .set_position(self.camera.position() - self.camera.facing()),
            KeyCode::Char('d') => self
                .camera
                .set_position(self.camera.position() + self.camera.right()),
            KeyCode::Char('q') => self.render_depth -= 0.5,
            KeyCode::Char('e') => self.render_depth += 0.5,
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
        let title = Line::from(" STL ".bold());
        let instructions = Line::from(vec![" Quit ".into(), "<ESC> ".blue().bold()]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        self.camera.render(area, buf);
    }
}
