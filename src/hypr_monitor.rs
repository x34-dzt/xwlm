use std::process::Command;

use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HyprMonitor {
    pub name: String,
    pub refresh_rate: f32,
    pub available_modes: Vec<String>,
}

impl HyprMonitor {
    pub fn get_hypr_monitors() -> Result<Vec<HyprMonitor>> {
        let hyprctl_output =
            Command::new("hyprctl").args(["monitors", "-j"]).output()?;
        let hyprctl_json_output_string =
            String::from_utf8(hyprctl_output.stdout)?;
        let hyprctl_monitors = serde_json::from_str::<Vec<HyprMonitor>>(
            &hyprctl_json_output_string,
        )?;
        Ok(hyprctl_monitors)
    }
}
