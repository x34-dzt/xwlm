use std::env;
use std::io;

use wlx_monitors::{WlMonitor, WlTransform};

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

pub fn save_monitor_config(
    compositor: Compositor,
    path: &str,
    monitors: &[WlMonitor],
) -> io::Result<()> {
    let content = match compositor {
        Compositor::Hyprland => format_hyprland(monitors),
        Compositor::Sway => format_sway(monitors),
        Compositor::River => format_river(monitors),
        Compositor::Unknown => return Ok(()),
    };
    std::fs::write(path, content)
}

fn current_mode(monitor: &WlMonitor) -> (i32, i32, i32) {
    monitor
        .modes
        .iter()
        .find(|m| m.is_current)
        .map(|m| (m.resolution.width, m.resolution.height, m.refresh_rate))
        .unwrap_or((0, 0, 60))
}

fn format_scale(scale: f64) -> String {
    if (scale - scale.round()).abs() < 0.001 {
        format!("{}", scale as i32)
    } else {
        format!("{:.2}", scale)
    }
}

fn transform_to_hyprland(t: WlTransform) -> u8 {
    match t {
        WlTransform::Normal => 0,
        WlTransform::Rotate90 => 1,
        WlTransform::Rotate180 => 2,
        WlTransform::Rotate270 => 3,
        WlTransform::Flipped => 4,
        WlTransform::Flipped90 => 5,
        WlTransform::Flipped180 => 6,
        WlTransform::Flipped270 => 7,
    }
}

fn transform_to_sway(t: WlTransform) -> &'static str {
    match t {
        WlTransform::Normal => "normal",
        WlTransform::Rotate90 => "90",
        WlTransform::Rotate180 => "180",
        WlTransform::Rotate270 => "270",
        WlTransform::Flipped => "flipped",
        WlTransform::Flipped90 => "flipped-90",
        WlTransform::Flipped180 => "flipped-180",
        WlTransform::Flipped270 => "flipped-270",
    }
}

fn format_hyprland(monitors: &[WlMonitor]) -> String {
    let mut lines = Vec::new();
    for m in monitors {
        let (w, h, refresh) = current_mode(m);
        let scale = format_scale(m.scale);
        let base = format!(
            "monitor = {}, {}x{}@{}, {}x{}, {}",
            m.name, w, h, refresh, m.position.x, m.position.y, scale,
        );
        if m.transform != WlTransform::Normal {
            lines.push(format!(
                "{}, transform, {}",
                base,
                transform_to_hyprland(m.transform),
            ));
        } else {
            lines.push(base);
        }
        if !m.enabled {
            lines.push(format!("monitor = {}, disable", m.name));
        }
    }
    lines.push(String::new());
    lines.join("\n")
}

fn format_sway(monitors: &[WlMonitor]) -> String {
    let mut blocks = Vec::new();
    for m in monitors {
        if !m.enabled {
            blocks.push(format!("output {} disable", m.name));
            continue;
        }
        let (w, h, refresh) = current_mode(m);
        let scale = format_scale(m.scale);
        let transform = transform_to_sway(m.transform);
        blocks.push(format!(
            "output {} {{\n    mode {}x{}@{}Hz\n    pos {} {}\n    scale {}\n    transform {}\n}}",
            m.name, w, h, refresh,
            m.position.x, m.position.y,
            scale, transform,
        ));
    }
    blocks.push(String::new());
    blocks.join("\n\n")
}

fn format_river(monitors: &[WlMonitor]) -> String {
    let mut lines = vec!["#!/bin/sh".to_string()];
    for m in monitors {
        if !m.enabled {
            continue;
        }
        let (w, h, refresh) = current_mode(m);
        let scale = format_scale(m.scale);
        let transform = transform_to_sway(m.transform);
        lines.push(format!(
            "riverctl output-mode {} {}x{}@{}",
            m.name, w, h, refresh,
        ));
        lines.push(format!("riverctl output-scale {} {}", m.name, scale));
        lines.push(format!(
            "riverctl output-transform {} {}",
            m.name, transform,
        ));
    }
    lines.push(String::new());
    lines.join("\n")
}
