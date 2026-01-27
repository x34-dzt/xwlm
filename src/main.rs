use std::default;

use color_eyre::eyre::{Ok, Result};
use crossterm::event::{Event, KeyCode, read};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders},
};

fn main() -> Result<()> {
    println!("Hello, world");
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal) -> Result<()> {
    loop {
        let _ = terminal.draw(render);
        if let Event::Key(k) = read()?
            && let KeyCode::Esc = k.code
        {
            break;
        }
    }
    Ok(())
}

fn render(frame: &mut Frame) {
    let rect = frame.area();
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![
            Constraint::Min(1),
            Constraint::Percentage(70),
            Constraint::Min(1),
        ])
        .split(rect);

    let control_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Length(10),
            Constraint::Min(1),
        ])
        .split(layout[1]);

    let active_monitor = Block::default()
        .borders(Borders::ALL)
        .title("active monitor");
    let search_monitor = Block::default()
        .borders(Borders::ALL)
        .title("search monitor monitor");
    let monitor_list = Block::default().borders(Borders::ALL).title("monitor list");

    frame.render_widget(active_monitor, control_layout[1]);
    frame.render_widget(search_monitor, control_layout[2]);
    frame.render_widget(monitor_list, control_layout[3]);
}
