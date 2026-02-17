use std::time::Duration;

use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::{DefaultTerminal, Frame};

use crate::compositor::Compositor;
use crate::config::AppConfig;

struct SetupState {
    input: String,
    cursor: usize,
    compositor: Compositor,
    error: Option<String>,
}

impl SetupState {
    fn prev_cursor(&self) -> usize {
        self.input[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn next_cursor(&self) -> usize {
        self.input[self.cursor..]
            .char_indices()
            .nth(1)
            .map(|(i, _)| self.cursor + i)
            .unwrap_or(self.input.len())
    }
}

fn default_config_path(compositor: Compositor) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    match compositor {
        Compositor::Hyprland => format!("{home}/.config/hypr/monitors.conf"),
        Compositor::Sway => format!("{home}/.config/sway/monitors.conf"),
        Compositor::River => format!("{home}/.config/river/monitors.conf"),
        Compositor::Unknown => String::new(),
    }
}

pub fn run(
    mut terminal: DefaultTerminal,
    compositor: Compositor,
) -> Result<Option<AppConfig>> {
    let config_path = default_config_path(compositor);
    let cursor = config_path.len();
    let mut state = SetupState {
        input: config_path,
        cursor,
        compositor,
        error: None,
    };

    loop {
        terminal.draw(|f| render(f, &state))?;

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(k) = event::read()?
        {
            match k.code {
                KeyCode::Esc => return Ok(None),
                KeyCode::Char(c) => {
                    state.input.insert(state.cursor, c);
                    state.cursor += c.len_utf8();
                    state.error = None;
                }
                KeyCode::Backspace => {
                    if state.cursor > 0 {
                        let prev = state.prev_cursor();
                        state.input.remove(prev);
                        state.cursor = prev;
                    }
                    state.error = None;
                }
                KeyCode::Delete => {
                    if state.cursor < state.input.len() {
                        state.input.remove(state.cursor);
                    }
                    state.error = None;
                }
                KeyCode::Left => {
                    if state.cursor > 0 {
                        state.cursor = state.prev_cursor();
                    }
                }
                KeyCode::Right => {
                    if state.cursor < state.input.len() {
                        state.cursor = state.next_cursor();
                    }
                }
                KeyCode::Home => state.cursor = 0,
                KeyCode::End => state.cursor = state.input.len(),
                KeyCode::Enter => {
                    let path = state.input.trim();
                    if path.is_empty() {
                        state.error = Some("Path cannot be empty".to_string());
                        continue;
                    }
                    let expanded = expand_tilde(path);
                    return Ok(Some(AppConfig {
                        monitor_config_path: expanded,
                    }));
                }
                _ => {}
            }
        }
    }
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{home}/{rest}");
    }
    path.to_string()
}

const LOGO: &[&str] = &[
    r"░██    ░██ ░██       ░██ ░██         ░███     ░███ ",
    r" ░██  ░██  ░██       ░██ ░██         ░████   ░████ ",
    r"  ░██░██   ░██  ░██  ░██ ░██         ░██░██ ░██░██ ",
    r"   ░███    ░██ ░████ ░██ ░██         ░██ ░████ ░██ ",
    r"  ░██░██   ░██░██ ░██░██ ░██         ░██  ░██  ░██ ",
    r" ░██  ░██  ░████   ░████ ░██         ░██       ░██ ",
    r"░██    ░██ ░███     ░███ ░██████████ ░██       ░██ ",
    r"                                                   ",
];

fn render(frame: &mut Frame, state: &SetupState) {
    let [_, center_v, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Max(19),
        Constraint::Fill(1),
    ])
    .areas(frame.area());

    let [_, center, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Max(90),
        Constraint::Fill(1),
    ])
    .areas(center_v);

    let [logo_area, title_area, desc_area, input_area, info_area] =
        Layout::vertical([
            Constraint::Length(9),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .areas(center);

    let logo_lines: Vec<Line> = LOGO
        .iter()
        .map(|line| {
            Line::from(Span::styled(*line, Style::default().fg(Color::Cyan)))
        })
        .collect();
    frame.render_widget(Paragraph::new(logo_lines), logo_area);

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "xwlm ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "first-time setup let's",
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    frame.render_widget(title, title_area);

    let desc = Paragraph::new(Line::from(Span::styled(
        format!(
            "Enter the path to your {} config file:",
            state.compositor.label()
        ),
        Style::default().fg(Color::White),
    )));
    frame.render_widget(desc, desc_area);

    let (before, after) = state.input.split_at(state.cursor);
    let cursor_char = if after.is_empty() { " " } else { &after[..1] };
    let rest = if after.len() > 1 { &after[1..] } else { "" };

    let input_line = Line::from(vec![
        Span::styled(before, Style::default().fg(Color::White)),
        Span::styled(
            cursor_char,
            Style::default().fg(Color::Black).bg(Color::White),
        ),
        Span::styled(rest, Style::default().fg(Color::White)),
    ]);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Blue))
        .title(" Path ");

    frame.render_widget(
        Paragraph::new(input_line).block(input_block),
        input_area,
    );

    if let Some(ref err) = state.error {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {err}"),
                Style::default().fg(Color::Red),
            ))),
            info_area,
        );
    } else {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("Enter ", Style::default().fg(Color::Cyan)),
                Span::styled("confirm  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Esc ", Style::default().fg(Color::Cyan)),
                Span::styled("quit", Style::default().fg(Color::DarkGray)),
            ])),
            info_area,
        );
    }
}
