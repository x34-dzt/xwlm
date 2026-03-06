use crate::{
    state::{App, Panel},
    tui::key_binds::get_modes_keybinds,
};

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
};

pub fn panel(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.panel == Panel::Mode;
    let border_color = if focused {
        Color::Blue
    } else {
        Color::DarkGray
    };

    let title = if focused {
        let mut keys = Vec::new();
        keys.push(Span::styled(" Modes ", Style::default().fg(Color::Blue)));
        get_modes_keybinds(&mut keys);
        Line::from(keys)
    } else {
        Line::from(Span::styled(
            " Modes ",
            Style::default().fg(Color::DarkGray),
        ))
    };

    let monitor = app.selected_monitor().cloned();
    let items: Vec<ListItem> = monitor
        .as_ref()
        .map(|m| {
            m.modes
                .iter()
                .map(|mode| {
                    let marker = if mode.is_current { "▸ " } else { "  " };
                    let preferred = if mode.preferred { " ★" } else { "" };
                    let style = if mode.is_current {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    Line::from(vec![
                        Span::styled(marker, style),
                        Span::styled(
                            format!(
                                "{}x{}@{}",
                                mode.resolution.width, mode.resolution.height, mode.refresh_rate,
                            ),
                            style,
                        ),
                        Span::styled(preferred, Style::default().fg(Color::Yellow)),
                    ])
                    .into()
                })
                .collect()
        })
        .unwrap_or_default();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let list = List::new(items)
        .block(block)
        .highlight_symbol(" › ")
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut app.mode_state);
}
