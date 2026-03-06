use crate::{
    state::{App, Panel},
    tui::key_binds::get_workspaces_keybinds,
};

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
    Frame,
};

pub fn panel(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.panel == Panel::Workspace;
    let border_color = if focused {
        Color::Blue
    } else {
        Color::DarkGray
    };

    let title = if focused {
        let mut keys = Vec::new();
        keys.push(Span::styled(" Wkspc ", Style::default().fg(Color::Blue)));
        get_workspaces_keybinds(&mut keys, app.compositor);
        Line::from(keys)
    } else {
        Line::from(Span::styled(
            " Workspaces ",
            Style::default().fg(Color::DarkGray),
        ))
    };

    let has_pending = app.has_pending_workspaces();
    let pending_color = if has_pending {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let supports_defaults = app.compositor.supports_workspace_defaults();
    let monitors = app.monitors.clone();
    let pending_keys: Vec<usize> = app.pending_workspaces.keys().copied().collect();

    let items: Vec<ListItem> = app
        .workspace_assignments
        .iter()
        .enumerate()
        .map(|(idx, _ws)| {
            let effective = app
                .get_effective_workspace(idx)
                .unwrap_or_else(|| _ws.clone());
            let monitor_name = effective
                .monitor_idx
                .and_then(|i| monitors.get(i))
                .map(|m| m.name.as_str())
                .unwrap_or("unassigned");

            let is_assigned = effective.monitor_idx.is_some();
            let is_pending = pending_keys.contains(&idx);
            let name_style = if is_pending {
                Style::default().fg(Color::Yellow)
            } else if is_assigned {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let mut spans = vec![
                Span::styled(
                    format!("  WS {} ", effective.id),
                    Style::default().fg(Color::White),
                ),
                Span::styled("\u{2192} ", Style::default().fg(pending_color)),
                Span::styled(monitor_name, name_style),
            ];

            if effective.is_default && supports_defaults {
                spans.push(Span::styled(" [D]", Style::default().fg(Color::Green)));
            }
            if effective.is_persistent && supports_defaults {
                spans.push(Span::styled(" [P]", Style::default().fg(Color::Yellow)));
            }

            if is_pending {
                spans.push(Span::styled(" *", Style::default().fg(Color::Yellow)));
            }

            Line::from(spans).into()
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let list = List::new(items)
        .block(block)
        .highlight_symbol(" \u{203a} ")
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut app.workspace_state);
}
