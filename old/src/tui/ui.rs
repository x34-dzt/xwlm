use std::sync::mpsc::SendError;
use std::{io, sync::mpsc::Receiver, time::Duration};

use crossterm::event::{self, Event, KeyCode};
use ratatui::{DefaultTerminal, Terminal, backend::CrosstermBackend};
use thiserror::Error;
use wlx_monitors::WlMonitorEvent;

use crate::state::{App, Panel};
use crate::tui::layout;

#[derive(Error, Debug)]
pub enum TuiLoopError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("wlx_monitors error: {0}")]
    WlxMonitorActionError(#[from] SendError<wlx_monitors::WlMonitorAction>),
}

pub fn tui_loop(
    app: &mut App,
    wlx_events: Receiver<WlMonitorEvent>,
    terminal: &mut DefaultTerminal,
) -> Result<(), TuiLoopError> {
    loop {
        let mut had_events = false;
        while let Ok(event) = wlx_events.try_recv() {
            had_events = true;
            match event {
                WlMonitorEvent::InitialState(monitors) => {
                    app.set_monitors(monitors);
                }
                WlMonitorEvent::Changed(monitor) => {
                    app.update_monitor(*monitor);
                }
                WlMonitorEvent::Removed { name, .. } => {
                    app.remove_monitor(&name);
                }
                WlMonitorEvent::ActionFailed { action: _, reason } => {
                    app.needs_save = false;
                    app.set_error(format!("Action failed: {}", reason));
                }
            }
        }

        if had_events {
            app.save_config();
        }

        render(terminal, app)?;

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(k) = event::read()?
        {
            app.clear_error();

            if app.pending_last_toggle_monitor {
                match k.code {
                    KeyCode::Char('y') => {
                        if let Err(e) = app.toggle_monitor() {
                            app.set_error(format!("Failed to toggle monitor: {}", e));
                        }
                    }
                    _ => app.dismiss_warning(),
                }
            } else {
                match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        app.reset_positions();
                        break;
                    }
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Left | KeyCode::Char('h') => app.nav_left(),
                    KeyCode::Right | KeyCode::Char('l') => app.nav_right(),
                    KeyCode::Tab => app.toggle_panel(),
                    KeyCode::Char('t') => {
                        if let Err(e) = app.toggle_monitor() {
                            app.set_error(format!("Failed to toggle monitor: {}", e));
                        }
                    }
                    KeyCode::Char('r') => app.reset_positions(),
                    KeyCode::Char(']') => app.select_next_monitor(),
                    KeyCode::Char('[') => app.select_prev_monitor(),
                    KeyCode::Char('+') => {
                        if app.panel == Panel::Monitor {
                            app.zoom_in();
                        } else {
                            app.scale_up();
                        }
                    }
                    KeyCode::Char('-') => {
                        if app.panel == Panel::Monitor {
                            app.zoom_out();
                        } else {
                            app.scale_down();
                        }
                    }
                    KeyCode::Char('d') => {
                        if app.panel == Panel::Workspace
                            && app.compositor.supports_workspace_defaults()
                        {
                            app.toggle_default();
                        }
                    }
                    KeyCode::Char('p') => {
                        if app.panel == Panel::Workspace
                            && app.compositor.supports_workspace_defaults()
                        {
                            app.toggle_persistent();
                        }
                    }
                    KeyCode::Enter => {
                        if let Err(e) = app.apply_action() {
                            app.set_error(format!("Failed to apply: {}", e));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

pub fn render(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    terminal.draw(|f| layout::draw(f, app))?;
    Ok(())
}
