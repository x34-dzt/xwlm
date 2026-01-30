use super::hypr_monitor;
use color_eyre::eyre::Result;

#[derive(Debug)]
pub struct Monitor {
    pub name: String,
    pub refresh_rate: String,
    pub active: bool,
}

impl Monitor {
    pub fn get_monitors() -> Result<Vec<Monitor>> {
        let monitors = hypr_monitor::HyprMonitor::get_hypr_monitors()?;
        let monitor_names: Vec<Monitor> = monitors
            .iter()
            .flat_map(|monitor| {
                monitor.available_modes.iter().filter_map(|m| {
                    m.split("@").nth(1).map(|h| Monitor {
                        refresh_rate: h.to_string(),
                        name: monitor.name.clone(),
                        active: monitor.focused,
                    })
                })
            })
            .collect();
        Ok(monitor_names)
    }
}
