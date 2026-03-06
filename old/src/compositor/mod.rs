pub mod extraction;
pub mod format;
mod hyprland;
pub mod position;
mod sway;
pub mod workspace_config;

use std::env;

#[derive(Debug, Clone, Copy)]
pub enum Compositor {
    Hyprland,
    Sway,
    River,
    Unknown,
}

impl Compositor {
    pub fn label(self) -> &'static str {
        match self {
            Compositor::Hyprland => "Hyprland",
            Compositor::Sway => "Sway",
            Compositor::River => "River",
            Compositor::Unknown => "Unknown",
        }
    }

    pub fn supports_workspace_defaults(self) -> bool {
        matches!(self, Compositor::Hyprland)
    }
}

pub fn detect() -> Compositor {
    if env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
        return Compositor::Hyprland;
    }

    if env::var_os("SWAYSOCK").is_some() {
        return Compositor::Sway;
    }

    if let Ok(desktop) = env::var("XDG_CURRENT_DESKTOP") {
        let lower = desktop.to_ascii_lowercase();
        for entry in lower.split(':') {
            match entry.trim() {
                "hyprland" => return Compositor::Hyprland,
                "sway" => return Compositor::Sway,
                "river" => return Compositor::River,
                _ => {}
            }
        }
    }

    Compositor::Unknown
}
