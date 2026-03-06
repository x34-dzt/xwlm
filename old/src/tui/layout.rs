use crate::{
    state::App,
    tui::{
        key_binds,
        panels::{
            left::{self},
            mode, workspace,
        },
    },
};

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let error_exists =
        app.error_message.is_some() || app.pending_last_toggle_monitor;

    let constraints: [Constraint; 3] = if error_exists {
        [
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ]
    } else {
        [
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(0),
        ]
    };

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(20),
            Constraint::Percentage(30),
        ])
        .split(main_layout[0]);

    left::panel(frame, app, content[0]);
    mode::panel(frame, app, content[1]);
    workspace::panel(frame, app, content[2]);
    key_binds::config(frame, main_layout[1], app);

    if let Some(ref err) = app.error_message {
        let error_bar =
            Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red));
        frame.render_widget(error_bar, main_layout[2]);
    }

    if app.pending_last_toggle_monitor {
        let config_path = app.comp_monitor_config_path.to_string_lossy();
        left::render_warning_modal(frame, area, &config_path);
    }
}
