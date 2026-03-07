use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, List, ListItem, ListState, StatefulWidget, Widget},
};
use wlx_monitors::WlMonitor;

#[derive(Debug, Default)]
pub struct MonitorMap {
    monitors: Vec<WlMonitor>,
    state: ListState,
    active: bool,
}

impl MonitorMap {
    pub fn draw(&mut self, frame: &mut Frame, area: Rect, is_active: bool) {
        self.active = is_active;
        frame.render_widget(self, area);
    }

    pub fn set_montiors(&mut self, monitors: Vec<WlMonitor>) {
        self.monitors = monitors;
        let mut state = ListState::default();
        state.select(Some(0));
        self.state = state;
    }

    pub fn binds(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Down => {
                let i = match self.state.selected() {
                    Some(i) => (i + 1).min(self.monitors.len() - 1),
                    None => 0,
                };
                self.state.select(Some(i));
            }
            KeyCode::Up => {
                let i = match self.state.selected() {
                    Some(i) => i.saturating_sub(1),
                    None => 0,
                };
                self.state.select(Some(i));
            }
            _ => {}
        };
    }
}

impl Widget for &mut MonitorMap {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from("Monitors".bold());
        let border_style = if self.active {
            Style::new().green()
        } else {
            Style::new().dim()
        };

        let list_items: Vec<ListItem> = self
            .monitors
            .iter()
            .map(|m| ListItem::new(m.name.as_str()).style(Style::new().white()))
            .collect();

        let block = Block::bordered()
            .title(title)
            .border_set(border::THICK)
            .style(border_style);

        let list = List::new(list_items).block(block).highlight_symbol(" > ");

        StatefulWidget::render(list, area, buf, &mut self.state);
    }
}
