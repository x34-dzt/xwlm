use ratatui::widgets::ListState;

use super::monitor;

pub struct App {
    pub monitors: Vec<monitor::Monitor>,
    pub list_state: ListState,
}

impl App {
    pub fn new() -> App {
        App {
            monitors: Vec::new(),
            list_state: ListState::default(),
        }
    }

    pub fn set_monitor(&mut self, monitor: Vec<monitor::Monitor>) {
        self.monitors = monitor;
    }

    pub fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) if i + 1 < self.monitors.len() => i + 1,
            _ => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) if i > 0 => i - 1,
            _ => self.monitors.len().saturating_sub(1),
        };
        self.list_state.select(Some(i));
    }
}
