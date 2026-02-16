use std::sync::mpsc::Receiver;
use std::time::Duration;

use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};
use wlx_monitors::WlMonitorEvent;

use crate::app::{App, Panel, TRANSFORMS, transform_label};
use crate::compositor::Compositor;

pub fn run(
    mut terminal: DefaultTerminal,
    app: &mut App,
    event_rx: Receiver<WlMonitorEvent>,
) -> Result<()> {
    loop {
        let mut had_events = false;
        while let Ok(event) = event_rx.try_recv() {
            had_events = true;
            match event {
                WlMonitorEvent::InitialState(ref monitors) => {
                    app.set_monitors(monitors.clone());
                }
                WlMonitorEvent::Changed(monitor) => {
                    app.update_monitor(*monitor);
                }
                WlMonitorEvent::Removed { ref name, .. } => {
                    app.remove_monitor(name);
                }
                WlMonitorEvent::ActionFailed { action, reason } => {
                    app.needs_save = false;
                    eprintln!("Action failed ({:?}): {}", action, reason);
                }
            }
        }
        if had_events {
            app.save_config();
        }

        terminal.draw(|f| render(f, app))?;

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(k) = event::read()?
        {
            match k.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Up | KeyCode::Char('k') => app.previous(),
                KeyCode::Down | KeyCode::Char('j') => app.next(),
                KeyCode::Tab => app.toggle_panel(),
                KeyCode::Char('t') => app.toggle_monitor(),
                KeyCode::Char('+') | KeyCode::Char('=')
                | KeyCode::Right => app.scale_up(),
                KeyCode::Char('-') | KeyCode::Left => app.scale_down(),
                KeyCode::Enter => app.apply_action(),
                _ => {}
            }
        }
    }
    Ok(())
}

fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(main_layout[0]);

    render_monitor_list(frame, app, panels[0]);
    render_right_panel(frame, app, panels[1]);
    render_keybindings(frame, main_layout[1], app.compositor);
}

fn render_monitor_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.panel == Panel::Monitors;
    let border_color = if focused {
        Color::Blue
    } else {
        Color::DarkGray
    };

    let items: Vec<ListItem> = app
        .monitors
        .iter()
        .map(|m| {
            let indicator = if m.enabled { "●" } else { "○" };
            let color = if m.enabled {
                Color::Green
            } else {
                Color::DarkGray
            };
            Line::from(vec![
                Span::styled(
                    format!(" {} ", indicator),
                    Style::default().fg(color),
                ),
                Span::styled(&m.name, Style::default().fg(Color::White)),
            ])
            .into()
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(" Monitors ");

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );

    frame.render_stateful_widget(list, area, &mut app.monitor_state);
}

fn render_right_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    let Some(monitor) = app.selected_monitor() else {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" No monitor selected ");
        frame.render_widget(block, area);
        return;
    };
    let monitor = monitor.clone();

    let right_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(10),
        ])
        .split(area);

    // Info bar
    render_info_bar(frame, &monitor, right_layout[0]);

    // Modes panel (always in the middle area)
    render_modes(frame, app, &monitor, right_layout[1]);

    // Bottom: Scale + Transform side by side
    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(right_layout[2]);

    render_scale(frame, app, &monitor, bottom[0]);
    render_transform(frame, app, &monitor, bottom[1]);
}

fn render_info_bar(
    frame: &mut Frame,
    monitor: &wlx_monitors::WlMonitor,
    area: Rect,
) {
    let status = if monitor.enabled {
        Span::styled("enabled", Style::default().fg(Color::Green))
    } else {
        Span::styled("disabled", Style::default().fg(Color::Red))
    };

    let info = Line::from(vec![
        Span::styled(
            format!(" {} ", monitor.name),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" · ", Style::default().fg(Color::DarkGray)),
        Span::styled(&monitor.description, Style::default().fg(Color::DarkGray)),
        Span::styled(" · ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(
                "{}x{} ",
                monitor.resolution.width, monitor.resolution.height
            ),
            Style::default().fg(Color::White),
        ),
        Span::styled(" · ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("({}, {}) ", monitor.position.x, monitor.position.y),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(" · ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}x ", monitor.scale),
            Style::default().fg(Color::White),
        ),
        Span::styled(" · ", Style::default().fg(Color::DarkGray)),
        status,
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    frame.render_widget(Paragraph::new(info).block(block), area);
}

fn render_modes(
    frame: &mut Frame,
    app: &mut App,
    monitor: &wlx_monitors::WlMonitor,
    area: Rect,
) {
    let focused = app.panel == Panel::Modes;
    let border_color = if focused {
        Color::Blue
    } else {
        Color::DarkGray
    };

    let items: Vec<ListItem> = monitor
        .modes
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
                Span::styled(format!("  {}", marker), style),
                Span::styled(
                    format!(
                        "{}x{} @ {}Hz",
                        mode.resolution.width,
                        mode.resolution.height,
                        mode.refresh_rate
                    ),
                    style,
                ),
                Span::styled(preferred, Style::default().fg(Color::Yellow)),
            ])
            .into()
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(" Modes ");

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );

    frame.render_stateful_widget(list, area, &mut app.mode_state);
}

fn render_scale(
    frame: &mut Frame,
    app: &App,
    monitor: &wlx_monitors::WlMonitor,
    area: Rect,
) {
    let focused = app.panel == Panel::Scale;
    let border_color = if focused {
        Color::Blue
    } else {
        Color::DarkGray
    };

    let current = monitor.scale;
    let pending = app.pending_scale;
    let changed = (current - pending).abs() > 0.001;

    // Slider bar
    let bar_width = (area.width as usize).saturating_sub(6);
    let max_scale = 10.0_f64;
    let fill = ((pending / max_scale) * bar_width as f64)
        .round()
        .min(bar_width as f64) as usize;
    let empty = bar_width.saturating_sub(fill);
    let filled_part = "━".repeat(fill.saturating_sub(1));
    let empty_part = "─".repeat(empty);

    let pending_color = if changed { Color::Yellow } else { Color::White };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  current ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.2}x", current),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  pending ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.2}x", pending),
                Style::default().fg(pending_color),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {}", filled_part), Style::default().fg(Color::Cyan)),
            Span::styled("●", Style::default().fg(Color::White)),
            Span::styled(empty_part, Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        if changed {
            Line::from(vec![
                Span::styled(
                    "  Enter to apply",
                    Style::default().fg(Color::Yellow),
                ),
            ])
        } else {
            Line::from(vec![
                Span::styled(
                    "  ↑↓ or +/- adjust",
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        },
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(" Scale ");

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_transform(
    frame: &mut Frame,
    app: &mut App,
    monitor: &wlx_monitors::WlMonitor,
    area: Rect,
) {
    let focused = app.panel == Panel::Transform;
    let border_color = if focused {
        Color::Blue
    } else {
        Color::DarkGray
    };

    let current_transform = monitor.transform;

    let items: Vec<ListItem> = TRANSFORMS
        .iter()
        .map(|&t| {
            let is_current = t == current_transform;
            let marker = if is_current { " ✓" } else { "" };
            let style = if is_current {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };

            Line::from(vec![
                Span::styled(format!("  {}", transform_label(t)), style),
                Span::styled(marker, Style::default().fg(Color::Green)),
            ])
            .into()
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(" Transform ");

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );

    frame.render_stateful_widget(list, area, &mut app.transform_state);
}

fn render_keybindings(frame: &mut Frame, area: Rect, compositor: Compositor) {
    let keys = Line::from(vec![
        Span::styled(" ↑↓ ", Style::default().fg(Color::Cyan)),
        Span::styled("navigate  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Tab ", Style::default().fg(Color::Cyan)),
        Span::styled("panel  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter ", Style::default().fg(Color::Cyan)),
        Span::styled("apply  ", Style::default().fg(Color::DarkGray)),
        Span::styled("+/- ", Style::default().fg(Color::Cyan)),
        Span::styled("scale  ", Style::default().fg(Color::DarkGray)),
        Span::styled("t ", Style::default().fg(Color::Cyan)),
        Span::styled("toggle  ", Style::default().fg(Color::DarkGray)),
        Span::styled("q ", Style::default().fg(Color::Cyan)),
        Span::styled("quit  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("[{}]", compositor.label()),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(keys), area);
}
