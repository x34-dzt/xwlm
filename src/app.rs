use std::sync::mpsc::SyncSender;

use ratatui::widgets::ListState;
use wlx_monitors::{WlMonitor, WlMonitorAction};

#[derive(PartialEq)]
pub enum Panel {
    Monitors,
    Modes,
}

pub struct App {
    pub monitors: Vec<WlMonitor>,
    pub monitor_state: ListState,
    pub mode_state: ListState,
    pub panel: Panel,
    pub controller: SyncSender<WlMonitorAction>,
}

impl App {
    pub fn new(controller: SyncSender<WlMonitorAction>) -> Self {
        Self {
            monitors: Vec::new(),
            monitor_state: ListState::default(),
            mode_state: ListState::default(),
            panel: Panel::Monitors,
            controller,
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
        }
    }

    pub fn update_monitor(&mut self, monitor: WlMonitor) {
        if let Some(existing) =
            self.monitors.iter_mut().find(|m| m.name == monitor.name)
        {
            *existing = monitor;
        }
    }

    pub fn remove_monitor(&mut self, name: &str) {
        self.monitors.retain(|m| m.name != name);
        if let Some(selected) = self.monitor_state.selected()
            && selected >= self.monitors.len()
        {
            self.monitor_state
                .select(Some(self.monitors.len().saturating_sub(1)));
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
        }
    }

    pub fn toggle_panel(&mut self) {
        self.panel = match self.panel {
            Panel::Monitors => Panel::Modes,
            Panel::Modes => Panel::Monitors,
        };
    }

    pub fn toggle_monitor(&self) {
        if let Some(monitor) = self.selected_monitor() {
            let _ = self.controller.send(WlMonitorAction::Toggle {
                name: monitor.name.clone(),
            });
        }
    }

    pub fn apply_mode(&self) {
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
}
