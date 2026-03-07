use std::{error::Error, sync::mpsc};

use wlx_monitors::{WlMonitorManager, WlMonitorManagerError};

use crate::app::App;

mod app;
mod panes;

fn main() -> Result<(), Box<dyn Error>> {
    let (wlx_emitter, wlx_events) = mpsc::sync_channel(16);
    let (wlx_action_handler, wlx_action_rx) = mpsc::sync_channel(16);
    let (wlx_manager, wlx_eq) = WlMonitorManager::new_connection(wlx_emitter, wlx_action_rx)?;

    std::thread::spawn(move || -> Result<(), WlMonitorManagerError> {
        wlx_manager.run(wlx_eq)?;
        Ok(())
    });

    ratatui::run(|terminal| App::default().run(terminal, wlx_events))?;
    Ok(())
}
