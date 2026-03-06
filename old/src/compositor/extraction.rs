use std::{env, path::PathBuf};

use crate::compositor::{Compositor, hyprland, sway};

#[derive(Debug)]
pub struct ExtractionPlan {
    pub output_content: String,
    pub modified_files: Vec<(PathBuf, String)>,
    pub source_line: Option<String>,
    pub main_config: PathBuf,
    pub source_exists: bool,
}

impl ExtractionPlan {
    pub fn has_monitors(&self) -> bool {
        !self.output_content.is_empty()
    }

    pub fn apply(&self) -> Result<(), String> {
        if self.output_content.is_empty() {
            return Err("No monitor configuration found to extract".into());
        }

        let output_dir = self
            .main_config
            .parent()
            .ok_or("Cannot determine config directory")?;

        let output_filename = self.extract_output_filename();
        let output_path = output_dir.join(output_filename);

        // Step 1: Write the monitors.conf file first
        let comment = "# This file is managed by xwlm. Do not edit manually.\n\n";
        let final_content = format!("{}{}", comment, self.output_content);
        std::fs::write(&output_path, final_content)
            .map_err(|e| format!("Failed to write {}: {e}", output_path.display()))?;

        // Step 2: Write modified files, adding source line to main_config if needed
        for (path, content) in &self.modified_files {
            if path == &self.main_config {
                // For main config, add source line to the modified content
                let mut final_content = content.clone();
                if !final_content.ends_with('\n') {
                    final_content.push('\n');
                }
                if let Some(ref line) = self.source_line {
                    final_content.push('\n');
                    final_content.push_str(line);
                    final_content.push('\n');
                }
                std::fs::write(path, final_content)
                    .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
            } else {
                std::fs::write(path, content)
                    .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
            }
        }

        // Step 3: If main_config wasn't in modified_files but we need to add source
        if !self
            .modified_files
            .iter()
            .any(|(p, _)| p == &self.main_config)
            && let Some(ref line) = self.source_line
        {
            let mut content = std::fs::read_to_string(&self.main_config)
                .map_err(|e| format!("Failed to read {}: {e}", self.main_config.display()))?;
            if !content.ends_with('\n') {
                content.push('\n');
            }
            content.push('\n');
            content.push_str(line);
            content.push('\n');
            std::fs::write(&self.main_config, content)
                .map_err(|e| format!("Failed to write {}: {e}", self.main_config.display()))?;
        }

        Ok(())
    }

    fn extract_output_filename(&self) -> &str {
        if let Some(ref line) = self.source_line {
            if let Some(path) = line.strip_prefix("source = ") {
                return extract_filename(path);
            }
            if let Some(path) = line.strip_prefix("include ") {
                return extract_filename(path);
            }
        }
        "monitors.conf"
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

fn extract_filename(path: &str) -> &str {
    let path = path.trim();
    path.rsplit('/').next().unwrap_or(path)
}

pub fn extract_monitors(
    config_path: &std::path::Path,
    compositor: Compositor,
    output_filename: &str,
) -> Result<ExtractionPlan, String> {
    match compositor {
        Compositor::Hyprland => hyprland::extract(config_path, output_filename),
        Compositor::Sway => sway::extract(config_path, output_filename),
        _ => Err(format!(
            "Config extraction not supported for {}",
            compositor.label()
        )),
    }
}

pub fn resolve_path(base_dir: &std::path::Path, path: &str) -> PathBuf {
    let path = path.trim();
    if let Some(rest) = path.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return PathBuf::from(format!("{home}/{rest}"));
    }
    let p = PathBuf::from(path);
    if p.is_absolute() { p } else { base_dir.join(p) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_filename_with_tilde() {
        assert_eq!(
            extract_filename("~/.config/hypr/monitors.conf"),
            "monitors.conf"
        );
    }

    #[test]
    fn test_extract_filename_with_absolute() {
        assert_eq!(
            extract_filename("/home/user/.config/hypr/monitors.conf"),
            "monitors.conf"
        );
    }

    #[test]
    fn test_extract_filename_relative() {
        assert_eq!(extract_filename("monitors.conf"), "monitors.conf");
    }

    #[test]
    fn test_extract_filename_with_spaces() {
        assert_eq!(
            extract_filename("  ~/.config/hypr/monitors.conf  "),
            "monitors.conf"
        );
    }

    #[test]
    fn test_extract_filename_nested() {
        assert_eq!(
            extract_filename("~/.config/hypr/subdir/monitors.conf"),
            "monitors.conf"
        );
    }
}
