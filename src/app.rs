use std::collections::HashMap;
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

#[derive(Clone, Debug)]
pub enum PositionDirection {
    Left,
    Right,
    Up,
    Down,
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

#[derive(PartialEq)]
pub enum Panel {
    Modes,
    Map,
    Scale,
    Transform,
}

/// Minimum step when very close to a neighbor.
const MIN_STEP: i32 = 1;
/// Distance threshold below which we slow down to MIN_STEP.
const SLOW_THRESHOLD: i32 = 10;

pub struct App {
    pub monitors: Vec<WlMonitor>,
    pub selected_monitor: usize,
    pub mode_state: ListState,
    pub transform_state: ListState,
    pub pending_scale: f64,
    pub pending_positions: HashMap<usize, (i32, i32)>,
    pub map_zoom: f64,
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
            selected_monitor: 0,
            mode_state: ListState::default(),
            transform_state: ListState::default().with_selected(Some(0)),
            pending_scale: 1.0,
            pending_positions: HashMap::new(),
            map_zoom: 1.0,
            panel: Panel::Map,
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
        self.monitors.get(self.selected_monitor)
    }

    pub fn set_monitors(&mut self, monitors: Vec<WlMonitor>) {
        self.monitors = monitors;
        if !self.monitors.is_empty() {
            self.selected_monitor = 0;
            self.mode_state.select(Some(0));
            self.sync_panel_state();
        }
    }

    pub fn update_monitor(&mut self, monitor: WlMonitor) {
        if let Some(existing) =
            self.monitors.iter_mut().find(|m| m.name == monitor.name)
        {
            *existing = monitor;
        }
        self.sync_panel_state();
    }

    pub fn remove_monitor(&mut self, name: &str) {
        self.monitors.retain(|m| m.name != name || !m.enabled);
        if self.selected_monitor >= self.monitors.len() {
            self.selected_monitor = self.monitors.len().saturating_sub(1);
        }
        self.sync_panel_state();
    }

    fn sync_panel_state(&mut self) {
        let Some(monitor) = self.monitors.get(self.selected_monitor) else {
            return;
        };
        self.pending_scale = monitor.scale;
        if let Some(tidx) =
            TRANSFORMS.iter().position(|&x| x == monitor.transform)
        {
            self.transform_state.select(Some(tidx));
        }
        if let Some(mode_idx) = monitor.modes.iter().position(|m| m.is_current)
        {
            self.mode_state.select(Some(mode_idx));
        } else {
            self.mode_state.select(Some(0));
        }
    }

    pub fn select_next_monitor(&mut self) {
        if self.monitors.is_empty() {
            return;
        }
        self.selected_monitor =
            (self.selected_monitor + 1) % self.monitors.len();
        self.mode_state.select(Some(0));
        self.sync_panel_state();
    }

    pub fn select_prev_monitor(&mut self) {
        if self.monitors.is_empty() {
            return;
        }
        self.selected_monitor = if self.selected_monitor == 0 {
            self.monitors.len() - 1
        } else {
            self.selected_monitor - 1
        };
        self.mode_state.select(Some(0));
        self.sync_panel_state();
    }

    pub fn next(&mut self) {
        match self.panel {
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
            Panel::Map => {
                self.move_monitor(PositionDirection::Down);
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
            Panel::Map => {
                self.move_monitor(PositionDirection::Up);
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

    pub fn nav_left(&mut self) {
        match self.panel {
            Panel::Map => self.move_monitor(PositionDirection::Left),
            Panel::Scale => self.scale_down(),
            _ => {}
        }
    }

    pub fn nav_right(&mut self) {
        match self.panel {
            Panel::Map => self.move_monitor(PositionDirection::Right),
            Panel::Scale => self.scale_up(),
            _ => {}
        }
    }

    pub fn toggle_panel(&mut self) {
        self.panel = match self.panel {
            Panel::Map => Panel::Modes,
            Panel::Modes => Panel::Scale,
            Panel::Scale => Panel::Transform,
            Panel::Transform => Panel::Map,
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

    pub fn zoom_in(&mut self) {
        self.map_zoom = (self.map_zoom + 0.1).min(5.0);
    }

    pub fn zoom_out(&mut self) {
        self.map_zoom = (self.map_zoom - 0.1).max(0.2);
    }

    pub fn move_monitor(&mut self, direction: PositionDirection) {
        let Some(selected) = self.monitors.get(self.selected_monitor) else {
            return;
        };
        if !selected.enabled {
            return;
        }

        let (cur_x, cur_y) = self.display_position(self.selected_monitor);
        let (sel_w, sel_h) = effective_dimensions(selected);

        // Find nearest neighbor in the movement direction and calculate distance
        let mut nearest_dist: Option<i32> = None;
        let mut _nearest_idx: Option<usize> = None;

        for (i, m) in self.monitors.iter().enumerate() {
            if i == self.selected_monitor || !m.enabled {
                continue;
            }
            let (mx, my) = self.display_position(i);
            let (mw, mh) = effective_dimensions(m);

            // Edge-to-edge distance in the movement direction
            let dist = match direction {
                PositionDirection::Left => {
                    // Only consider monitors whose right edge is to our left
                    let right_edge = mx + mw;
                    if right_edge <= cur_x {
                        // Check vertical overlap (they share some y range)
                        if cur_y < my + mh && cur_y + sel_h > my {
                            Some(cur_x - right_edge)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                PositionDirection::Right => {
                    let left_edge = mx;
                    if left_edge >= cur_x + sel_w {
                        if cur_y < my + mh && cur_y + sel_h > my {
                            Some(left_edge - (cur_x + sel_w))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                PositionDirection::Up => {
                    let bottom_edge = my + mh;
                    if bottom_edge <= cur_y {
                        if cur_x < mx + mw && cur_x + sel_w > mx {
                            Some(cur_y - bottom_edge)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                PositionDirection::Down => {
                    let top_edge = my;
                    if top_edge >= cur_y + sel_h {
                        if cur_x < mx + mw && cur_x + sel_w > mx {
                            Some(top_edge - (cur_y + sel_h))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            };

            if let Some(d) = dist {
                if nearest_dist.is_none() || d < nearest_dist.unwrap() {
                    nearest_dist = Some(d);
                    _nearest_idx = Some(i);
                }
            }
        }

        // Dynamic velocity: fast when far, slow when close
        let step = match nearest_dist {
            Some(d) if d <= SLOW_THRESHOLD => MIN_STEP,
            Some(d) => {
                // Scale step: 1/10th of distance, clamped between MIN_STEP and distance
                (d / 10).max(MIN_STEP).min(d)
            }
            None => {
                // No neighbor in this direction — use a moderate step
                50
            }
        };

        let (new_x, new_y) = match direction {
            PositionDirection::Left => (cur_x - step, cur_y),
            PositionDirection::Right => (cur_x + step, cur_y),
            PositionDirection::Up => (cur_x, cur_y - step),
            PositionDirection::Down => (cur_x, cur_y + step),
        };

        // AABB collision check at the new position
        let collided = self.monitors.iter().enumerate().find(|(i, m)| {
            if *i == self.selected_monitor || !m.enabled {
                return false;
            }
            let (mx, my) = self.display_position(*i);
            let (mw, mh) = effective_dimensions(m);
            new_x < mx + mw
                && new_x + sel_w > mx
                && new_y < my + mh
                && new_y + sel_h > my
        });

        if let Some((other_idx, other_mon)) = collided {
            // Swap: place edge-to-edge based on direction
            let (other_x, other_y) = self.display_position(other_idx);
            let (other_w, other_h) = effective_dimensions(other_mon);

            let (sel_new, other_new) = match direction {
                PositionDirection::Left => {
                    ((other_x, other_y), (other_x + sel_w, other_y))
                }
                PositionDirection::Right => {
                    ((cur_x + other_w, cur_y), (cur_x, cur_y))
                }
                PositionDirection::Up => {
                    ((other_x, other_y), (other_x, other_y + sel_h))
                }
                PositionDirection::Down => {
                    ((cur_x, cur_y + other_h), (cur_x, cur_y))
                }
            };

            self.pending_positions
                .insert(self.selected_monitor, sel_new);
            self.pending_positions.insert(other_idx, other_new);
        } else {
            self.pending_positions
                .insert(self.selected_monitor, (new_x, new_y));
        }
    }

    /// Get the display position for a monitor (pending if moved, otherwise actual).
    pub fn display_position(&self, idx: usize) -> (i32, i32) {
        if let Some(&pos) = self.pending_positions.get(&idx) {
            return pos;
        }
        self.monitors
            .get(idx)
            .map(|m| (m.position.x, m.position.y))
            .unwrap_or((0, 0))
    }

    pub fn has_pending_positions(&self) -> bool {
        !self.pending_positions.is_empty()
    }

    fn apply_positions(&self) {
        for (&idx, &(x, y)) in &self.pending_positions {
            if let Some(monitor) = self.monitors.get(idx) {
                let _ = self.controller.send(WlMonitorAction::SetPosition {
                    name: monitor.name.clone(),
                    x,
                    y,
                });
            }
        }
    }

    pub fn apply_action(&mut self) {
        match self.panel {
            Panel::Modes => self.apply_mode(),
            Panel::Scale => self.apply_scale(),
            Panel::Transform => self.apply_transform(),
            Panel::Map => {
                if self.pending_positions.is_empty() {
                    return;
                }
                self.apply_positions();
                self.pending_positions.clear();
            }
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
