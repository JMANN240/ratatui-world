use std::{
    f32::consts::{PI, TAU}, fs::File, time::Duration
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
};
use glam::{Affine3, Vec3, vec3};
use ratatui::{
    DefaultTerminal, Frame,
    style::{Stylize},
    symbols::border,
    text::Line,
    widgets::{
        Block, Widget,
    },
};
use ratatui_world::{world::World, camera::Camera, shape::Shape3D};
use stl::{BinaryStlFile, read_stl};

fn main() -> std::io::Result<()> {
    let stl = read_stl(&mut File::open("monkey.stl").unwrap()).unwrap();

    ratatui::run(|terminal| App::new(stl).run(terminal))
}

pub struct App {
    t: f32,
    camera: Camera,
    exit: bool,
    stl_shape: Shape3D,
    render_depth: f32,
}

impl App {
    pub fn new(stl: BinaryStlFile) -> Self {
        Self {
            t: f32::default(),
            camera: Camera::default(),
            exit: bool::default(),
            stl_shape: Shape3D::from_stl(stl, Affine3::from_axis_angle(Vec3::X, -PI / 2.0)),
            render_depth: 10.0,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
            self.t += 1.0 / 60.0;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> std::io::Result<()> {
        if event::poll(Duration::from_secs_f64(1.0 / 60.0))? {
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

        World::new(Box::new(self.camera), self.render_depth)
            .block(block)
            .add_shape(self.stl_shape.clone())
            .render(area, buf);
    }
}
