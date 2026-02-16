use std::sync::mpsc::SyncSender;

use ratatui::widgets::ListState;
use wlx_monitors::{WlMonitor, WlMonitorAction, WlTransform};

use crate::compositor::Compositor;

pub const TRANSFORMS: [WlTransform; 8] = [
    WlTransform::Normal,
    WlTransform::Rotate90,
    WlTransform::Rotate180,
    WlTransform::Rotate270,
    WlTransform::Flipped,
    WlTransform::Flipped90,
    WlTransform::Flipped180,
    WlTransform::Flipped270,
];

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

#[derive(PartialEq)]
pub enum Panel {
    Monitors,
    Modes,
    Scale,
    Transform,
}

pub struct App {
    pub monitors: Vec<WlMonitor>,
    pub monitor_state: ListState,
    pub mode_state: ListState,
    pub transform_state: ListState,
    pub pending_scale: f64,
    pub panel: Panel,
    pub controller: SyncSender<WlMonitorAction>,
    pub compositor: Compositor,
    pub monitor_config_path: String,
    pub needs_save: bool,
}

impl App {
    pub fn new(
        controller: SyncSender<WlMonitorAction>,
        compositor: Compositor,
        monitor_config_path: String,
    ) -> Self {
        Self {
            monitors: Vec::new(),
            monitor_state: ListState::default(),
            mode_state: ListState::default(),
            transform_state: ListState::default().with_selected(Some(0)),
            pending_scale: 1.0,
            panel: Panel::Monitors,
            controller,
            compositor,
            monitor_config_path,
            needs_save: false,
        }
    }

    pub fn save_config(&mut self) {
        if !self.needs_save || self.monitor_config_path.is_empty() {
            return;
        }
        self.needs_save = false;
        if let Err(e) = crate::compositor::save_monitor_config(
            self.compositor,
            &self.monitor_config_path,
            &self.monitors,
        ) {
            eprintln!("Failed to save monitor config: {e}");
        }
    }

    pub fn selected_monitor(&self) -> Option<&WlMonitor> {
        self.monitor_state
            .selected()
            .and_then(|i| self.monitors.get(i))
    }

    pub fn set_monitors(&mut self, monitors: Vec<WlMonitor>) {
        self.monitors = monitors;
        if !self.monitors.is_empty() {
            self.monitor_state.select(Some(0));
            self.mode_state.select(Some(0));
            self.sync_scale_and_transform();
        }
    }

    pub fn update_monitor(&mut self, monitor: WlMonitor) {
        if let Some(existing) =
            self.monitors.iter_mut().find(|m| m.name == monitor.name)
        {
            *existing = monitor;
        }
        self.sync_scale_and_transform();
    }

    pub fn remove_monitor(&mut self, name: &str) {
        self.monitors.retain(|m| m.name != name || !m.enabled);
        if let Some(selected) = self.monitor_state.selected()
            && selected >= self.monitors.len()
        {
            self.monitor_state
                .select(Some(self.monitors.len().saturating_sub(1)));
        }
        self.sync_scale_and_transform();
    }

    fn sync_scale_and_transform(&mut self) {
        let Some(idx) = self.monitor_state.selected() else {
            return;
        };
        let Some(monitor) = self.monitors.get(idx) else {
            return;
        };
        let scale = monitor.scale;
        let transform = monitor.transform;

        self.pending_scale = scale;
        if let Some(tidx) = TRANSFORMS.iter().position(|&x| x == transform) {
            self.transform_state.select(Some(tidx));
        }
    }

    pub fn next(&mut self) {
        match self.panel {
            Panel::Monitors => {
                let len = self.monitors.len();
                if len == 0 {
                    return;
                }
                let i = self
                    .monitor_state
                    .selected()
                    .map(|i| (i + 1) % len)
                    .unwrap_or(0);
                self.monitor_state.select(Some(i));
                self.mode_state.select(Some(0));
                self.sync_scale_and_transform();
            }
            Panel::Modes => {
                let len =
                    self.selected_monitor().map(|m| m.modes.len()).unwrap_or(0);
                if len == 0 {
                    return;
                }
                let i = self
                    .mode_state
                    .selected()
                    .map(|i| (i + 1) % len)
                    .unwrap_or(0);
                self.mode_state.select(Some(i));
            }
            Panel::Scale => {
                self.scale_up();
            }
            Panel::Transform => {
                let len = TRANSFORMS.len();
                let i = self
                    .transform_state
                    .selected()
                    .map(|i| (i + 1) % len)
                    .unwrap_or(0);
                self.transform_state.select(Some(i));
            }
        }
    }

    pub fn previous(&mut self) {
        match self.panel {
            Panel::Monitors => {
                let len = self.monitors.len();
                if len == 0 {
                    return;
                }
                let i = self
                    .monitor_state
                    .selected()
                    .map(|i| if i == 0 { len - 1 } else { i - 1 })
                    .unwrap_or(0);
                self.monitor_state.select(Some(i));
                self.mode_state.select(Some(0));
                self.sync_scale_and_transform();
            }
            Panel::Modes => {
                let len =
                    self.selected_monitor().map(|m| m.modes.len()).unwrap_or(0);
                if len == 0 {
                    return;
                }
                let i = self
                    .mode_state
                    .selected()
                    .map(|i| if i == 0 { len - 1 } else { i - 1 })
                    .unwrap_or(0);
                self.mode_state.select(Some(i));
            }
            Panel::Scale => {
                self.scale_down();
            }
            Panel::Transform => {
                let len = TRANSFORMS.len();
                let i = self
                    .transform_state
                    .selected()
                    .map(|i| if i == 0 { len - 1 } else { i - 1 })
                    .unwrap_or(0);
                self.transform_state.select(Some(i));
            }
        }
    }

    pub fn toggle_panel(&mut self) {
        self.panel = match self.panel {
            Panel::Monitors => Panel::Modes,
            Panel::Modes => Panel::Scale,
            Panel::Scale => Panel::Transform,
            Panel::Transform => Panel::Monitors,
        };
    }

    pub fn toggle_monitor(&mut self) {
        if let Some(monitor) = self.selected_monitor() {
            let _ = self.controller.send(WlMonitorAction::Toggle {
                name: monitor.name.clone(),
                mode: None,
            });
            self.needs_save = true;
        }
    }

    pub fn scale_up(&mut self) {
        self.pending_scale = (self.pending_scale + 0.25).min(10.0);
    }

    pub fn scale_down(&mut self) {
        self.pending_scale = (self.pending_scale - 0.25).max(0.5);
    }

    pub fn apply_action(&mut self) {
        match self.panel {
            Panel::Modes => self.apply_mode(),
            Panel::Scale => self.apply_scale(),
            Panel::Transform => self.apply_transform(),
            _ => return,
        }
        self.needs_save = true;
    }

    fn apply_mode(&self) {
        let Some(monitor) = self.selected_monitor() else {
            return;
        };
        let Some(mode_idx) = self.mode_state.selected() else {
            return;
        };
        let Some(mode) = monitor.modes.get(mode_idx) else {
            return;
        };
        let _ = self.controller.send(WlMonitorAction::SwitchMode {
            name: monitor.name.clone(),
            width: mode.resolution.width,
            height: mode.resolution.height,
            refresh_rate: mode.refresh_rate,
        });
    }

    fn apply_scale(&self) {
        let Some(monitor) = self.selected_monitor() else {
            return;
        };
        let _ = self.controller.send(WlMonitorAction::SetScale {
            name: monitor.name.clone(),
            scale: self.pending_scale,
        });
    }

    fn apply_transform(&self) {
        let Some(monitor) = self.selected_monitor() else {
            return;
        };
        let Some(idx) = self.transform_state.selected() else {
            return;
        };
        let Some(&transform) = TRANSFORMS.get(idx) else {
            return;
        };
        let _ = self.controller.send(WlMonitorAction::SetTransform {
            name: monitor.name.clone(),
            transform,
        });
    }
}
