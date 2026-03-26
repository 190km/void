// src/canvas/edges.rs — Canvas edge overlay for inter-panel connections
//
// Draws animated connection lines between terminal panels on the canvas.
// Renders ABOVE the background but BELOW panel contents.

#![allow(dead_code)]

use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};
use std::collections::HashMap;
use std::time::Instant;

use uuid::Uuid;

use crate::bus::types::BusEvent;

// ─── Edge Types (reused from network) ───────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    Command,
    Message,
    Dependency,
    Broadcast,
}

impl EdgeType {
    pub fn color(&self) -> Color32 {
        match self {
            Self::Command => Color32::from_rgb(59, 130, 246),
            Self::Message => Color32::from_rgb(163, 163, 163),
            Self::Dependency => Color32::from_rgb(234, 179, 8),
            Self::Broadcast => Color32::from_rgb(168, 85, 247),
        }
    }

    pub fn base_thickness(&self) -> f32 {
        match self {
            Self::Command => 2.0,
            Self::Message => 1.5,
            Self::Dependency => 1.0,
            Self::Broadcast => 3.0,
        }
    }
}

// ─── Structures ─────────────────────────────────────────────────

struct CanvasEdge {
    from: Uuid,
    to: Uuid,
    edge_type: EdgeType,
    event_count: u32,
    last_event_at: Instant,
}

struct CanvasParticle {
    from: Uuid,
    to: Uuid,
    t: f32,
    speed: f32,
    color: Color32,
    size: f32,
}

/// Overlay that draws animated connection lines between panels on the canvas.
pub struct CanvasEdgeOverlay {
    edges: Vec<CanvasEdge>,
    particles: Vec<CanvasParticle>,
    pub enabled: bool,
}

impl CanvasEdgeOverlay {
    pub fn new() -> Self {
        Self {
            edges: Vec::new(),
            particles: Vec::new(),
            enabled: false,
        }
    }

    /// Register a communication event. Creates edge if needed, spawns particle.
    pub fn on_event(&mut self, event: &BusEvent) {
        if !self.enabled {
            return;
        }

        match event {
            BusEvent::CommandInjected {
                source: Some(src),
                target,
                ..
            } => {
                self.register_edge(*src, *target, EdgeType::Command);
                self.spawn_particle(*src, *target, EdgeType::Command);
            }
            BusEvent::MessageSent { from, to, .. } => {
                self.register_edge(*from, *to, EdgeType::Message);
                self.spawn_particle(*from, *to, EdgeType::Message);
            }
            BusEvent::BroadcastSent { from, .. } => {
                // Broadcast particles are spawned per-target elsewhere
                let _ = from;
            }
            _ => {}
        }
    }

    fn register_edge(&mut self, from: Uuid, to: Uuid, edge_type: EdgeType) {
        if let Some(edge) = self
            .edges
            .iter_mut()
            .find(|e| e.from == from && e.to == to && e.edge_type == edge_type)
        {
            edge.event_count += 1;
            edge.last_event_at = Instant::now();
        } else {
            self.edges.push(CanvasEdge {
                from,
                to,
                edge_type,
                event_count: 1,
                last_event_at: Instant::now(),
            });
        }
    }

    fn spawn_particle(&mut self, from: Uuid, to: Uuid, edge_type: EdgeType) {
        // Cap particles at 100
        if self.particles.len() >= 100 {
            return;
        }
        self.particles.push(CanvasParticle {
            from,
            to,
            t: 0.0,
            speed: 0.8,
            color: edge_type.color(),
            size: 3.0,
        });
    }

    /// Draw all edges and particles.
    pub fn draw(
        &self,
        painter: &Painter,
        panel_rects: &HashMap<Uuid, Rect>,
        _transform: egui::emath::TSTransform,
    ) {
        if !self.enabled {
            return;
        }

        for edge in &self.edges {
            let from_rect = panel_rects.get(&edge.from);
            let to_rect = panel_rects.get(&edge.to);
            if let (Some(from), Some(to)) = (from_rect, to_rect) {
                self.draw_edge(painter, from, to, edge);
            }
        }

        for particle in &self.particles {
            let from_rect = panel_rects.get(&particle.from);
            let to_rect = panel_rects.get(&particle.to);
            if let (Some(from), Some(to)) = (from_rect, to_rect) {
                self.draw_particle(painter, from, to, particle);
            }
        }
    }

    /// Tick animations.
    pub fn tick(&mut self, dt: f32) {
        self.particles.retain_mut(|p| {
            p.t += p.speed * dt;
            p.t < 1.0
        });

        // Fade old edges
        let now = Instant::now();
        self.edges
            .retain(|e| now.duration_since(e.last_event_at).as_secs() < 120);
    }

    fn draw_edge(&self, painter: &Painter, from: &Rect, to: &Rect, edge: &CanvasEdge) {
        let (start, end) = closest_edge_points(from, to);

        let color = edge.edge_type.color();
        let alpha = 60;
        let line_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);
        let thickness = edge.edge_type.base_thickness();

        // Draw as bezier approximation
        let mid = Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
        let perpendicular = Vec2::new(-(end.y - start.y), end.x - start.x).normalized();
        let offset = perpendicular * 20.0;
        let cp = Pos2::new(mid.x + offset.x, mid.y + offset.y);

        // Simple quadratic bezier as line segments
        let segments = 16;
        let mut prev = start;
        for i in 1..=segments {
            let t = i as f32 / segments as f32;
            let it = 1.0 - t;
            let x = it * it * start.x + 2.0 * it * t * cp.x + t * t * end.x;
            let y = it * it * start.y + 2.0 * it * t * cp.y + t * t * end.y;
            let curr = Pos2::new(x, y);
            painter.line_segment([prev, curr], Stroke::new(thickness, line_color));
            prev = curr;
        }

        // Arrowhead
        let dir = (end - prev).normalized();
        let arrow_size = 6.0;
        let perp = Vec2::new(-dir.y, dir.x);
        let p1 = end - dir * arrow_size + perp * arrow_size * 0.5;
        let p2 = end - dir * arrow_size - perp * arrow_size * 0.5;
        painter.line_segment([p1, end], Stroke::new(thickness, line_color));
        painter.line_segment([p2, end], Stroke::new(thickness, line_color));
    }

    fn draw_particle(&self, painter: &Painter, from: &Rect, to: &Rect, particle: &CanvasParticle) {
        let (start, end) = closest_edge_points(from, to);
        let pos = lerp_pos(start, end, particle.t);
        painter.circle_filled(pos, particle.size, particle.color);

        // Trail
        for i in 1..=3 {
            let trail_t = (particle.t - 0.03 * i as f32).max(0.0);
            let trail_pos = lerp_pos(start, end, trail_t);
            let alpha = (255 - i * 60).max(0) as u8;
            let trail_color = Color32::from_rgba_unmultiplied(
                particle.color.r(),
                particle.color.g(),
                particle.color.b(),
                alpha,
            );
            painter.circle_filled(trail_pos, particle.size * 0.6, trail_color);
        }
    }
}

fn lerp_pos(a: Pos2, b: Pos2, t: f32) -> Pos2 {
    Pos2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

/// Find the closest points on the edges of two rectangles.
fn closest_edge_points(a: &Rect, b: &Rect) -> (Pos2, Pos2) {
    let a_center = a.center();
    let b_center = b.center();

    let start = rect_edge_intersection(a, a_center, b_center);
    let end = rect_edge_intersection(b, b_center, a_center);

    (start, end)
}

/// Find where a ray from `inside` toward `target` exits a rectangle.
fn rect_edge_intersection(rect: &Rect, inside: Pos2, target: Pos2) -> Pos2 {
    let dx = target.x - inside.x;
    let dy = target.y - inside.y;

    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        return inside;
    }

    let mut t_min = f32::MAX;

    if dx != 0.0 {
        let t = (rect.min.x - inside.x) / dx;
        let y = inside.y + t * dy;
        if t > 0.0 && t < t_min && y >= rect.min.y && y <= rect.max.y {
            t_min = t;
        }
        let t = (rect.max.x - inside.x) / dx;
        let y = inside.y + t * dy;
        if t > 0.0 && t < t_min && y >= rect.min.y && y <= rect.max.y {
            t_min = t;
        }
    }
    if dy != 0.0 {
        let t = (rect.min.y - inside.y) / dy;
        let x = inside.x + t * dx;
        if t > 0.0 && t < t_min && x >= rect.min.x && x <= rect.max.x {
            t_min = t;
        }
        let t = (rect.max.y - inside.y) / dy;
        let x = inside.x + t * dx;
        if t > 0.0 && t < t_min && x >= rect.min.x && x <= rect.max.x {
            t_min = t;
        }
    }

    if t_min == f32::MAX {
        inside
    } else {
        Pos2::new(inside.x + t_min * dx, inside.y + t_min * dy)
    }
}
