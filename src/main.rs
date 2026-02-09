mod app;
mod ui;

use std::sync::mpsc;

use color_eyre::eyre::Result;
use wlx_monitors::WlMonitorManager;

fn main() -> Result<()> {
    color_eyre::install()?;

    let (emitter, event_receiver) = mpsc::sync_channel(16);
    let (controller, action_receiver) = mpsc::sync_channel(16);

    let (state, event_queue) =
        WlMonitorManager::new_connection(emitter, action_receiver)
            .expect("Failed to connect to Wayland");

    std::thread::spawn(move || {
        state.run(event_queue).expect("Event loop error");
    });

    let mut app = app::App::new(controller);
    let terminal = ratatui::init();
    let result = ui::run(terminal, &mut app, event_receiver);
    ratatui::restore();
    result
}
