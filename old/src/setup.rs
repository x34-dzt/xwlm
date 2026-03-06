use std::io::{self};
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode};
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::CrosstermBackend;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::{DefaultTerminal, Frame, Terminal};

use crate::compositor::Compositor;
use crate::compositor::extraction::{ExtractionPlan, extract_monitors, main_config_path};
use crate::utils::expand_tilde;
use crate::xwlm_config::{self, Config, save_config};

enum SetupPhase {
    Extraction,
    Manual,
}

struct ExtractionResult {
    plan: ExtractionPlan,
    output_path: String,
    source_files: Vec<String>,
    monitor_count: usize,
    already_consolidated: bool,
}

struct SetupState {
    input: String,
    cursor: usize,
    compositor: Compositor,
    error: Option<String>,
    phase: SetupPhase,
    extraction: Option<ExtractionResult>,
    warned: bool,
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
    match compositor {
        Compositor::Hyprland => "~/.config/hypr/monitors.conf".to_string(),
        Compositor::Sway => "~/.config/sway/output.conf".to_string(),
        Compositor::River => "~/.config/river/monitors.conf".to_string(),
        Compositor::Unknown => String::new(),
    }
}

fn get_monitors_config_name(compositor: Compositor) -> &'static str {
    match compositor {
        Compositor::Hyprland => "monitors.conf",
        Compositor::Sway => "output.conf",
        Compositor::River => "monitors.conf",
        Compositor::Unknown => "monitors.conf",
    }
}

fn get_outputfile_name(compositor: Compositor) -> String {
    get_monitors_config_name(compositor).to_string()
}

fn attempt_extraction(compositor: Compositor) -> Option<ExtractionResult> {
    let main_config = main_config_path(compositor)?;
    let output_filename = get_outputfile_name(compositor);

    let plan = extract_monitors(&main_config, compositor, &output_filename).ok()?;

    if !plan.has_monitors() {
        return None;
    }

    let output_path = main_config
        .parent()?
        .join(output_filename)
        .to_string_lossy()
        .to_string();

    let source_files: Vec<String> = plan
        .modified_files
        .iter()
        .map(|(p, _)| p.to_string_lossy().to_string())
        .collect();

    let monitor_count = plan
        .output_content
        .lines()
        .filter(|l| {
            let trimmed = l.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .count();

    let already_consolidated = plan.source_exists
        && source_files.len() <= 1
        && source_files.first().is_some_and(|f| f == &output_path);

    Some(ExtractionResult {
        plan,
        output_path,
        source_files,
        monitor_count,
        already_consolidated,
    })
}

pub fn run(compositor: Compositor) -> Result<Option<Config>, xwlm_config::ConfigError> {
    let result = run_setup(compositor).map_err(io::Error::other)?;
    match result {
        Some(cfg) => {
            save_config(&cfg)?;
            Ok(Some(cfg))
        }
        None => Ok(None),
    }
}

fn run_setup(compositor: Compositor) -> io::Result<Option<Config>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = init(&mut terminal, compositor);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    result
}

pub fn init(terminal: &mut DefaultTerminal, compositor: Compositor) -> io::Result<Option<Config>> {
    let extraction = attempt_extraction(compositor);

    let (phase, config_path) = match &extraction {
        Some(result) => (SetupPhase::Extraction, result.output_path.clone()),
        None => (SetupPhase::Manual, default_config_path(compositor)),
    };

    let cursor = config_path.clone().len();

    let mut state = SetupState {
        input: config_path.clone(),
        cursor,
        compositor,
        error: None,
        phase,
        extraction,
        warned: false,
    };

    loop {
        terminal.draw(|f| render(f, &state))?;

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(k) = event::read()?
        {
            match (&state.phase, k.code) {
                (SetupPhase::Extraction, KeyCode::Enter) => {
                    let Some(ref result) = state.extraction else {
                        continue;
                    };
                    if !result.already_consolidated
                        && let Err(e) = result.plan.apply()
                    {
                        state.error = Some(format!("Extraction failed: {e}"));
                        state.phase = SetupPhase::Manual;
                        continue;
                    }
                    return Ok(Some(Config {
                        monitor_config_path: PathBuf::from(config_path),
                        workspace_count: 10,
                    }));
                }
                (SetupPhase::Extraction, KeyCode::Char('m')) => {
                    state.phase = SetupPhase::Manual;
                    state.input = default_config_path(compositor);
                    state.cursor = state.input.len();
                    state.error = None;
                    state.warned = false;
                }
                (SetupPhase::Extraction, KeyCode::Esc) => return Ok(None),

                // --- Manual phase ---
                (SetupPhase::Manual, KeyCode::Esc) => return Ok(None),
                (SetupPhase::Manual, KeyCode::Char(c)) => {
                    state.input.insert(state.cursor, c);
                    state.cursor += c.len_utf8();
                    state.error = None;
                    state.warned = false;
                }
                (SetupPhase::Manual, KeyCode::Backspace) => {
                    if state.cursor > 0 {
                        let prev = state.prev_cursor();
                        state.input.remove(prev);
                        state.cursor = prev;
                    }
                    state.error = None;
                    state.warned = false;
                }
                (SetupPhase::Manual, KeyCode::Delete) => {
                    if state.cursor < state.input.len() {
                        state.input.remove(state.cursor);
                    }
                    state.error = None;
                    state.warned = false;
                }
                (SetupPhase::Manual, KeyCode::Left) => {
                    if state.cursor > 0 {
                        state.cursor = state.prev_cursor();
                    }
                }
                (SetupPhase::Manual, KeyCode::Right) => {
                    if state.cursor < state.input.len() {
                        state.cursor = state.next_cursor();
                    }
                }
                (SetupPhase::Manual, KeyCode::Home) => state.cursor = 0,
                (SetupPhase::Manual, KeyCode::End) => state.cursor = state.input.len(),
                (SetupPhase::Manual, KeyCode::Enter) => {
                    let path = state.input.trim();
                    if path.is_empty() {
                        state.error = Some("Path cannot be empty".to_string());
                        continue;
                    }

                    if !state.warned {
                        state.warned = true;
                        state.error = Some("Are you sure? This file will be overwritten with monitor settings. If it's your main Hyprland/Sway config (like hyprland.conf), you will LOSE all your keybinds, animations, window rules, and other settings! Use a separate file like monitors.conf or output.conf instead. Press Enter again to confirm.".to_string());
                        continue;
                    }

                    let expanded = expand_tilde(path).map_err(io::Error::other)?;
                    if !expanded.exists() {
                        state.error = Some("File does not exist. Please enter a valid path.".to_string());
                        state.warned = false;
                        continue;
                    }

                    return Ok(Some(Config {
                        monitor_config_path: expanded,
                        workspace_count: 10,
                    }));
                }
                _ => {}
            }
        }
    }
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
    match state.phase {
        SetupPhase::Extraction => render_extraction(frame, state),
        SetupPhase::Manual => render_manual(frame, state),
    }
}

fn render_logo(frame: &mut Frame, area: Rect) {
    let logo_lines: Vec<Line> = LOGO
        .iter()
        .map(|line| Line::from(Span::styled(*line, Style::default().fg(Color::Cyan))))
        .collect();
    frame.render_widget(Paragraph::new(logo_lines), area);
}

fn render_title(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "xwlm ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("first-time setup", Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(title, area);
}

fn render_extraction(frame: &mut Frame, state: &SetupState) {
    let extraction = match state.extraction {
        Some(ref e) => e,
        None => return,
    };

    let file_count = extraction.source_files.len().max(1) as u16;

    let [_, center_v, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Max(16 + file_count),
        Constraint::Fill(1),
    ])
    .areas(frame.area());

    let [_, center, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Max(90),
        Constraint::Fill(1),
    ])
    .areas(center_v);

    let [
        logo_area,
        title_area,
        desc_area,
        files_area,
        output_area,
        info_area,
    ] = Layout::vertical([
        Constraint::Length(9),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(file_count),
        Constraint::Length(2),
        Constraint::Length(2),
    ])
    .areas(center);

    render_logo(frame, logo_area);
    render_title(frame, title_area);

    if extraction.already_consolidated {
        let desc = Paragraph::new(Line::from(Span::styled(
            format!(
                "Detected existing {} monitor config at:",
                state.compositor.label()
            ),
            Style::default().fg(Color::White),
        )));
        frame.render_widget(desc, desc_area);

        let path_line = Line::from(Span::styled(
            format!("  {}", extraction.output_path),
            Style::default().fg(Color::Cyan),
        ));
        frame.render_widget(Paragraph::new(path_line), files_area);

        frame.render_widget(Paragraph::new(""), output_area);
    } else {
        let desc = Paragraph::new(Line::from(Span::styled(
            format!(
                "Found {} monitor config line(s) in:",
                extraction.monitor_count
            ),
            Style::default().fg(Color::White),
        )));
        frame.render_widget(desc, desc_area);

        let file_lines: Vec<Line> = extraction
            .source_files
            .iter()
            .map(|f| {
                Line::from(Span::styled(
                    format!("  {f}"),
                    Style::default().fg(Color::Cyan),
                ))
            })
            .collect();
        frame.render_widget(Paragraph::new(file_lines), files_area);

        let output = Paragraph::new(Line::from(vec![
            Span::styled("Consolidate to: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&extraction.output_path, Style::default().fg(Color::Cyan)),
        ]));
        frame.render_widget(output, output_area);
    }

    if let Some(ref err) = state.error {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {err}"),
                Style::default().fg(Color::Red),
            ))),
            info_area,
        );
    } else {
        let mut hints = vec![
            Span::styled("Enter ", Style::default().fg(Color::Cyan)),
            Span::styled("confirm  ", Style::default().fg(Color::DarkGray)),
        ];
        hints.push(Span::styled("m ", Style::default().fg(Color::Cyan)));
        hints.push(Span::styled(
            "manual  ",
            Style::default().fg(Color::DarkGray),
        ));
        hints.push(Span::styled("Esc ", Style::default().fg(Color::Cyan)));
        hints.push(Span::styled("quit", Style::default().fg(Color::DarkGray)));
        frame.render_widget(Paragraph::new(Line::from(hints)), info_area);
    }
}

fn render_manual(frame: &mut Frame, state: &SetupState) {
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

    let [logo_area, title_area, desc_area, warning_area, input_area, info_area] = Layout::vertical([
        Constraint::Length(9),
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(2),
    ])
    .areas(center);

    render_logo(frame, logo_area);
    render_title(frame, title_area);

    let desc = Paragraph::new(Line::from(Span::styled(
        format!(
            "Enter the path to your {} monitor config file:",
            state.compositor.label()
        ),
        Style::default().fg(Color::White),
    )));
    frame.render_widget(desc, desc_area);

    let warning = Paragraph::new(Line::from(Span::styled(
        "WARNING: Don't use your main config file! Use a separate file like monitors.conf",
        Style::default().fg(Color::Yellow),
    )));
    frame.render_widget(warning, warning_area);

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

    frame.render_widget(Paragraph::new(input_line).block(input_block), input_area);

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
