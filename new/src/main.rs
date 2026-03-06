use std::io;

use crate::app::App;

mod app;
mod panes;

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::default().run(terminal))
}
