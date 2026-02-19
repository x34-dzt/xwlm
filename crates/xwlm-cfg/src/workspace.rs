use crate::Compositor;

pub fn parse_workspace_config(
    compositor: Compositor,
    path: &str,
) -> Vec<(usize, String)> {
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

fn parse_hyprland_workspaces(content: &str) -> Vec<(usize, String)> {
    content
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("workspace")?.trim_start();
            let rest = rest.strip_prefix('=')?.trim_start();
            let (id_str, rest) = rest.split_once(',')?;
            let id: usize = id_str.trim().parse().ok()?;
            let monitor = rest.trim().strip_prefix("monitor:")?;
            Some((id, monitor.trim().to_string()))
        })
        .collect()
}

fn parse_sway_workspaces(content: &str) -> Vec<(usize, String)> {
    content
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("workspace")?.trim_start();
            let (id_str, rest) = rest.split_once(char::is_whitespace)?;
            let id: usize = id_str.trim().parse().ok()?;
            let monitor = rest.trim().strip_prefix("output")?.trim_start();
            Some((id, monitor.to_string()))
        })
        .collect()
}
