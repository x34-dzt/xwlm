use std::collections::HashMap;
use std::sync::mpsc::SyncSender;
use std::time::Instant;

use ratatui::widgets::ListState;
use wlx_monitors::{WlMonitor, WlMonitorAction, WlTransform};

use xwlm_cfg::Compositor;

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

#[derive(Clone, Debug)]
pub struct WorkspaceAssignment {
    pub id: usize,
    pub monitor_idx: Option<usize>,
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
    Workspaces,
}

const REPEAT_WINDOW_MS: u128 = 200;

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
    pub pending_toggle_warning: bool,
    pub workspace_assignments: Vec<WorkspaceAssignment>,
    pub workspace_state: ListState,
    initial_workspace_names: Option<Vec<(usize, String)>>,
    last_move_time: Instant,
    last_move_direction: Option<PositionDirection>,
    move_repeat_count: u32,
}

impl App {
    pub fn new(
        controller: SyncSender<WlMonitorAction>,
        compositor: Compositor,
        monitor_config_path: String,
        workspace_count: usize,
    ) -> Self {
        let initial_workspace_names =
            Some(xwlm_cfg::workspace::parse_workspace_config(
                compositor,
                &monitor_config_path,
            ));

        let workspace_assignments = (1..=workspace_count)
            .map(|id| WorkspaceAssignment {
                id,
                monitor_idx: None,
            })
            .collect();

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
            pending_toggle_warning: false,
            workspace_assignments,
            workspace_state: ListState::default().with_selected(Some(0)),
            initial_workspace_names,
            last_move_time: Instant::now(),
            last_move_direction: None,
            move_repeat_count: 0,
        }
    }

    pub fn save_config(&mut self) {
        if !self.needs_save || self.monitor_config_path.is_empty() {
            return;
        }
        self.needs_save = false;
        let workspaces: Vec<(usize, Option<String>)> = self
            .workspace_assignments
            .iter()
            .map(|ws| {
                let name = ws
                    .monitor_idx
                    .and_then(|idx| self.monitors.get(idx))
                    .map(|m| m.name.clone());
                (ws.id, name)
            })
            .collect();
        if let Err(e) = xwlm_cfg::format::save_monitor_config(
            self.compositor,
            &self.monitor_config_path,
            &self.monitors,
            &workspaces,
        ) {
            eprintln!("Failed to save monitor config: {e}");
        } else {
            xwlm_cfg::format::reload(self.compositor);
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
        self.resolve_initial_workspaces();
        self.validate_workspace_assignments();
    }

    fn resolve_initial_workspaces(&mut self) {
        let Some(names) = self.initial_workspace_names.take() else {
            return;
        };
        for (ws_id, monitor_name) in &names {
            let monitor_idx =
                self.monitors.iter().position(|m| m.name == *monitor_name);
            if let Some(ws) = self
                .workspace_assignments
                .iter_mut()
                .find(|ws| ws.id == *ws_id)
            {
                ws.monitor_idx = monitor_idx;
            }
        }
    }

    pub fn update_monitor(&mut self, monitor: WlMonitor) {
        if let Some(existing) =
            self.monitors.iter_mut().find(|m| m.name == monitor.name)
        {
            *existing = monitor;
        } else {
            self.monitors.push(monitor);
        }
        self.sync_panel_state();
        self.validate_workspace_assignments();
    }

    pub fn remove_monitor(&mut self, name: &str) {
        self.monitors.retain(|m| m.name != name || !m.enabled);
        if self.selected_monitor >= self.monitors.len() {
            self.selected_monitor = self.monitors.len().saturating_sub(1);
        }
        self.sync_panel_state();
        self.validate_workspace_assignments();
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
            Panel::Workspaces => {
                let len = self.workspace_assignments.len();
                if len == 0 {
                    return;
                }
                let i = self
                    .workspace_state
                    .selected()
                    .map(|i| (i + 1) % len)
                    .unwrap_or(0);
                self.workspace_state.select(Some(i));
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
            Panel::Workspaces => {
                let len = self.workspace_assignments.len();
                if len == 0 {
                    return;
                }
                let i = self
                    .workspace_state
                    .selected()
                    .map(|i| if i == 0 { len - 1 } else { i - 1 })
                    .unwrap_or(0);
                self.workspace_state.select(Some(i));
            }
        }
    }

    pub fn nav_left(&mut self) {
        match self.panel {
            Panel::Map => self.move_monitor(PositionDirection::Left),
            Panel::Scale => self.scale_down(),
            Panel::Workspaces => self.cycle_workspace_monitor(false),
            _ => {}
        }
    }

    pub fn nav_right(&mut self) {
        match self.panel {
            Panel::Map => self.move_monitor(PositionDirection::Right),
            Panel::Scale => self.scale_up(),
            Panel::Workspaces => self.cycle_workspace_monitor(true),
            _ => {}
        }
    }

    pub fn toggle_panel(&mut self) {
        self.panel = match self.panel {
            Panel::Map => Panel::Modes,
            Panel::Modes => Panel::Workspaces,
            Panel::Workspaces => Panel::Scale,
            Panel::Scale => Panel::Transform,
            Panel::Transform => Panel::Map,
        };
    }

    pub fn toggle_monitor(&mut self) {
        if self.pending_toggle_warning {
            self.pending_toggle_warning = false;
            let Some(monitor) = self.monitors.get(self.selected_monitor) else {
                return;
            };
            self.perform_toggle(&monitor.name.clone(), monitor.enabled);
            return;
        }

        let Some(monitor) = self.monitors.get(self.selected_monitor) else {
            return;
        };

        if monitor.enabled && self.enabled_count() == 1 {
            self.pending_toggle_warning = true;
            return;
        }
        self.perform_toggle(&monitor.name.clone(), monitor.enabled);
    }

    fn perform_toggle(&mut self, monitor_name: &str, currently_enabled: bool) {
        let will_enable = !currently_enabled;
        let position = if will_enable {
            let saved_pos = xwlm_cfg::parse::get_saved_monitor_position(
                self.compositor,
                &self.monitor_config_path,
                monitor_name,
            );
            let (w, h) = self
                .monitors
                .iter()
                .find(|m| m.name == monitor_name)
                .map(effective_dimensions)
                .unwrap_or((1920, 1080));

            if let Some(saved) = saved_pos {
                let pos = (saved.x, saved.y);
                if self.position_overlaps(monitor_name, pos, (w, h)) {
                    Some(self.calculate_closest_non_overlapping_position(
                        monitor_name, pos, (w, h),
                    ))
                } else {
                    Some(pos)
                }
            } else {
                Some(self.calculate_non_overlapping_position(monitor_name))
            }
        } else {
            None
        };

        let _ = self.controller.send(WlMonitorAction::Toggle {
            name: monitor_name.to_string(),
            mode: None,
            position,
        });

        self.needs_save = true;
    }

    fn position_overlaps(
        &self,
        exclude_name: &str,
        pos: (i32, i32),
        size: (i32, i32),
    ) -> bool {
        let (x1, y1) = pos;
        let (w1, h1) = size;

        self.monitors.iter().any(|m| {
            if m.name == exclude_name || !m.enabled {
                return false;
            }
            let (x2, y2) = (m.position.x, m.position.y);
            let (w2, h2) = effective_dimensions(m);

            x1 < x2 + w2 && x1 + w1 > x2 && y1 < y2 + h2 && y1 + h1 > y2
        })
    }

    fn calculate_closest_non_overlapping_position(
        &self,
        exclude_name: &str,
        preferred_pos: (i32, i32),
        size: (i32, i32),
    ) -> (i32, i32) {
        let (w, h) = size;
        let enabled_monitors: Vec<&WlMonitor> = self
            .monitors
            .iter()
            .filter(|m| m.enabled && m.name != exclude_name)
            .collect();

        if enabled_monitors.is_empty() {
            return preferred_pos;
        }

        let mut candidates: Vec<(i32, i32)> = Vec::new();

        let min_left = enabled_monitors
            .iter()
            .map(|m| m.position.x)
            .min()
            .unwrap_or(0);
        candidates.push((min_left - w, 0));

        let max_right = enabled_monitors
            .iter()
            .map(|m| {
                let (mw, _) = effective_dimensions(m);
                m.position.x + mw
            })
            .max()
            .unwrap_or(0);
        candidates.push((max_right, 0));

        let min_top = enabled_monitors
            .iter()
            .map(|m| m.position.y)
            .min()
            .unwrap_or(0);
        candidates.push((0, min_top - h));

        let max_bottom = enabled_monitors
            .iter()
            .map(|m| {
                let (_, mh) = effective_dimensions(m);
                m.position.y + mh
            })
            .max()
            .unwrap_or(0);
        candidates.push((0, max_bottom));

        candidates
            .into_iter()
            .filter(|pos| !self.position_overlaps(exclude_name, *pos, size))
            .map(|pos| {
                let dist = (pos.0 - preferred_pos.0).abs()
                    + (pos.1 - preferred_pos.1).abs();
                (dist, pos)
            })
            .min_by_key(|(d, _)| *d)
            .map(|(_, pos)| pos)
            .unwrap_or((max_right, 0))
    }

    fn calculate_non_overlapping_position(&self, exclude_name: &str) -> (i32, i32) {
        let enabled_monitors: Vec<&WlMonitor> = self
            .monitors
            .iter()
            .filter(|m| m.enabled && m.name != exclude_name)
            .collect();

        if enabled_monitors.is_empty() {
            return (0, 0);
        }

        let max_right = enabled_monitors
            .iter()
            .map(|m| {
                let (w, _) = effective_dimensions(m);
                m.position.x + w
            })
            .max()
            .unwrap_or(0);

        (max_right, 0)
    }

    pub fn dismiss_warning(&mut self) {
        self.pending_toggle_warning = false;
    }

    fn enabled_count(&self) -> usize {
        self.monitors.iter().filter(|m| m.enabled).count()
    }

    pub fn scale_up(&mut self) {
        self.pending_scale = (self.pending_scale + 0.01).min(10.0);
    }

    pub fn scale_down(&mut self) {
        self.pending_scale = (self.pending_scale - 0.01).max(0.5);
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

        let now = Instant::now();
        let elapsed = now.duration_since(self.last_move_time).as_millis();
        let same_direction = self
            .last_move_direction
            .as_ref()
            .map(|d| {
                std::mem::discriminant(d) == std::mem::discriminant(&direction)
            })
            .unwrap_or(false);

        if elapsed < REPEAT_WINDOW_MS && same_direction {
            self.move_repeat_count += 1;
        } else {
            self.move_repeat_count = 0;
        }
        self.last_move_time = now;
        self.last_move_direction = Some(direction.clone());

        let step = 1 + (self.move_repeat_count * 2) as i32;

        let (cur_x, cur_y) = self.display_position(self.selected_monitor);
        let (sel_w, sel_h) = effective_dimensions(selected);

        let (new_x, new_y) = match direction {
            PositionDirection::Left => (cur_x - step, cur_y),
            PositionDirection::Right => (cur_x + step, cur_y),
            PositionDirection::Up => (cur_x, cur_y - step),
            PositionDirection::Down => (cur_x, cur_y + step),
        };

        let new_x = new_x.max(0);
        let new_y = new_y.max(0);

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
            let (other_x, other_y) = self.display_position(other_idx);
            let (other_w, other_h) = effective_dimensions(other_mon);

            let (new_pos_selected, new_pos_other) = match direction {
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

            let new_pos_selected =
                (new_pos_selected.0.max(0), new_pos_selected.1.max(0));
            let new_pos_other =
                (new_pos_other.0.max(0), new_pos_other.1.max(0));

            self.pending_positions
                .insert(self.selected_monitor, new_pos_selected);
            self.pending_positions.insert(other_idx, new_pos_other);
        } else {
            self.pending_positions
                .insert(self.selected_monitor, (new_x, new_y));
        }
    }

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
            Panel::Workspaces => {
                self.cycle_workspace_monitor(true);
                return;
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

    pub fn reset_positions(&mut self) {
        self.pending_positions.clear();
    }

    pub fn cycle_workspace_monitor(&mut self, forward: bool) {
        let Some(ws_idx) = self.workspace_state.selected() else {
            return;
        };
        let Some(ws) = self.workspace_assignments.get_mut(ws_idx) else {
            return;
        };

        let enabled: Vec<usize> = self
            .monitors
            .iter()
            .enumerate()
            .filter(|(_, m)| m.enabled)
            .map(|(i, _)| i)
            .collect();
        if enabled.is_empty() {
            return;
        }

        ws.monitor_idx = match ws.monitor_idx {
            None => {
                if forward {
                    Some(enabled[0])
                } else {
                    Some(enabled[enabled.len() - 1])
                }
            }
            Some(idx) => {
                let pos = enabled.iter().position(|&i| i == idx);
                match pos {
                    Some(p) => {
                        if forward {
                            if p + 1 >= enabled.len() {
                                None
                            } else {
                                Some(enabled[p + 1])
                            }
                        } else if p == 0 {
                            None
                        } else {
                            Some(enabled[p - 1])
                        }
                    }
                    None => None,
                }
            }
        };
        self.needs_save = true;
        self.save_config();
    }

    fn validate_workspace_assignments(&mut self) {
        let mon_count = self.monitors.len();
        for ws in &mut self.workspace_assignments {
            if let Some(idx) = ws.monitor_idx
                && idx >= mon_count
            {
                ws.monitor_idx = None;
            }
        }
    }
}
