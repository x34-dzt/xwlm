pub mod extract;
pub mod format;
pub mod workspace;

use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

pub fn main_config_path(compositor: Compositor) -> Option<PathBuf> {
    let home = env::var("HOME").ok()?;
    let path = match compositor {
        Compositor::Hyprland => format!("{home}/.config/hypr/hyprland.conf"),
        Compositor::Sway => format!("{home}/.config/sway/config"),
        _ => return None,
    };
    let p = PathBuf::from(path);
    if p.exists() { Some(p) } else { None }
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
