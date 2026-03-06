use std::path::PathBuf;

use crate::compositor::Compositor;

#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceRule {
    pub id: usize,
    pub monitor: String,
    pub is_default: bool,
    pub is_persistent: bool,
}

pub fn parse_workspace_config(compositor: Compositor, path: &PathBuf) -> Vec<WorkspaceRule> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    match compositor {
        Compositor::Hyprland => parse_hyprland_workspaces(&content),
        Compositor::Sway => parse_sway_workspaces(&content),
        _ => Vec::new(),
    }
}

fn parse_hyprland_workspaces(content: &str) -> Vec<WorkspaceRule> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            let rest = trimmed.strip_prefix("workspace")?.trim_start();
            let rest = rest.strip_prefix('=')?.trim_start();
            let (id_str, rules) = rest.split_once(',')?;
            let id: usize = id_str.trim().parse().ok()?;

            let rules_str = rules.trim();
            let is_default = rules_str.contains("default:true");
            let is_persistent = rules_str.contains("persistent:true");

            let monitor = extract_monitor_name(rules_str);

            Some(WorkspaceRule {
                id,
                monitor,
                is_default,
                is_persistent,
            })
        })
        .collect()
}

fn extract_monitor_name(rules: &str) -> String {
    if let Some(monitor_part) = rules.strip_prefix("monitor:") {
        let monitor_part = monitor_part.trim();
        if let Some((name, _)) = monitor_part.split_once(',') {
            return name.trim().trim_matches('"').trim_matches(':').to_string();
        }
        return monitor_part
            .trim()
            .trim_matches('"')
            .trim_matches(':')
            .to_string();
    }
    rules.split(',').next().unwrap_or(rules).trim().to_string()
}

fn parse_sway_workspaces(content: &str) -> Vec<WorkspaceRule> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            let rest = trimmed.strip_prefix("workspace")?.trim_start();
            let (id_str, rest) = rest.split_once(char::is_whitespace)?;
            let id: usize = id_str.trim().parse().ok()?;
            let monitor = rest.trim().strip_prefix("output")?.trim().to_string();
            Some(WorkspaceRule {
                id,
                monitor,
                is_default: false,
                is_persistent: false,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hyprland_workspace_with_persistence() {
        let content = r#"
workspace=1,monitor:"DP-1",default:true,persistent:true
workspace=2,monitor:"DP-1",persistent:true
workspace=3,monitor:"HDMI-A-1",persistent:true
"#;
        let result = parse_hyprland_workspaces(content);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].monitor, "DP-1");
        assert!(result[0].is_default);
        assert!(result[0].is_persistent);

        assert_eq!(result[1].id, 2);
        assert!(!result[1].is_default);
        assert!(result[1].is_persistent);

        assert_eq!(result[2].id, 3);
        assert!(result[2].is_persistent);
    }

    #[test]
    fn test_parse_hyprland_workspace_simple() {
        let content = r#"
workspace = 1, monitor:HDMI-A-1
workspace = 2, monitor:eDP-1
"#;
        let result = parse_hyprland_workspaces(content);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].monitor, "HDMI-A-1");
        assert!(result[0].is_default);
        assert!(result[0].is_persistent);
    }

    #[test]
    fn test_extract_monitor_name() {
        assert_eq!(
            extract_monitor_name(r#"monitor:"HDMI-A-1",default:true,persistent:true"#),
            "HDMI-A-1"
        );
        assert_eq!(
            extract_monitor_name("monitor:HDMI-A-1,persistent:true"),
            "HDMI-A-1"
        );
        assert_eq!(extract_monitor_name("monitor:eDP-1"), "eDP-1");
    }
}
