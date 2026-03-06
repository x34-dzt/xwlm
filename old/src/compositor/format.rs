use std::process::Command;
use std::{io, path::PathBuf};

use wlx_monitors::{WlMonitor, WlTransform};

use crate::compositor::{workspace_config::WorkspaceRule, Compositor};

pub fn reload(compositor: Compositor) {
    let result = match compositor {
        Compositor::Hyprland => Command::new("hyprctl").arg("reload").output(),
        Compositor::Sway => Command::new("swaymsg").arg("reload").output(),
        _ => return,
    };
    if let Err(e) = result {
        eprintln!("Failed to reload compositor: {e}");
    }
}

pub fn save_monitor_config(
    compositor: Compositor,
    path: &PathBuf,
    monitors: &[WlMonitor],
    workspaces: &[WorkspaceRule],
) -> io::Result<()> {
    let content = match compositor {
        Compositor::Hyprland => format_hyprland(monitors, workspaces),
        Compositor::Sway => format_sway(monitors, workspaces),
        Compositor::River => format_river(monitors),
        Compositor::Unknown => return Ok(()),
    };
    let comment = "# This file is managed by xwlm. Do not edit manually.\n\n";
    let final_content = format!("{}{}", comment, content);
    std::fs::write(path, final_content)
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

fn format_hyprland(
    monitors: &[WlMonitor],
    workspaces: &[WorkspaceRule],
) -> String {
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

    let ws_lines: Vec<String> = workspaces
        .iter()
        .map(|ws| {
            let mut rules = format!("monitor:{}", ws.monitor);
            if ws.is_default {
                rules.push_str(",default:true");
            }
            if ws.is_persistent {
                rules.push_str(",persistent:true");
            }
            format!("workspace = {}, {}", ws.id, rules)
        })
        .collect();
    if !ws_lines.is_empty() {
        lines.push(String::new());
        lines.extend(ws_lines);
    }

    lines.push(String::new());
    lines.join("\n")
}

fn format_sway(monitors: &[WlMonitor], workspaces: &[WorkspaceRule]) -> String {
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
            m.name, w, h, refresh, m.position.x, m.position.y, scale, transform,
        ));
    }

    let ws_lines: Vec<String> = workspaces
        .iter()
        .map(|ws| format!("workspace {} output {}", ws.id, ws.monitor))
        .collect();
    if !ws_lines.is_empty() {
        blocks.push(ws_lines.join("\n"));
    }

    blocks.push(String::new());
    blocks.join("\n\n")
}

fn format_river(monitors: &[WlMonitor]) -> String {
    let mut lines = vec!["#!/bin/sh".to_string()];
    for m in monitors {
        if !m.enabled {
            lines.push(format!("wlr-randr --output {} --off", m.name));
            continue;
        }
        let (w, h, refresh) = current_mode(m);
        let scale = format_scale(m.scale);
        let transform = transform_to_sway(m.transform);
        lines.push(format!(
            "wlr-randr --output {} --mode {}x{}@{}Hz --pos {},{} --scale {} --transform {}",
            m.name, w, h, refresh, m.position.x, m.position.y, scale, transform,
        ));
    }
    lines.push(String::new());
    lines.join("\n")
}
