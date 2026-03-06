mod compositor;
mod constants;
mod setup;
mod state;
mod tui;
mod utils;
mod xwlm_config;

use std::{error::Error, io, sync::mpsc};

use wlx_monitors::{WlMonitorManager, WlMonitorManagerError};

use crate::{state::App, xwlm_config::Config};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let (wlx_emitter, wlx_events) = mpsc::sync_channel(16);
    let (wlx_action_handler, wlx_action_rx) = mpsc::sync_channel(16);
    let (wlx_manager, wlx_eq) = WlMonitorManager::new_connection(wlx_emitter, wlx_action_rx)?;

    std::thread::spawn(move || -> Result<(), WlMonitorManagerError> {
        wlx_manager.run(wlx_eq)?;
        Ok(())
    });

    let Some(config) = load()? else { return Ok(()) };

    let mut app = App::new(
        wlx_action_handler,
        config.monitor_config_path,
        config.workspace_count,
    );
    tui::run(&mut app, wlx_events)?;
    Ok(())
}

fn load() -> io::Result<Option<Config>> {
    let comp = compositor::detect();
    let Ok(cfg) = xwlm_config::load_config() else {
        return setup::run(comp).map_err(io::Error::other);
    };

    let path_str = cfg.monitor_config_path.to_string_lossy();
    if !utils::monitor_config_exists(&path_str) {
        eprintln!("Monitor config file not found: {}", path_str);
        eprintln!("Re-running setup...");
        return setup::run(comp).map_err(io::Error::other);
    }

    Ok(Some(cfg))
}
