use std::{fs, path::PathBuf};

use crate::compositor::{hyprland, sway, Compositor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigPosition {
    pub x: i32,
    pub y: i32,
}

pub fn get_position(
    compositor: Compositor,
    config_path: &PathBuf,
    monitor_name: &str,
) -> Option<ConfigPosition> {
    if !config_path.exists() {
        return None;
    }

    let content = fs::read_to_string(config_path).ok()?;

    match compositor {
        Compositor::Hyprland => hyprland::config_position(&content, monitor_name),
        Compositor::Sway => sway::config_position(&content, monitor_name),
        _ => None,
    }
}
