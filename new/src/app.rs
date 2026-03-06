use std::io;

use ratatui::{DefaultTerminal, Frame, layout::Layout};

use crate::panes::{
    Pane, modes::Modes, monitor_map::MonitorMap, scale::Scale, transform::Transform,
    workspace::Workspace,
};

#[derive(Debug)]
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
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while self.exit {
            terminal.draw(|frame| self.draw(frame))?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {}
}
