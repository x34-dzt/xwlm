mod app;
mod config;
mod setup;
mod ui;

use std::sync::mpsc::{self, Receiver};

use color_eyre::eyre::Result;
use wlx_monitors::{WlMonitorEvent, WlMonitorManager};
use xwlm_cfg::Compositor;

use crate::{app::App, config::AppConfig};

fn main() -> Result<()> {
    color_eyre::install()?;

    let compositor = xwlm_cfg::detect();

    let app_config = match load_config(compositor)? {
        Some(cfg) => cfg,
        // when we get none here, we didn't got any config so we do early Ok return
        None => return Ok(()),
    };

    let (emitter, event_receiver) = mpsc::sync_channel(16);
    let (controller, action_receiver) = mpsc::sync_channel(16);

    let (state, event_queue) =
        WlMonitorManager::new_connection(emitter, action_receiver)
            .expect("Failed to connect to Wayland");

    std::thread::spawn(move || {
        state.run(event_queue).expect("Event loop error");
    });

    let app = app::App::new(
        controller,
        compositor,
        app_config.monitor_config_path,
        app_config.workspace_count,
    );
    run_xwlm(app, event_receiver)
}

fn load_config(compositor: Compositor) -> Result<Option<AppConfig>> {
    match config::load()? {
        // app config already exists return Some(cfg)
        Some(cfg) => Ok(Some(cfg)),
        None => {
            // app config doesn't already exists run setup
            let result = run_setup(compositor);
            match result? {
                // if setup is successful we return the config
                Some(cfg) => {
                    config::save(&cfg)?;
                    Ok(Some(cfg))
                }
                // else nothing, close setup as user pressed esc
                None => Ok(None),
            }
        }
    }
}

fn run_setup(compositor: Compositor) -> Result<Option<AppConfig>> {
    let terminal = ratatui::init();
    let result = setup::run(terminal, compositor);
    ratatui::restore();
    result
}

fn run_xwlm(mut app: App, event_rx: Receiver<WlMonitorEvent>) -> Result<()> {
    let terminal = ratatui::init();
    let result = ui::run(terminal, &mut app, event_rx);
    ratatui::restore();
    result
}
