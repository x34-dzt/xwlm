use std::{io, sync::mpsc::Receiver};

use crate::panes::{
    Pane, modes::Modes, monitor_map::MonitorMap, scale::Scale, transform::Transform,
    workspace::Workspace,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
};
use wlx_monitors::{WlMonitor, WlMonitorEvent};

// TODO: USE LIFECYCLES LATER IF POSSIBLE
#[derive(Debug, Default)]
pub struct App {
    monitors: Vec<WlMonitor>,
    selected_monitor: usize,
    active_pane: Pane,
    monitor_map: MonitorMap,
    scale: Scale,
    transform: Transform,
    workspace: Workspace,
    modes: Modes,
    exit: bool,
}

enum AppEvent {
    Key(KeyEvent),
    Monitor(WlMonitorEvent),
}

pub fn spawn_event_loop(wlx_events: Receiver<WlMonitorEvent>) -> Receiver<AppEvent> {
    let (tx, rx) = std::sync::mpsc::channel();

    let txt1 = tx.clone();
    std::thread::spawn(move || {
        loop {
            if let Ok(Event::Key(k)) = event::read() {
                let _ = txt1.send(AppEvent::Key(k));
            }
        }
    });

    let tx2 = tx.clone();
    std::thread::spawn(move || {
        loop {
            if let Ok(event) = wlx_events.recv() {
                let _ = tx2.send(AppEvent::Monitor(event));
            }
        }
    });

    rx
}

impl App {
    pub fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        wlx_events: Receiver<WlMonitorEvent>,
    ) -> io::Result<()> {
        let events = spawn_event_loop(wlx_events);

        while !self.exit {
            while let Ok(event) = events.try_recv() {
                match event {
                    AppEvent::Key(k) => self.handle_key_events(k),
                    AppEvent::Monitor(e) => self.handle_wlx_events(e),
                }
            }

            terminal.draw(|frame| self.draw(frame))?;

            // now block until at least one new event arrives
            match events.recv() {
                Ok(AppEvent::Key(k)) => self.handle_key_events(k),
                Ok(AppEvent::Monitor(e)) => self.handle_wlx_events(e),
                Err(_) => break,
            }
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let root = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(frame.area());

        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(root[0]);

        let bottom_left = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(left[1]);

        let right = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(root[1]);

        self.monitor_map
            .draw(frame, left[0], self.active_pane == Pane::Map);
        self.scale
            .draw(frame, bottom_left[0], self.active_pane == Pane::Scale);
        self.transform
            .draw(frame, bottom_left[1], self.active_pane == Pane::Transform);
        self.modes
            .draw(frame, right[0], self.active_pane == Pane::Mode);
        self.workspace
            .draw(frame, right[1], self.active_pane == Pane::Worksapce);
    }

    pub fn handle_key_events(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Char('q') => self.exit = true,
            KeyCode::Tab => self.cycle_pane(),
            _ => match self.active_pane {
                Pane::Map => self.monitor_map.binds(k),
                _ => {}
            },
        };
    }

    pub fn cycle_pane(&mut self) {
        self.active_pane = self.active_pane.next();
    }

    pub fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_events(key_event)
            }
            _ => {}
        };

        Ok(())
    }

    pub fn handle_wlx_events(&mut self, event: WlMonitorEvent) {
        match event {
            WlMonitorEvent::InitialState(monitors) => {
                self.add_monitors(monitors);
            }
            _ => {}
        }
    }

    pub fn add_monitors(&mut self, monitors: Vec<WlMonitor>) {
        self.monitor_map.set_montiors(monitors.clone());
        self.monitors = monitors;
    }
}
