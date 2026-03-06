mod key_binds;
mod layout;
mod panels;
mod ui;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, prelude::CrosstermBackend};
use std::{io, sync::mpsc::Receiver};
use wlx_monitors::WlMonitorEvent;

use crate::state::App;

pub fn run(app: &mut App, wlx_events: Receiver<WlMonitorEvent>) -> Result<(), ui::TuiLoopError> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    ui::tui_loop(app, wlx_events, &mut terminal)?;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}
