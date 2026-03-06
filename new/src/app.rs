use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::panes::{
    Pane, modes::Modes, monitor_map::MonitorMap, scale::Scale, transform::Transform,
    workspace::Workspace,
};

#[derive(Debug, Default)]
pub struct App {
    active_pane: Pane,
    monitor_map: MonitorMap,
    scale: Scale,
    transform: Transform,
    workspace: Workspace,
    modes: Modes,
    exit: bool,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let root = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(frame.area());

        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(root[0]);

        let bottom_left = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(left[1]);

        let right = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(root[1]);

        self.monitor_map
            .draw(frame, left[0], self.active_pane == Pane::Map);
        self.scale
            .draw(frame, bottom_left[0], self.active_pane == Pane::Scale);
        self.transform
            .draw(frame, bottom_left[1], self.active_pane == Pane::Transform);
        self.modes
            .draw(frame, right[0], self.active_pane == Pane::Mode);
        self.workspace
            .draw(frame, right[1], self.active_pane == Pane::Worksapce);
    }

    pub fn handle_key_events(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Char('q') => self.exit = true,
            KeyCode::Tab => self.cycle_pane(),
            _ => {}
        }
    }

    pub fn cycle_pane(&mut self) {
        self.active_pane = self.active_pane.next();
    }

    pub fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_events(key_event)
            }
            _ => {}
        };

        Ok(())
    }
}
