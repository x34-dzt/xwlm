use std::sync::mpsc::Receiver;
use std::time::Duration;

use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, Paragraph,
};
use wlx_monitors::{WlMonitorEvent, WlTransform};

use crate::app::{
    App, Panel, TRANSFORMS, effective_dimensions, monitor_resolution,
    transform_label,
};
use xwlm_cfg::Compositor;

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
            if app.pending_toggle_warning {
                match k.code {
                    KeyCode::Char('y') => {
                        app.toggle_monitor();
                    }
                    _ => app.dismiss_warning(),
                }
            } else {
                match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Left | KeyCode::Char('h') => app.nav_left(),
                    KeyCode::Right | KeyCode::Char('l') => app.nav_right(),
                    KeyCode::Tab => app.toggle_panel(),
                    KeyCode::Char('t') => app.toggle_monitor(),
                    KeyCode::Char('r') => app.reset_positions(),
                    KeyCode::Char(']') => app.select_next_monitor(),
                    KeyCode::Char('[') => app.select_prev_monitor(),
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        if app.panel == Panel::Map {
                            app.zoom_in();
                        } else {
                            app.scale_up();
                        }
                    }
                    KeyCode::Char('-') => {
                        if app.panel == Panel::Map {
                            app.zoom_out();
                        } else {
                            app.scale_down();
                        }
                    }
                    KeyCode::Enter => app.apply_action(),
                    _ => {}
                }
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

    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(main_layout[0]);

    render_left_panel(frame, app, content[0]);
    render_modes(frame, app, content[1]);
    render_workspaces(frame, app, content[2]);
    render_keybindings(frame, main_layout[1], app.compositor);

    if app.pending_toggle_warning {
        render_warning_modal(frame, area, &app.monitor_config_path);
    }
}

fn render_warning_modal(frame: &mut Frame, area: Rect, config_path: &str) {
    let path_w = config_path.len() as u16 + 14;
    let modal_w = path_w.max(48).min(area.width.saturating_sub(4));
    let modal_h = 15u16.min(area.height.saturating_sub(2));
    let x = (area.width.saturating_sub(modal_w)) / 2;
    let y = (area.height.saturating_sub(modal_h)) / 2;
    let modal_area = Rect::new(x, y, modal_w, modal_h);

    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red))
        .title(" Warning ");

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(inner);

    let text = vec![
        Line::from(vec![Span::styled(
            " ⚠ Disable your last monitor?",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            " No way to undo from here.",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            " To recover, you'll need to:",
            Style::default().fg(Color::White),
        )]),
        Line::from(vec![Span::styled(
            " 1. Reboot your machine",
            Style::default().fg(Color::DarkGray),
        )]),
        Line::from(vec![Span::styled(
            " 2. Open a TTY session",
            Style::default().fg(Color::DarkGray),
        )]),
        Line::from(vec![
            Span::styled(" 3. Edit ", Style::default().fg(Color::DarkGray)),
            Span::styled(config_path, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![Span::styled(
            "    and remove the disable line",
            Style::default().fg(Color::DarkGray),
        )]),
        Line::from(vec![Span::styled(
            " 4. Reboot and log into your compositor",
            Style::default().fg(Color::DarkGray),
        )]),
    ];

    let buttons = vec![
        Line::from(vec![
            Span::styled(" ┌───────┐ ", Style::default().fg(Color::Red)),
            Span::styled("┌──────┐", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled(" │ ", Style::default().fg(Color::Red)),
            Span::styled(
                "[Y]",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled("es ", Style::default().fg(Color::Red)),
            Span::styled("│ ", Style::default().fg(Color::Red)),
            Span::styled("│ ", Style::default().fg(Color::Green)),
            Span::styled(
                "[N]",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("o ", Style::default().fg(Color::Green)),
            Span::styled("│", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled(" └───────┘ ", Style::default().fg(Color::Red)),
            Span::styled("└──────┘", Style::default().fg(Color::Green)),
        ]),
    ];

    frame.render_widget(Paragraph::new(text), layout[0]);
    frame.render_widget(Paragraph::new(buttons), layout[1]);
}

fn render_left_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(10)])
        .split(area);

    render_map(frame, app, left[0]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(left[1]);

    render_scale(frame, app, bottom[0]);
    render_transform(frame, app, bottom[1]);
}

fn render_modes(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.panel == Panel::Modes;
    let border_color = if focused {
        Color::Blue
    } else {
        Color::DarkGray
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
                                mode.resolution.width,
                                mode.resolution.height,
                                mode.refresh_rate,
                            ),
                            style,
                        ),
                        Span::styled(
                            preferred,
                            Style::default().fg(Color::Yellow),
                        ),
                    ])
                    .into()
                })
                .collect()
        })
        .unwrap_or_default();

    let title = if focused {
        Line::from(vec![
            Span::styled(" Modes ", Style::default().fg(Color::Blue)),
            Span::styled(
                "\u{2191}\u{2193} select ",
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else {
        Line::from(Span::styled(
            " Modes ",
            Style::default().fg(Color::DarkGray),
        ))
    };

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

fn render_workspaces(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.panel == Panel::Workspaces;
    let border_color = if focused {
        Color::Blue
    } else {
        Color::DarkGray
    };

    let items: Vec<ListItem> = app
        .workspace_assignments
        .iter()
        .map(|ws| {
            let monitor_name = ws
                .monitor_idx
                .and_then(|idx| app.monitors.get(idx))
                .map(|m| m.name.as_str())
                .unwrap_or("unassigned");

            let is_assigned = ws.monitor_idx.is_some();
            let name_style = if is_assigned {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            Line::from(vec![
                Span::styled(
                    format!("  WS {} ", ws.id),
                    Style::default().fg(Color::White),
                ),
                Span::styled("\u{2192} ", Style::default().fg(Color::DarkGray)),
                Span::styled(monitor_name, name_style),
            ])
            .into()
        })
        .collect();

    let title = if focused {
        Line::from(vec![
            Span::styled(" Workspaces ", Style::default().fg(Color::Blue)),
            Span::styled(
                "\u{2190}\u{2192} assign ",
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else {
        Line::from(Span::styled(
            " Workspaces ",
            Style::default().fg(Color::DarkGray),
        ))
    };

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

fn render_map(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.panel == Panel::Map;
    let border_color = if focused {
        Color::Blue
    } else {
        Color::DarkGray
    };

    let title = if focused {
        Line::from(vec![
            Span::styled(" Monitor Layout ", Style::default().fg(Color::Blue)),
            Span::styled(
                "\u{2190}\u{2191}\u{2192}\u{2193} move  +/- zoom  [] switch  t on/off ",
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else {
        Line::from(Span::styled(
            " Monitor Layout ",
            Style::default().fg(Color::DarkGray),
        ))
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 4 || inner.width < 10 {
        return;
    }

    let grid_height = inner.height.saturating_sub(1) as usize;
    let grid_width = inner.width as usize;

    let mut lines = build_layout_map(app, grid_width, grid_height);

    while lines.len() < grid_height {
        lines.push(Line::from(""));
    }

    if let Some(monitor) = app.selected_monitor() {
        let (ew, eh) = effective_dimensions(monitor);
        if monitor.enabled {
            let (dx, dy) = app.display_position(app.selected_monitor);
            let has_pending = app.has_pending_positions();
            let pos_color = if has_pending {
                Color::Yellow
            } else {
                Color::DarkGray
            };
            let mut spans = vec![
                Span::styled("  ○ ", Style::default().fg(Color::Green)),
                Span::styled(
                    format!("{}  ", monitor.name),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{}×{}  ", ew, eh),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("({},{})  ", dx, dy),
                    Style::default().fg(pos_color),
                ),
                Span::styled(
                    format!("{}×  ", monitor.scale),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    "ON",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ];
            if has_pending {
                spans.push(Span::styled(
                    "  Enter to apply",
                    Style::default().fg(Color::Yellow),
                ));
            }
            lines.push(Line::from(spans));
        } else {
            lines.push(Line::from(vec![
                Span::styled("  ○ ", Style::default().fg(Color::Red)),
                Span::styled(
                    format!("{}  ", monitor.name),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{}×{}  ", ew, eh),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    "OFF ",
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "— t to enable",
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    } else {
        lines.push(Line::from("  No monitor selected"));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn build_layout_map<'a>(
    app: &App,
    width: usize,
    height: usize,
) -> Vec<Line<'a>> {
    let monitors = &app.monitors;
    let selected_idx = app.selected_monitor;
    let zoom = app.map_zoom;

    if monitors.is_empty() {
        return vec![Line::from("  No monitors")];
    }
    if width < 5 || height < 3 {
        return vec![Line::from("  Panel too small")];
    }

    struct MonRect {
        name: String,
        px: i32,
        py: i32,
        pw: i32,
        ph: i32,
        is_selected: bool,
        is_enabled: bool,
        res_label: String,
        pos_label: String,
    }

    let mut monitor_rects: Vec<MonRect> = Vec::new();
    for (idx, m) in monitors.iter().enumerate() {
        if !m.enabled {
            continue;
        }
        let (w, h) = effective_dimensions(m);
        let (rw, rh) = monitor_resolution(m);
        let (px, py) = app.display_position(idx);
        monitor_rects.push(MonRect {
            name: m.name.clone(),
            px,
            py,
            pw: w.max(1),
            ph: h.max(1),
            is_selected: idx == selected_idx,
            is_enabled: true,
            res_label: format!("{}×{}", rw, rh),
            pos_label: format!("({},{})", px, py),
        });
    }

    let bottom_y = if monitor_rects.is_empty() {
        0
    } else {
        monitor_rects.iter().map(|r| r.py + r.ph).max().unwrap_or(0)
    };
    let disabled_y = bottom_y + 200;
    let mut disabled_x = monitor_rects.iter().map(|r| r.px).min().unwrap_or(0);

    for (idx, m) in monitors.iter().enumerate() {
        if m.enabled {
            continue;
        }
        let (w, h) = effective_dimensions(m);
        let (rw, rh) = monitor_resolution(m);
        let pw = w.max(1);
        let ph = h.max(1);
        monitor_rects.push(MonRect {
            name: m.name.clone(),
            px: disabled_x,
            py: disabled_y,
            pw,
            ph,
            is_selected: idx == selected_idx,
            is_enabled: false,
            res_label: format!("{}×{}", rw, rh),
            pos_label: "OFF".to_string(),
        });
        disabled_x += pw + 100;
    }

    let min_x = monitor_rects.iter().map(|r| r.px).min().unwrap_or(0);
    let min_y = monitor_rects.iter().map(|r| r.py).min().unwrap_or(0);
    let max_x = monitor_rects.iter().map(|r| r.px + r.pw).max().unwrap_or(0);
    let max_y = monitor_rects.iter().map(|r| r.py + r.ph).max().unwrap_or(0);

    let total_w = (max_x - min_x) as f64;
    let total_h = (max_y - min_y) as f64;

    if total_w <= 0.0 || total_h <= 0.0 {
        return vec![];
    }

    const CHAR_ASPECT: f64 = 2.0;

    let pad = 2_usize;
    let avail_w = width.saturating_sub(pad * 2) as f64;
    let avail_h = height.saturating_sub(1) as f64;

    let ppc_x = total_w / (avail_w * 0.8);
    let ppc_y = total_h / (avail_h * CHAR_ASPECT * 0.8);
    let ppc = ppc_x.max(ppc_y) / zoom;

    if ppc <= 0.0 {
        return vec![];
    }

    let mut grid: Vec<Vec<(char, Color, bool)>> =
        vec![vec![(' ', Color::Reset, false); width]; height];

    for rect in &monitor_rects {
        let cx = pad + ((rect.px - min_x) as f64 / ppc) as usize;
        let cy = ((rect.py - min_y) as f64 / (ppc * CHAR_ASPECT)) as usize;
        let cw = (rect.pw as f64 / ppc).round().max(1.0) as usize;
        let ch =
            (rect.ph as f64 / (ppc * CHAR_ASPECT)).round().max(1.0) as usize;

        let x1 = cx.min(width.saturating_sub(1));
        let y1 = cy.min(height.saturating_sub(1));
        let x2 = (cx + cw).min(width);
        let y2 = (cy + ch).min(height);
        let w = x2.saturating_sub(x1);
        let h = y2.saturating_sub(y1);

        if w < 2 || h < 2 {
            if y1 < height && x1 < width {
                let ch = rect.name.chars().next().unwrap_or('?');
                let fg = if rect.is_selected {
                    Color::Cyan
                } else if rect.is_enabled {
                    Color::White
                } else {
                    Color::DarkGray
                };
                grid[y1][x1] = (ch, fg, rect.is_selected);
            }
            continue;
        }

        let border_fg = if rect.is_selected && rect.is_enabled {
            Color::Cyan
        } else if rect.is_selected {
            Color::Yellow
        } else if rect.is_enabled {
            Color::DarkGray
        } else {
            Color::Rgb(60, 60, 60)
        };
        let text_fg = if rect.is_selected && rect.is_enabled {
            Color::White
        } else if rect.is_selected {
            Color::Yellow
        } else if rect.is_enabled {
            Color::Gray
        } else {
            Color::Rgb(80, 80, 80)
        };

        let (tl, tr, bl, br, hc, vc) = if rect.is_selected {
            ('╔', '╗', '╚', '╝', '═', '║')
        } else if rect.is_enabled {
            ('┌', '┐', '└', '┘', '─', '│')
        } else {
            ('┌', '┐', '└', '┘', '╌', '╎')
        };

        grid[y1][x1] = (tl, border_fg, false);
        grid[y1][x2 - 1] = (tr, border_fg, false);
        grid[y2 - 1][x1] = (bl, border_fg, false);
        grid[y2 - 1][x2 - 1] = (br, border_fg, false);

        for cell in grid[y1][(x1 + 1)..(x2 - 1)].iter_mut() {
            *cell = (hc, border_fg, false);
        }
        for cell in grid[y2 - 1][(x1 + 1)..(x2 - 1)].iter_mut() {
            *cell = (hc, border_fg, false);
        }

        for row in grid[(y1 + 1)..(y2 - 1)].iter_mut() {
            row[x1] = (vc, border_fg, false);
            row[x2 - 1] = (vc, border_fg, false);
        }

        for row in grid[(y1 + 1)..(y2 - 1)].iter_mut() {
            for cell in row[(x1 + 1)..(x2 - 1)].iter_mut() {
                *cell = (' ', text_fg, false);
            }
        }

        let inner_w = w.saturating_sub(2);
        let inner_h = h.saturating_sub(2);

        if inner_w >= 1 && inner_h >= 1 {
            let text_lines: Vec<(&str, bool)> = vec![
                (&rect.name, true),
                (&rect.res_label, false),
                (&rect.pos_label, false),
            ];
            let count = text_lines.len().min(inner_h);
            let start_y = y1 + 1 + inner_h.saturating_sub(count) / 2;

            for (i, (text, bold)) in text_lines.iter().take(count).enumerate() {
                let row = start_y + i;
                if row >= y2 - 1 {
                    break;
                }
                let truncated: String = text.chars().take(inner_w).collect();
                let text_start =
                    x1 + 1 + inner_w.saturating_sub(truncated.len()) / 2;
                for (j, ch) in truncated.chars().enumerate() {
                    let col = text_start + j;
                    if col < x2 - 1 {
                        grid[row][col] =
                            (ch, text_fg, *bold || rect.is_selected);
                    }
                }
            }
        }
    }

    let mut lines = Vec::new();
    for row in &grid {
        let mut spans = Vec::new();
        let mut i = 0;
        while i < width {
            let (ch, color, bold) = row[i];
            let mut run = String::new();
            run.push(ch);
            let mut j = i + 1;
            while j < width && row[j].1 == color && row[j].2 == bold {
                run.push(row[j].0);
                j += 1;
            }
            let mut style = Style::default().fg(color);
            if bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            spans.push(Span::styled(run, style));
            i = j;
        }
        lines.push(Line::from(spans));
    }

    lines
}

fn render_scale(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.panel == Panel::Scale;
    let border_color = if focused {
        Color::Blue
    } else {
        Color::DarkGray
    };

    let monitor = app.selected_monitor();
    let current = monitor.map(|m| m.scale).unwrap_or(1.0);
    let pending = app.pending_scale;
    let changed = (current - pending).abs() > 0.001;

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
            Span::styled(
                format!("  {}", filled_part),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled("●", Style::default().fg(Color::White)),
            Span::styled(empty_part, Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        if changed {
            Line::from(vec![Span::styled(
                "  Enter to apply",
                Style::default().fg(Color::Yellow),
            )])
        } else {
            Line::from(vec![Span::styled(
                "  ↑↓ or +/- adjust",
                Style::default().fg(Color::DarkGray),
            )])
        },
    ];

    let title = if focused {
        Line::from(vec![
            Span::styled(" Scale ", Style::default().fg(Color::Blue)),
            Span::styled(
                "\u{2190}\u{2192} adjust ",
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else {
        Line::from(Span::styled(
            " Scale ",
            Style::default().fg(Color::DarkGray),
        ))
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(title);

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_transform(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.panel == Panel::Transform;
    let border_color = if focused {
        Color::Blue
    } else {
        Color::DarkGray
    };

    let current_transform = app
        .selected_monitor()
        .map(|m| m.transform)
        .unwrap_or(WlTransform::Normal);

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

    let title = if focused {
        Line::from(vec![
            Span::styled(" Transform ", Style::default().fg(Color::Blue)),
            Span::styled(
                "\u{2191}\u{2193} select ",
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else {
        Line::from(Span::styled(
            " Transform ",
            Style::default().fg(Color::DarkGray),
        ))
    };

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

    frame.render_stateful_widget(list, area, &mut app.transform_state);
}

fn render_keybindings(frame: &mut Frame, area: Rect, compositor: Compositor) {
    let keys = Line::from(vec![
        Span::styled(
            " xwlm ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
        Span::styled("Tab ", Style::default().fg(Color::Cyan)),
        Span::styled("panel  ", Style::default().fg(Color::DarkGray)),
        Span::styled("↑↓ ", Style::default().fg(Color::Cyan)),
        Span::styled("navigate  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter ", Style::default().fg(Color::Cyan)),
        Span::styled("apply  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[] ", Style::default().fg(Color::Cyan)),
        Span::styled("monitor  ", Style::default().fg(Color::DarkGray)),
        Span::styled("+/- ", Style::default().fg(Color::Cyan)),
        Span::styled("scale/zoom  ", Style::default().fg(Color::DarkGray)),
        Span::styled("t ", Style::default().fg(Color::Cyan)),
        Span::styled("toggle  ", Style::default().fg(Color::DarkGray)),
        Span::styled("r ", Style::default().fg(Color::Cyan)),
        Span::styled("reset  ", Style::default().fg(Color::DarkGray)),
        Span::styled("q ", Style::default().fg(Color::Cyan)),
        Span::styled("quit  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("[{}]", compositor.label()),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(keys), area);
}
