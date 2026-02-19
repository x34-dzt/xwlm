pub mod hyprland;
pub mod sway;

use std::path::PathBuf;

use crate::Compositor;

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
        let output_path = output_dir.join(
            if self
                .source_line
                .as_ref()
                .is_some_and(|l| l.contains("include"))
            {
                self.source_line
                    .as_ref()
                    .and_then(|l| l.strip_prefix("include "))
                    .unwrap_or("outputs.conf")
            } else {
                self.source_line
                    .as_ref()
                    .and_then(|l| l.strip_prefix("source = "))
                    .unwrap_or("monitors.conf")
            },
        );

        std::fs::write(&output_path, &self.output_content).map_err(|e| {
            format!("Failed to write {}: {e}", output_path.display())
        })?;

        for (path, content) in &self.modified_files {
            std::fs::write(path, content).map_err(|e| {
                format!("Failed to write {}: {e}", path.display())
            })?;
        }

        if let Some(ref line) = self.source_line {
            let mut content = std::fs::read_to_string(&self.main_config)
                .map_err(|e| {
                    format!(
                        "Failed to read {}: {e}",
                        self.main_config.display()
                    )
                })?;
            if !content.ends_with('\n') {
                content.push('\n');
            }
            content.push('\n');
            content.push_str(line);
            content.push('\n');
            std::fs::write(&self.main_config, content).map_err(|e| {
                format!("Failed to write {}: {e}", self.main_config.display())
            })?;
        }

        Ok(())
    }
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

pub(crate) fn resolve_path(base_dir: &std::path::Path, path: &str) -> PathBuf {
    let path = path.trim();
    if let Some(rest) = path.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return PathBuf::from(format!("{home}/{rest}"));
    }
    let p = PathBuf::from(path);
    if p.is_absolute() { p } else { base_dir.join(p) }
}
