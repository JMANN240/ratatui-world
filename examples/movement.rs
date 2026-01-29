use std::{
    f64::consts::TAU, time::Duration
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
};
use glam::dvec3;
use ratatui::{
    DefaultTerminal, Frame,
    style::{Stylize},
    symbols::border,
    text::Line,
    widgets::{
        Block, Widget,
    },
};
use ratatui_world::{World, camera::Camera, shape::Shape3D};

fn main() -> std::io::Result<()> {
    ratatui::run(|terminal| App::default().run(terminal))
}

#[derive(Debug, Default)]
pub struct App {
    t: f64,
    camera: Camera,
    exit: bool,
}

impl App {
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
            KeyCode::Char('q') => self.exit(),
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
        let title = Line::from(" Movement ".bold());
        let instructions = Line::from(vec![" Quit ".into(), "<Q> ".blue().bold()]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        World::new(self.camera, 10.0)
            .block(block)
            .add_shape(Shape3D::triangular_pyramid(self.t))
            .render(area, buf);
    }
}
