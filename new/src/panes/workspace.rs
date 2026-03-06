use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Widget},
};

#[derive(Debug)]
pub struct Workspace {
    active: bool,
}

impl Default for Workspace {
    fn default() -> Self {
        Self { active: false }
    }
}

impl Workspace {
    pub fn draw(&mut self, frame: &mut Frame, area: Rect, is_active: bool) {
        self.active = is_active;
        frame.render_widget(self, area);
    }
}

impl Widget for &mut Workspace {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from("Workspace".bold());
        let border_style = if self.active {
            Style::new().green()
        } else {
            Style::new().dim()
        };

        Block::bordered()
            .title(title)
            .border_set(border::THICK)
            .style(border_style)
            .render(area, buf);
    }
}
