use super::app;
use super::monitor;
use color_eyre::eyre::Result;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::read;
use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;

pub fn run(mut terminal: DefaultTerminal) -> Result<()> {
    let monitors = monitor::Monitor::get_monitors()?;
    let mut app = app::App::new();
    app.set_monitor(monitors);
    app.list_state.select(Some(0));

    loop {
        let _ = terminal.draw(|f| render(f, &mut app));
        if let Event::Key(k) = read()? {
            match k.code {
                KeyCode::Esc => break,
                KeyCode::Up => app.next(),
                KeyCode::Down => app.previous(),
                _ => {}
            }
        }
    }
    Ok(())
}

fn render(frame: &mut Frame, app: &mut app::App) {
    let rect = frame.area();
    let items: Vec<ListItem> = app
        .monitors
        .iter()
        .map(|m| ListItem::new(format!("{} @ {}", m.name, m.refresh_rate)))
        .collect();
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

    let monitor_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("monitors list"),
        )
        .highlight_symbol(">")
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::Black));

    frame.render_widget(active_monitor, control_layout[1]);
    frame.render_widget(search_monitor, control_layout[2]);
    frame.render_stateful_widget(
        monitor_list,
        control_layout[3],
        &mut app.list_state,
    );
}
