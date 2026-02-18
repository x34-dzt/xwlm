mod app;
mod compositor;
mod config;
mod setup;
mod ui;

use std::sync::mpsc;

use color_eyre::eyre::Result;
use wlx_monitors::WlMonitorManager;

fn main() -> Result<()> {
    color_eyre::install()?;

    let compositor = compositor::detect();

    let app_config = match config::load()? {
        Some(cfg) => cfg,
        None => {
            let terminal = ratatui::init();
            let result = setup::run(terminal, compositor);
            ratatui::restore();
            match result? {
                Some(cfg) => {
                    config::save(&cfg)?;
                    cfg
                }
                None => return Ok(()),
            }
        }
    };

    let (emitter, event_receiver) = mpsc::sync_channel(16);
    let (controller, action_receiver) = mpsc::sync_channel(16);

    let (state, event_queue) =
        WlMonitorManager::new_connection(emitter, action_receiver)
            .expect("Failed to connect to Wayland");

    std::thread::spawn(move || {
        state.run(event_queue).expect("Event loop error");
    });

    let mut app = app::App::new(
        controller,
        compositor,
        app_config.monitor_config_path,
        app_config.workspace_count,
    );
    let terminal = ratatui::init();
    let result = ui::run(terminal, &mut app, event_receiver);
    ratatui::restore();
    result
}
