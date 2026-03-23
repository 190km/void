// Workspace — each workspace is an independent canvas with its own panels

use egui::Vec2;
use std::path::PathBuf;
use uuid::Uuid;

use crate::canvas::config::{DEFAULT_PANEL_HEIGHT, DEFAULT_PANEL_WIDTH, PANEL_GAP};
use crate::panel::CanvasPanel;
use crate::terminal::panel::TerminalPanel;

pub struct Workspace {
    #[allow(dead_code)]
    pub id: Uuid,
    pub name: String,
    pub cwd: Option<PathBuf>,
    pub panels: Vec<CanvasPanel>,
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
            viewport_zoom: 0.75,
            next_z: 0,
            next_color: 0,
        }
    }

    /// Restore a workspace from saved state, spawning terminal processes.
    pub fn from_saved(
        ctx: &egui::Context,
        state: &crate::state::persistence::WorkspaceState,
        colors: &[egui::Color32],
    ) -> Self {
        let cwd = state.cwd.clone();
        let mut ws = Self {
            id: Uuid::parse_str(&state.id).unwrap_or_else(|_| Uuid::new_v4()),
            name: state.name.clone(),
            cwd: cwd.clone(),
            panels: Vec::new(),
            viewport_pan: Vec2::new(state.viewport_pan[0], state.viewport_pan[1]),
            viewport_zoom: state.viewport_zoom,
            next_z: state.next_z,
            next_color: state.next_color,
        };

        for panel_state in &state.panels {
            let panel = TerminalPanel::from_saved(ctx, panel_state, cwd.as_deref());
            ws.panels.push(CanvasPanel::Terminal(panel));
        }

        // If no panels were restored, spawn a default one
        if ws.panels.is_empty() {
            ws.spawn_terminal(ctx, colors);
        }

        ws
    }

    /// Snapshot the workspace layout for persistence.
    pub fn to_saved(&self) -> crate::state::persistence::WorkspaceState {
        crate::state::persistence::WorkspaceState {
            id: self.id.to_string(),
            name: self.name.clone(),
            cwd: self.cwd.clone(),
            panels: self.panels.iter().map(|p| p.to_saved()).collect(),
            viewport_pan: [self.viewport_pan.x, self.viewport_pan.y],
            viewport_zoom: self.viewport_zoom,
            next_z: self.next_z,
            next_color: self.next_color,
        }
    }

    pub fn bring_to_front(&mut self, index: usize) {
        for p in &mut self.panels {
            p.set_focused(false);
        }
        self.panels[index].set_focused(true);
        self.panels[index].set_z_index(self.next_z);
        self.next_z += 1;
    }

    pub fn spawn_terminal(&mut self, ctx: &egui::Context, colors: &[egui::Color32]) {
        let color = colors[self.next_color % colors.len()];
        self.next_color += 1;

        let new_size = Vec2::new(DEFAULT_PANEL_WIDTH, DEFAULT_PANEL_HEIGHT);
        let position = self.find_free_position(new_size);

        // Unfocus all existing panels FIRST
        for p in &mut self.panels {
            p.set_focused(false);
        }

        let mut panel = TerminalPanel::new_with_terminal(
            ctx,
            position,
            new_size,
            color,
            self.cwd.as_deref(),
        );
        panel.z_index = self.next_z;
        panel.focused = true;
        self.next_z += 1;

        self.panels.push(CanvasPanel::Terminal(panel));
    }

    /// Find the best position for a new panel that fills gaps intelligently.
    ///
    /// Generates candidate positions from every edge intersection of existing panels
    /// (the "corners" formed by their boundaries), then picks the one that:
    /// 1. Doesn't overlap anything
    /// 2. Is closest to the center of the existing layout (fills gaps first)
    /// 3. Minimizes the total bounding box (keeps things compact)
    fn find_free_position(&self, size: Vec2) -> egui::Pos2 {
        if self.panels.is_empty() {
            return egui::Pos2::new(50.0, 50.0);
        }

        let gap = PANEL_GAP;
        let rects: Vec<egui::Rect> = self.panels.iter().map(|p| p.rect()).collect();

        // Bounding box of all existing panels
        let mut bbox_min_x = f32::MAX;
        let mut bbox_min_y = f32::MAX;
        let mut bbox_max_x = f32::MIN;
        let mut bbox_max_y = f32::MIN;
        for r in &rects {
            bbox_min_x = bbox_min_x.min(r.min.x);
            bbox_min_y = bbox_min_y.min(r.min.y);
            bbox_max_x = bbox_max_x.max(r.max.x);
            bbox_max_y = bbox_max_y.max(r.max.y);
        }
        let bbox_center = egui::Pos2::new(
            (bbox_min_x + bbox_max_x) * 0.5,
            (bbox_min_y + bbox_max_y) * 0.5,
        );

        // Collect all unique X and Y edges from existing panels.
        // Candidate positions are at every (x_edge, y_edge) intersection —
        // these are the "corners" where a new panel could snap to fill a gap.
        let mut x_edges: Vec<f32> = Vec::new();
        let mut y_edges: Vec<f32> = Vec::new();

        for r in &rects {
            // Right edge + gap  → align new panel's left to this x
            x_edges.push(r.max.x + gap);
            // Left edge of panel → align new panel's left here too
            x_edges.push(r.min.x);
            // Left edge - gap - width → align new panel's right to this panel's left
            x_edges.push(r.min.x - gap - size.x);

            // Bottom edge + gap → align new panel's top to this y
            y_edges.push(r.max.y + gap);
            // Top edge of panel → align new panel's top here too
            y_edges.push(r.min.y);
            // Top edge - gap - height → align new panel's bottom to this panel's top
            y_edges.push(r.min.y - gap - size.y);
        }

        // Also try the bounding box origin
        x_edges.push(bbox_min_x);
        y_edges.push(bbox_min_y);

        // Deduplicate (within 1px tolerance)
        x_edges.sort_by(|a, b| a.partial_cmp(b).unwrap());
        x_edges.dedup_by(|a, b| (*a - *b).abs() < 1.0);
        y_edges.sort_by(|a, b| a.partial_cmp(b).unwrap());
        y_edges.dedup_by(|a, b| (*a - *b).abs() < 1.0);

        // Test every (x, y) candidate and score it
        let mut best: Option<(egui::Pos2, f32)> = None;

        for &x in &x_edges {
            for &y in &y_edges {
                let candidate = egui::Pos2::new(x, y);

                if Self::overlaps_any(candidate, size, &rects, gap) {
                    continue;
                }

                // Score: prefer positions that keep the layout compact.
                // Lower score = better.
                let candidate_rect = egui::Rect::from_min_size(candidate, size);

                // How much would the total bounding box grow?
                let new_min_x = bbox_min_x.min(candidate_rect.min.x);
                let new_min_y = bbox_min_y.min(candidate_rect.min.y);
                let new_max_x = bbox_max_x.max(candidate_rect.max.x);
                let new_max_y = bbox_max_y.max(candidate_rect.max.y);
                let bbox_growth = (new_max_x - new_min_x) * (new_max_y - new_min_y)
                    - (bbox_max_x - bbox_min_x) * (bbox_max_y - bbox_min_y);

                // Distance from center of existing layout
                let candidate_center = candidate_rect.center();
                let dist = ((candidate_center.x - bbox_center.x).powi(2)
                    + (candidate_center.y - bbox_center.y).powi(2))
                .sqrt();

                // Combined score: heavily weight bbox growth (fills gaps),
                // then use distance as tiebreaker
                let score = bbox_growth * 2.0 + dist;

                if best.is_none() || score < best.unwrap().1 {
                    best = Some((candidate, score));
                }
            }
        }

        if let Some((pos, _)) = best {
            return pos;
        }

        // Fallback: place below everything
        egui::Pos2::new(bbox_min_x, bbox_max_y + gap)
    }

    fn overlaps_any(pos: egui::Pos2, size: Vec2, rects: &[egui::Rect], min_gap: f32) -> bool {
        let candidate = egui::Rect::from_min_size(pos, size);
        let half = min_gap * 0.5;
        rects
            .iter()
            .any(|r| candidate.expand(half).intersects(r.expand(half)))
    }

    pub fn close_panel(&mut self, idx: usize) {
        if idx < self.panels.len() {
            let was_focused = self.panels[idx].focused();
            self.panels.remove(idx);
            if was_focused {
                if let Some(last) = self.panels.last_mut() {
                    last.set_focused(true);
                }
            }
        }
    }

    pub fn close_focused(&mut self) {
        if let Some(idx) = self.panels.iter().position(|p| p.focused()) {
            self.close_panel(idx);
        }
    }

    pub fn focus_next(&mut self) {
        if self.panels.is_empty() {
            return;
        }
        let cur = self.panels.iter().position(|p| p.focused()).unwrap_or(0);
        let next = (cur + 1) % self.panels.len();
        self.bring_to_front(next);
    }

    pub fn focus_prev(&mut self) {
        if self.panels.is_empty() {
            return;
        }
        let cur = self.panels.iter().position(|p| p.focused()).unwrap_or(0);
        let prev = if cur == 0 {
            self.panels.len() - 1
        } else {
            cur - 1
        };
        self.bring_to_front(prev);
    }
}
