use crate::{
    compositor::Compositor,
    state::{App, Panel},
};

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn config(frame: &mut Frame, area: Rect, app: &App) {
    let panel = &app.panel;
    let mut keys = vec![
        Span::styled(
            format!("[xwlm]-[{}]", app.compositor.label()),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" | ", Style::default().fg(Color::Cyan)),
        Span::styled("Tab ", Style::default().fg(Color::Cyan)),
        Span::styled("switch panel  ", Style::default().fg(Color::DarkGray)),
        Span::styled("q ", Style::default().fg(Color::Cyan)),
        Span::styled("quit", Style::default().fg(Color::DarkGray)),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
    ];

    match panel {
        Panel::Monitor => {
            keys.push(Span::styled(
                "[ Monitor Layout | ",
                Style::default().fg(Color::Cyan),
            ));
            get_monitor_keybinds(&mut keys);
            keys.push(Span::styled("]", Style::default().fg(Color::Cyan)));
        }
        Panel::Mode => {
            keys.push(Span::styled(
                "[ Modes | ",
                Style::default().fg(Color::Cyan),
            ));
            get_modes_keybinds(&mut keys);
            keys.push(Span::styled("]", Style::default().fg(Color::Cyan)));
        }
        Panel::Scale => {
            keys.push(Span::styled(
                "[ Scale | ",
                Style::default().fg(Color::Cyan),
            ));
            get_scale_keybinds(&mut keys);
            keys.push(Span::styled("]", Style::default().fg(Color::Cyan)));
        }
        Panel::Transform => {
            keys.push(Span::styled(
                "[ Transform | ",
                Style::default().fg(Color::Cyan),
            ));
            get_transform_keybinds(&mut keys);
            keys.push(Span::styled("]", Style::default().fg(Color::Cyan)));
        }
        Panel::Workspace => {
            keys.push(Span::styled(
                "[ Workspaces | ",
                Style::default().fg(Color::Cyan),
            ));
            get_workspaces_keybinds(&mut keys, app.compositor);
            keys.push(Span::styled("]", Style::default().fg(Color::Cyan)));
        }
    };
    let line = Line::from(keys);
    frame.render_widget(Paragraph::new(line), area);
}

pub fn get_monitor_keybinds(keys: &mut Vec<Span<'static>>) {
    keys.push(Span::styled("↑↓ ←→ ", Style::default().fg(Color::Cyan)));
    keys.push(Span::styled("move  ", Style::default().fg(Color::DarkGray)));
    keys.push(Span::styled("+/- ", Style::default().fg(Color::Cyan)));
    keys.push(Span::styled("zoom  ", Style::default().fg(Color::DarkGray)));
    keys.push(Span::styled("[] ", Style::default().fg(Color::Cyan)));
    keys.push(Span::styled(
        "switch monitor ",
        Style::default().fg(Color::DarkGray),
    ));
}

pub fn get_modes_keybinds(keys: &mut Vec<Span<'static>>) {
    keys.push(Span::styled("↑↓ ", Style::default().fg(Color::Cyan)));
    keys.push(Span::styled(
        "select  ",
        Style::default().fg(Color::DarkGray),
    ));
    keys.push(Span::styled("Enter ", Style::default().fg(Color::Cyan)));
    keys.push(Span::styled(
        "apply  ",
        Style::default().fg(Color::DarkGray),
    ));
}

pub fn get_workspaces_keybinds(
    keys: &mut Vec<Span<'static>>,
    compositor: Compositor,
) {
    keys.push(Span::styled("←→ ", Style::default().fg(Color::Cyan)));
    keys.push(Span::styled(
        "assign  ",
        Style::default().fg(Color::DarkGray),
    ));
    if compositor.supports_workspace_defaults() {
        keys.push(Span::styled("d ", Style::default().fg(Color::Cyan)));
        keys.push(Span::styled(
            "default  ",
            Style::default().fg(Color::DarkGray),
        ));
        keys.push(Span::styled("p ", Style::default().fg(Color::Cyan)));
        keys.push(Span::styled(
            "persistent  ",
            Style::default().fg(Color::DarkGray),
        ));
    }
}

pub fn get_scale_keybinds(keys: &mut Vec<Span<'static>>) {
    keys.push(Span::styled("←→ ", Style::default().fg(Color::Cyan)));
    keys.push(Span::styled(
        "adjust ",
        Style::default().fg(Color::DarkGray),
    ));
    keys.push(Span::styled("Enter ", Style::default().fg(Color::Cyan)));
    keys.push(Span::styled(
        "apply  ",
        Style::default().fg(Color::DarkGray),
    ));
}

pub fn get_transform_keybinds(keys: &mut Vec<Span<'static>>) {
    keys.push(Span::styled("↑↓ ", Style::default().fg(Color::Cyan)));
    keys.push(Span::styled(
        "rotate  ",
        Style::default().fg(Color::DarkGray),
    ));
    keys.push(Span::styled("Enter ", Style::default().fg(Color::Cyan)));
    keys.push(Span::styled(
        "apply  ",
        Style::default().fg(Color::DarkGray),
    ));
}
