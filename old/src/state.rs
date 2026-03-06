use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc::{SendError, SyncSender},
    time::Instant,
};

use ratatui::widgets::ListState;
use wlx_monitors::{WlMonitor, WlMonitorAction};

use crate::{
    compositor::{
        self,
        format::{reload, save_monitor_config},
        position::get_position,
        workspace_config::{WorkspaceRule, parse_workspace_config},
    },
    constants::{REPEAT_WINDOW_MS, TRANSFORMS},
    utils::effective_dimensions,
};

#[derive(Debug, PartialEq)]
pub enum Panel {
    Monitor,
    Mode,
    Workspace,
    Scale,
    Transform,
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
    pub is_default: bool,
    pub is_persistent: bool,
}

#[derive(Debug)]
pub struct App {
    pub monitors: Vec<WlMonitor>,
    pub selected_monitor: usize,
    pub panel: Panel,
    pub compositor: compositor::Compositor,
    pub wlx_action_handler: SyncSender<WlMonitorAction>,
    pub workspace_assignments: Vec<WorkspaceAssignment>,
    pub comp_monitor_config_path: PathBuf,
    pub needs_save: bool,

    pub pending_positions: HashMap<usize, (i32, i32)>,
    pub pending_workspaces: HashMap<usize, WorkspaceAssignment>,
    pub pending_scale: f64,
    pub map_zoom: f64,
    pub transform_state: ListState,
    pub mode_state: ListState,
    pub workspace_state: ListState,
    pub pending_last_toggle_monitor: bool,
    pub error_message: Option<String>,

    last_move_time: Instant,
    move_repeat_count: u32,
    last_move_direction: Option<PositionDirection>,
    initial_workspaces: Option<Vec<WorkspaceRule>>,
}

impl App {
    pub fn new(
        wlx_action_handler: SyncSender<WlMonitorAction>,
        comp_monitor_config_path: PathBuf,
        comp_workspace_count: usize,
    ) -> Self {
        let comp = compositor::detect();
        let initial_workspaces = Some(parse_workspace_config(comp, &comp_monitor_config_path));

        let workspace_assignments = (1..=comp_workspace_count)
            .map(|id| WorkspaceAssignment {
                id,
                monitor_idx: None,
                is_default: false,
                is_persistent: false,
            })
            .collect();

        Self {
            monitors: Vec::new(),
            selected_monitor: 0,
            panel: Panel::Monitor,
            compositor: comp,
            wlx_action_handler,
            needs_save: false,
            pending_positions: HashMap::new(),
            pending_workspaces: HashMap::new(),
            workspace_assignments,
            workspace_state: ListState::default().with_selected(Some(0)),
            map_zoom: 1.0,
            pending_scale: 1.0,
            transform_state: ListState::default().with_selected(Some(0)),
            mode_state: ListState::default().with_selected(Some(0)),
            pending_last_toggle_monitor: false,
            error_message: None,
            comp_monitor_config_path,
            last_move_time: Instant::now(),
            last_move_direction: None,
            move_repeat_count: 0,
            initial_workspaces,
        }
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

    pub fn update_monitor(&mut self, monitor: WlMonitor) {
        if let Some(existing_monitor) = self.monitors.iter_mut().find(|m| m.name == monitor.name) {
            *existing_monitor = monitor;
        } else {
            self.monitors.push(monitor);
            self.sanitize_selection();
        };
    }

    pub fn remove_monitor(&mut self, name: &str) {
        let removed_idx = self.monitors.iter().position(|m| m.name == name);
        self.monitors.retain(|m| m.name != name);

        if let Some(idx) = removed_idx {
            self.pending_positions.remove(&idx);
            for key in self.pending_positions.keys().copied().collect::<Vec<_>>() {
                if key > idx
                    && let Some(pos) = self.pending_positions.remove(&key)
                {
                    self.pending_positions.insert(key - 1, pos);
                }
            }

            if self.selected_monitor >= self.monitors.len() {
                self.selected_monitor = self.monitors.len().saturating_sub(1);
            }
            self.sync_panel_state();
        }
    }

    fn sanitize_selection(&mut self) {
        if self.monitors.is_empty() {
            self.selected_monitor = 0;
        } else if self.selected_monitor >= self.monitors.len() {
            self.selected_monitor = self.monitors.len() - 1;
        }
    }

    pub fn selected_monitor(&self) -> Option<&WlMonitor> {
        self.monitors.get(self.selected_monitor)
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

    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error_message = Some(msg.into());
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    pub fn zoom_in(&mut self) {
        self.map_zoom = (self.map_zoom + 0.1).min(5.0);
    }

    pub fn zoom_out(&mut self) {
        self.map_zoom = (self.map_zoom - 0.1).max(0.2);
    }

    pub fn scale_up(&mut self) {
        self.pending_scale = (self.pending_scale + 0.01).min(10.0);
    }

    pub fn scale_down(&mut self) {
        self.pending_scale = (self.pending_scale - 0.01).max(0.5);
    }

    fn enabled_count(&self) -> usize {
        self.monitors.iter().filter(|m| m.enabled).count()
    }

    pub fn dismiss_warning(&mut self) {
        self.pending_last_toggle_monitor = false;
    }

    pub fn toggle_monitor(&mut self) -> Result<(), SendError<WlMonitorAction>> {
        if self.pending_last_toggle_monitor {
            self.pending_last_toggle_monitor = false;
            let Some(monitor) = self.monitors.get(self.selected_monitor) else {
                return Ok(());
            };
            self.perform_toggle(&monitor.name.clone(), monitor.enabled)?;
            return Ok(());
        }

        let Some(monitor) = self.monitors.get(self.selected_monitor) else {
            return Ok(());
        };

        if monitor.enabled && self.enabled_count() == 1 {
            self.pending_last_toggle_monitor = true;
            return Ok(());
        }
        self.perform_toggle(&monitor.name.clone(), monitor.enabled)?;

        Ok(())
    }

    fn perform_toggle(
        &mut self,
        monitor_name: &str,
        currently_enabled: bool,
    ) -> Result<(), SendError<WlMonitorAction>> {
        let will_enable = !currently_enabled;
        let position = if will_enable {
            let saved_pos = get_position(
                self.compositor,
                &self.comp_monitor_config_path,
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
                    Some(self.calculate_closest_non_overlapping_position(monitor_name, pos, (w, h)))
                } else {
                    Some(pos)
                }
            } else {
                Some(self.calculate_non_overlapping_position(monitor_name))
            }
        } else {
            None
        };

        self.wlx_action_handler.send(WlMonitorAction::Toggle {
            name: monitor_name.to_string(),
            mode: None,
            position,
        })?;

        self.needs_save = true;

        Ok(())
    }

    fn position_overlaps(&self, exclude_name: &str, pos: (i32, i32), size: (i32, i32)) -> bool {
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
                let dist = (pos.0 - preferred_pos.0).abs() + (pos.1 - preferred_pos.1).abs();
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
            .map(|d| std::mem::discriminant(d) == std::mem::discriminant(&direction))
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
            new_x < mx + mw && new_x + sel_w > mx && new_y < my + mh && new_y + sel_h > my
        });

        if let Some((other_idx, other_mon)) = collided {
            let (other_x, other_y) = self.display_position(other_idx);
            let (other_w, other_h) = effective_dimensions(other_mon);

            let (new_pos_selected, new_pos_other) = match direction {
                PositionDirection::Left => ((other_x, other_y), (other_x + sel_w, other_y)),
                PositionDirection::Right => ((cur_x + other_w, cur_y), (cur_x, cur_y)),
                PositionDirection::Up => ((other_x, other_y), (other_x, other_y + sel_h)),
                PositionDirection::Down => ((cur_x, cur_y + other_h), (cur_x, cur_y)),
            };

            let new_pos_selected = (new_pos_selected.0.max(0), new_pos_selected.1.max(0));
            let new_pos_other = (new_pos_other.0.max(0), new_pos_other.1.max(0));

            self.pending_positions
                .insert(self.selected_monitor, new_pos_selected);
            self.pending_positions.insert(other_idx, new_pos_other);
        } else {
            self.pending_positions
                .insert(self.selected_monitor, (new_x, new_y));
        }
    }

    pub fn previous(&mut self) {
        match self.panel {
            Panel::Mode => {
                let len = self.selected_monitor().map(|m| m.modes.len()).unwrap_or(0);
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
            Panel::Monitor => {
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
            Panel::Workspace => {
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

    pub fn next(&mut self) {
        match self.panel {
            Panel::Mode => {
                let len = self.selected_monitor().map(|m| m.modes.len()).unwrap_or(0);
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
            Panel::Monitor => {
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
            Panel::Workspace => {
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

    pub fn cycle_workspace_monitor(&mut self, forward: bool) {
        let Some(ws_idx) = self.workspace_state.selected() else {
            return;
        };

        let Some(effective) = self.get_effective_workspace(ws_idx) else {
            return;
        };

        let monitors: Vec<usize> = self.monitors.iter().enumerate().map(|(i, _)| i).collect();

        if monitors.is_empty() {
            return;
        }

        let new_monitor_idx = match effective.monitor_idx {
            None => {
                if forward {
                    Some(monitors[0])
                } else {
                    Some(monitors[monitors.len() - 1])
                }
            }
            Some(idx) => {
                let pos = monitors.iter().position(|&i| i == idx);
                match pos {
                    Some(p) => {
                        if forward {
                            if p + 1 >= monitors.len() {
                                None
                            } else {
                                Some(monitors[p + 1])
                            }
                        } else if p == 0 {
                            None
                        } else {
                            Some(monitors[p - 1])
                        }
                    }
                    None => {
                        if forward {
                            Some(monitors[0])
                        } else {
                            Some(monitors[monitors.len() - 1])
                        }
                    }
                }
            }
        };

        let mut new_ws = effective;
        new_ws.monitor_idx = new_monitor_idx;
        self.pending_workspaces.insert(ws_idx, new_ws);
    }

    pub fn get_effective_workspace(&self, idx: usize) -> Option<WorkspaceAssignment> {
        if let Some(ws) = self.pending_workspaces.get(&idx) {
            return Some(ws.clone());
        }
        self.workspace_assignments.get(idx).cloned()
    }

    pub fn has_pending_workspaces(&self) -> bool {
        !self.pending_workspaces.is_empty()
    }

    pub fn nav_left(&mut self) {
        match self.panel {
            Panel::Monitor => self.move_monitor(PositionDirection::Left),
            Panel::Scale => self.scale_down(),
            Panel::Workspace => self.cycle_workspace_monitor(false),
            _ => {}
        }
    }

    pub fn nav_right(&mut self) {
        match self.panel {
            Panel::Monitor => self.move_monitor(PositionDirection::Right),
            Panel::Scale => self.scale_up(),
            Panel::Workspace => self.cycle_workspace_monitor(true),
            _ => {}
        }
    }

    pub fn toggle_panel(&mut self) {
        self.panel = match self.panel {
            Panel::Monitor => Panel::Mode,
            Panel::Mode => Panel::Workspace,
            Panel::Workspace => Panel::Scale,
            Panel::Scale => Panel::Transform,
            Panel::Transform => Panel::Monitor,
        };
    }

    pub fn save_config(&mut self) {
        if !self.needs_save {
            return;
        }
        self.needs_save = false;

        let workspace_rules: Vec<WorkspaceRule> = self
            .workspace_assignments
            .iter()
            .map(|ws| {
                let monitor_name = ws
                    .monitor_idx
                    .and_then(|idx| self.monitors.get(idx))
                    .map(|m| m.name.clone())
                    .unwrap_or_default();
                WorkspaceRule {
                    id: ws.id,
                    monitor: monitor_name,
                    is_default: ws.is_default,
                    is_persistent: ws.is_persistent,
                }
            })
            .collect();

        if let Err(e) = save_monitor_config(
            self.compositor,
            &self.comp_monitor_config_path,
            &self.monitors,
            &workspace_rules,
        ) {
            self.set_error(format!("Failed to save config: {e}"));
        } else {
            reload(self.compositor);
        }
    }

    pub fn reset_positions(&mut self) {
        self.pending_positions.clear();
        self.pending_workspaces.clear();
    }

    pub fn select_next_monitor(&mut self) {
        if self.monitors.is_empty() {
            return;
        }
        self.selected_monitor = (self.selected_monitor + 1) % self.monitors.len();
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

    fn sync_panel_state(&mut self) {
        let Some(monitor) = self.monitors.get(self.selected_monitor) else {
            return;
        };
        self.pending_scale = monitor.scale;
        if let Some(tidx) = TRANSFORMS.iter().position(|&x| x == monitor.transform) {
            self.transform_state.select(Some(tidx));
        }
        if let Some(mode_idx) = monitor.modes.iter().position(|m| m.is_current) {
            self.mode_state.select(Some(mode_idx));
        } else {
            self.mode_state.select(Some(0));
        }
    }

    pub fn toggle_persistent(&mut self) {
        let Some(ws_idx) = self.workspace_state.selected() else {
            return;
        };

        let Some(mut effective) = self.get_effective_workspace(ws_idx) else {
            return;
        };
        effective.is_persistent = !effective.is_persistent;
        self.pending_workspaces.insert(ws_idx, effective);
    }

    pub fn toggle_default(&mut self) {
        let Some(ws_idx) = self.workspace_state.selected() else {
            return;
        };

        let Some(effective) = self.get_effective_workspace(ws_idx) else {
            return;
        };

        let new_default_monitor_idx = if effective.is_default { None } else { effective.monitor_idx };

        let Some(mut effective) = self.get_effective_workspace(ws_idx) else {
            return;
        };
        effective.is_default = new_default_monitor_idx.is_some();

        if let Some(target_monitor) = new_default_monitor_idx {
            for (_, w) in self.pending_workspaces.iter_mut() {
                if w.is_default && w.monitor_idx == Some(target_monitor) {
                    w.is_default = false;
                }
            }
            for w in self.workspace_assignments.iter_mut() {
                if w.is_default && w.monitor_idx == Some(target_monitor) {
                    w.is_default = false;
                }
            }
        }

        self.pending_workspaces.insert(ws_idx, effective);
    }

    pub fn apply_action(&mut self) -> Result<(), SendError<WlMonitorAction>> {
        match self.panel {
            Panel::Mode => self.apply_mode()?,
            Panel::Scale => self.apply_scale()?,
            Panel::Transform => self.apply_transform()?,
            Panel::Monitor => {
                if self.pending_positions.is_empty() {
                    return Ok(());
                }
                for (&idx, &(x, y)) in &self.pending_positions {
                    if let Some(monitor) = self.monitors.get_mut(idx) {
                        monitor.position.x = x;
                        monitor.position.y = y;
                    }
                }
                self.apply_positions()?;
                self.pending_positions.clear();
            }
            Panel::Workspace => {
                if self.pending_workspaces.is_empty() {
                    return Ok(());
                }
                for (&idx, ws) in &self.pending_workspaces {
                    if let Some(existing) = self.workspace_assignments.get_mut(idx) {
                        existing.monitor_idx = ws.monitor_idx;
                        existing.is_default = ws.is_default;
                        existing.is_persistent = ws.is_persistent;
                    }
                }
                self.pending_workspaces.clear();
            }
        }
        self.needs_save = true;
        self.save_config();

        Ok(())
    }

    fn apply_mode(&self) -> Result<(), SendError<WlMonitorAction>> {
        let Some(monitor) = self.selected_monitor() else {
            return Ok(());
        };
        let Some(mode_idx) = self.mode_state.selected() else {
            return Ok(());
        };
        let Some(mode) = monitor.modes.get(mode_idx) else {
            return Ok(());
        };

        self.wlx_action_handler.send(WlMonitorAction::SwitchMode {
            name: monitor.name.clone(),
            width: mode.resolution.width,
            height: mode.resolution.height,
            refresh_rate: mode.refresh_rate,
        })?;

        Ok(())
    }

    fn apply_scale(&self) -> Result<(), SendError<WlMonitorAction>> {
        let Some(monitor) = self.selected_monitor() else {
            return Ok(());
        };
        self.wlx_action_handler.send(WlMonitorAction::SetScale {
            name: monitor.name.clone(),
            scale: self.pending_scale,
        })?;
        Ok(())
    }

    fn apply_transform(&self) -> Result<(), SendError<WlMonitorAction>> {
        let Some(monitor) = self.selected_monitor() else {
            return Ok(());
        };
        let Some(idx) = self.transform_state.selected() else {
            return Ok(());
        };
        let Some(&transform) = TRANSFORMS.get(idx) else {
            return Ok(());
        };

        self.wlx_action_handler
            .send(WlMonitorAction::SetTransform {
                name: monitor.name.clone(),
                transform,
            })?;

        Ok(())
    }

    fn apply_positions(&self) -> Result<(), SendError<WlMonitorAction>> {
        for (&idx, &(x, y)) in &self.pending_positions {
            if let Some(monitor) = self.monitors.get(idx) {
                self.wlx_action_handler.send(WlMonitorAction::SetPosition {
                    name: monitor.name.clone(),
                    x,
                    y,
                })?
            }
        }

        Ok(())
    }

    fn resolve_initial_workspaces(&mut self) {
        let Some(workspace_rules) = self.initial_workspaces.take() else {
            return;
        };
        for rule in &workspace_rules {
            let monitor_idx = self.monitors.iter().position(|m| m.name == rule.monitor);
            if let Some(ws) = self
                .workspace_assignments
                .iter_mut()
                .find(|ws| ws.id == rule.id)
            {
                ws.monitor_idx = monitor_idx;
                ws.is_default = rule.is_default;
                ws.is_persistent = rule.is_persistent;
            }
        }
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
