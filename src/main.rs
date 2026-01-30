mod app;
mod hypr_monitor;
mod monitor;
mod ui;

use color_eyre::eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = ui::run(terminal);
    ratatui::restore();
    result

    // if let Ok(m) = hypr_monitor::HyprMonitor::get_hypr_monitors() {
    //     for mn in m {
    //         println!(
    //             "Name: {}\nFocused: {}\nRefresh rate: {}\n Available modes:{:?}\n",
    //             mn.name, mn.focused, mn.refresh_rate, mn.available_modes
    //         )
    //     }
    // };
}
