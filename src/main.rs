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
}
