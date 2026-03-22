// Workspace — each workspace is an independent canvas with its own terminals

use egui::Vec2;
use std::path::PathBuf;
use uuid::Uuid;

use crate::terminal::panel::TerminalPanel;

pub struct Workspace {
    #[allow(dead_code)]
    pub id: Uuid,
    pub name: String,
    pub cwd: Option<PathBuf>,
    pub panels: Vec<TerminalPanel>,
    pub viewport_pan: Vec2,
    pub viewport_zoom: f32,
    pub next_z: u32,
    pub next_color: usize,
}

impl Workspace {
    pub fn new(name: impl Into<String>, cwd: Option<PathBuf>) -> Self {
        let name = name.into();
        Self {
            id: Uuid::new_v4(),
            name,
            cwd,
            panels: Vec::new(),
            viewport_pan: Vec2::new(100.0, 50.0),
            viewport_zoom: 0.65,
            next_z: 0,
            next_color: 0,
        }
    }

    pub fn bring_to_front(&mut self, index: usize) {
        for p in &mut self.panels {
            p.focused = false;
        }
        self.panels[index].focused = true;
        self.panels[index].z_index = self.next_z;
        self.next_z += 1;
    }

    pub fn spawn_terminal(&mut self, ctx: &egui::Context, colors: &[egui::Color32]) {
        let color = colors[self.next_color % colors.len()];
        self.next_color += 1;

        let idx = self.panels.len();
        let col = (idx % 2) as f32;
        let row = (idx / 2) as f32;
        let position = egui::Pos2::new(50.0 + col * 1060.0, 50.0 + row * 660.0);

        // Unfocus all existing panels FIRST
        for p in &mut self.panels {
            p.focused = false;
        }

        let mut panel = TerminalPanel::new_with_terminal(
            ctx,
            position,
            Vec2::new(1000.0, 600.0),
            color,
            self.cwd.as_deref(),
        );
        panel.z_index = self.next_z;
        panel.focused = true; // New panel gets focus
        self.next_z += 1;

        self.panels.push(panel);
    }

    pub fn close_panel(&mut self, idx: usize) {
        if idx < self.panels.len() {
            let was_focused = self.panels[idx].focused;
            self.panels.remove(idx);
            if was_focused {
                if let Some(last) = self.panels.last_mut() {
                    last.focused = true;
                }
            }
        }
    }

    pub fn close_focused(&mut self) {
        if let Some(idx) = self.panels.iter().position(|p| p.focused) {
            self.close_panel(idx);
        }
    }

    pub fn focus_next(&mut self) {
        if self.panels.is_empty() {
            return;
        }
        let cur = self.panels.iter().position(|p| p.focused).unwrap_or(0);
        let next = (cur + 1) % self.panels.len();
        self.bring_to_front(next);
    }

    pub fn focus_prev(&mut self) {
        if self.panels.is_empty() {
            return;
        }
        let cur = self.panels.iter().position(|p| p.focused).unwrap_or(0);
        let prev = if cur == 0 {
            self.panels.len() - 1
        } else {
            cur - 1
        };
        self.bring_to_front(prev);
    }
}
