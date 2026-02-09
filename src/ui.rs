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

use crate::app::{App, Panel};

pub fn run(
    mut terminal: DefaultTerminal,
    app: &mut App,
    event_rx: Receiver<WlMonitorEvent>,
) -> Result<()> {
    loop {
        while let Ok(event) = event_rx.try_recv() {
            match event {
                WlMonitorEvent::InitialState(ref monitors) => {
                    app.set_monitors(monitors.clone());
                }
                WlMonitorEvent::Changed(ref monitor) => {
                    app.update_monitor(monitor.clone());
                }
                WlMonitorEvent::Removed { ref name, .. } => {
                    app.remove_monitor(name);
                }
            }
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
                KeyCode::Enter => app.apply_mode(),
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
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(main_layout[0]);

    render_monitor_list(frame, app, panels[0]);
    render_details(frame, app, panels[1]);
    render_keybindings(frame, main_layout[1]);
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
            let res = format!("{}x{}", m.resolution.width, m.resolution.height);
            Line::from(vec![
                Span::styled(
                    format!("  {} ", indicator),
                    Style::default().fg(color),
                ),
                Span::styled(m.name.clone(), Style::default().fg(Color::White)),
                Span::styled(
                    format!("  {}", res),
                    Style::default().fg(Color::DarkGray),
                ),
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

fn render_details(frame: &mut Frame, app: &mut App, area: Rect) {
    let Some(monitor) = app.selected_monitor() else {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Details ");
        frame.render_widget(block, area);
        return;
    };

    let monitor = monitor.clone();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(format!(" {} ", monitor.name));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let detail_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(3)])
        .split(inner);

    let status = if monitor.enabled {
        "enabled"
    } else {
        "disabled"
    };
    let status_color = if monitor.enabled {
        Color::Green
    } else {
        Color::Red
    };

    let details = vec![
        Line::from(vec![
            Span::styled(
                "  Description  ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                &monitor.description,
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Make         ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(&monitor.make, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(
                "  Model        ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(&monitor.model, Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Resolution   ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!(
                    "{} x {}",
                    monitor.resolution.width, monitor.resolution.height
                ),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Position     ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("({}, {})", monitor.position.x, monitor.position.y),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Scale        ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{}", monitor.scale),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Status       ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(status, Style::default().fg(status_color)),
        ]),
    ];

    frame.render_widget(Paragraph::new(details), detail_layout[0]);

    render_modes(frame, app, &monitor, detail_layout[1]);
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
            let is_current = mode.resolution.width == monitor.resolution.width
                && mode.resolution.height == monitor.resolution.height;
            let marker = if is_current { "▸ " } else { "  " };
            let preferred = if mode.preferred { " ★" } else { "" };

            let style = if is_current {
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

fn render_keybindings(frame: &mut Frame, area: Rect) {
    let keys = Line::from(vec![
        Span::styled(" ↑↓ ", Style::default().fg(Color::Cyan)),
        Span::styled("navigate  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Tab ", Style::default().fg(Color::Cyan)),
        Span::styled("switch panel  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter ", Style::default().fg(Color::Cyan)),
        Span::styled("apply mode  ", Style::default().fg(Color::DarkGray)),
        Span::styled("t ", Style::default().fg(Color::Cyan)),
        Span::styled("toggle  ", Style::default().fg(Color::DarkGray)),
        Span::styled("q ", Style::default().fg(Color::Cyan)),
        Span::styled("quit", Style::default().fg(Color::DarkGray)),
    ]);
    frame.render_widget(Paragraph::new(keys), area);
}
