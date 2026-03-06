use std::{env, io, path::PathBuf};

use thiserror::Error;
use wlx_monitors::{WlMonitor, WlTransform};

#[derive(Error, Debug)]
pub enum UtilsError {
    #[error("path must start with ~/")]
    NoTilde,

    #[error("no home variable was found")]
    NoHome,

    #[error(transparent)]
    Io(#[from] io::Error),
}

pub fn expand_tilde(path: &str) -> Result<PathBuf, UtilsError> {
    let Some(clean_path) = path.strip_prefix("~/") else {
        return Err(UtilsError::NoTilde);
    };

    let home = env::var("HOME").map_err(|_| UtilsError::NoHome)?;
    Ok(PathBuf::from(home).join(clean_path))
}

pub fn monitor_config_exists(path: &str) -> bool {
    let path_buf = if path.starts_with("~/") {
        match expand_tilde(path) {
            Ok(p) => p,
            Err(_) => return false,
        }
    } else {
        PathBuf::from(path)
    };

    path_buf.exists()
}

pub fn monitor_resolution(monitor: &WlMonitor) -> (i32, i32) {
    if let Some(mode) = monitor.modes.iter().find(|m| m.is_current) {
        return (mode.resolution.width, mode.resolution.height);
    }
    if let Some(mode) = monitor.modes.iter().find(|m| m.preferred) {
        return (mode.resolution.width, mode.resolution.height);
    }
    if let Some(mode) = monitor.modes.first() {
        return (mode.resolution.width, mode.resolution.height);
    }
    (monitor.resolution.width, monitor.resolution.height)
}

pub fn effective_dimensions(monitor: &WlMonitor) -> (i32, i32) {
    let (w, h) = monitor_resolution(monitor);
    match monitor.transform {
        WlTransform::Rotate90
        | WlTransform::Rotate270
        | WlTransform::Flipped90
        | WlTransform::Flipped270 => (h, w),
        _ => (w, h),
    }
}

pub fn transform_label(t: WlTransform) -> &'static str {
    match t {
        WlTransform::Normal => "Normal",
        WlTransform::Rotate90 => "Rotate 90",
        WlTransform::Rotate180 => "Rotate 180",
        WlTransform::Rotate270 => "Rotate 270",
        WlTransform::Flipped => "Flipped",
        WlTransform::Flipped90 => "Flipped 90",
        WlTransform::Flipped180 => "Flipped 180",
        WlTransform::Flipped270 => "Flipped 270",
    }
}
