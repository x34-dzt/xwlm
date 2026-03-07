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
use wlx_monitors::WlMonitorMode;

#[derive(Debug, Default)]
pub struct Modes {
    modes: Vec<WlMonitorMode>,
    state: ListState,
    active: bool,
}

// impl Default for Modes {
//     fn default() -> Self {
//         Self {
//             modes: Vec::new(),
//             state: ListState::default(),
//             active: false,
//         }
//     }
// }

impl Modes {
    pub fn draw(&mut self, frame: &mut Frame, area: Rect, is_active: bool) {
        self.active = is_active;
        frame.render_widget(self, area);
    }

    pub fn set_modes(&mut self, modes: Vec<WlMonitorMode>) {
        self.modes = modes;
        let mut state = ListState::default();
        state.select(Some(0));
        self.state = state;
    }

    pub fn binds(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Down => {
                let i = match self.state.selected() {
                    Some(i) => (i + 1).min(self.modes.len() - 1),
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

    pub fn sync(&mut self, modes: Vec<WlMonitorMode>) {
        self.modes = modes;
        let mut state = ListState::default();
        state.select(Some(0));
        self.state = state;
    }
}

impl Widget for &mut Modes {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from("Modes".bold());
        let border_style = if self.active {
            Style::new().green()
        } else {
            Style::new().dim()
        };

        let block = Block::bordered()
            .title(title)
            .border_set(border::THICK)
            .style(border_style);

        let list_items: Vec<ListItem> = self
            .modes
            .iter()
            .map(|m| {
                ListItem::new(format!(
                    "{}x{}@{}",
                    m.resolution.height, m.resolution.width, m.refresh_rate
                ))
                .style(Style::new().white())
            })
            .collect();

        let list = List::new(list_items)
            .style(Style::new().white())
            .block(block)
            .highlight_symbol(" > ");

        StatefulWidget::render(list, area, buf, &mut self.state);
    }
}
