// src/network/mod.rs — Network visualization canvas panel
//
// Live graph of agents (terminals) as nodes and their communications
// as animated edges with particles.

#![allow(dead_code)]

use egui::{Color32, Pos2, Rect, Vec2};
use std::sync::mpsc;
use uuid::Uuid;

use crate::bus::types::*;

// ─── Colors ─────────────────────────────────────────────────────

const NETWORK_BG: Color32 = Color32::from_rgb(17, 17, 21);
const NETWORK_BORDER: Color32 = Color32::from_rgb(39, 39, 42);
const GRID_COLOR: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 8);
const NODE_BG: Color32 = Color32::from_rgb(39, 39, 42);
const NODE_BORDER: Color32 = Color32::from_rgb(63, 63, 70);
const NODE_TEXT: Color32 = Color32::from_rgb(228, 228, 231);
const NODE_TEXT_DIM: Color32 = Color32::from_rgb(113, 113, 122);
const TITLE_BAR_HEIGHT: f32 = 32.0;
const BORDER_RADIUS: f32 = 8.0;

// ─── Force Layout Constants ─────────────────────────────────────

const REPULSION: f32 = 8000.0;
const ATTRACTION: f32 = 0.01;
const CENTER_GRAVITY: f32 = 0.005;
const DAMPING: f32 = 0.85;
const MAX_VELOCITY: f32 = 5.0;
const ITERATIONS_PER_FRAME: usize = 3;

// ─── Node ───────────────────────────────────────────────────────

pub struct NetworkNode {
    pub terminal_id: Uuid,
    pub pos: Pos2,
    pub radius: f32,
    pub role: TerminalRole,
    pub color: Color32,
    pub status: String,
    pub active_task: Option<String>,
    pub title: String,
    pub activity: f32,
}

// ─── Edge ───────────────────────────────────────────────────────

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

pub struct NetworkEdge {
    pub from: Uuid,
    pub to: Uuid,
    pub edge_type: EdgeType,
    pub event_count: u32,
    pub particles: Vec<EdgeParticle>,
    pub thickness: f32,
}

#[derive(Debug, Clone)]
pub struct EdgeParticle {
    pub t: f32,
    pub speed: f32,
    pub size: f32,
    pub color: Color32,
}

// ─── Panel ──────────────────────────────────────────────────────

pub struct NetworkPanel {
    pub id: Uuid,
    pub position: Pos2,
    pub size: Vec2,
    pub z_index: u32,
    pub focused: bool,

    /// Group this view is bound to.
    pub group_id: Uuid,

    /// Nodes (one per terminal in group).
    nodes: Vec<NetworkNode>,

    /// Edges (connections between nodes).
    edges: Vec<NetworkEdge>,

    /// Event subscription for real-time updates.
    subscription_id: Uuid,
    event_rx: mpsc::Receiver<BusEvent>,

    /// Edge type visibility toggles.
    show_commands: bool,
    show_messages: bool,
    show_dependencies: bool,
    show_broadcasts: bool,

    /// Internal zoom level.
    internal_zoom: f32,

    /// Drag state.
    pub drag_virtual_pos: Option<Pos2>,
    pub resize_virtual_rect: Option<Rect>,

    /// Animation time accumulator.
    anim_time: f32,

    /// Stats counters.
    total_messages: u32,
    total_commands: u32,
    total_tasks: u32,
}

impl NetworkPanel {
    pub fn new(
        position: Pos2,
        group_id: Uuid,
        subscription_id: Uuid,
        event_rx: mpsc::Receiver<BusEvent>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            position,
            size: Vec2::new(600.0, 500.0),
            z_index: 0,
            focused: false,
            group_id,
            nodes: Vec::new(),
            edges: Vec::new(),
            subscription_id,
            event_rx,
            show_commands: true,
            show_messages: true,
            show_dependencies: true,
            show_broadcasts: true,
            internal_zoom: 1.0,
            drag_virtual_pos: None,
            resize_virtual_rect: None,
            anim_time: 0.0,
            total_messages: 0,
            total_commands: 0,
            total_tasks: 0,
        }
    }

    pub fn rect(&self) -> Rect {
        Rect::from_min_size(self.position, self.size)
    }

    /// Sync nodes from bus group members.
    pub fn sync_nodes(&mut self, bus: &crate::bus::TerminalBus) {
        if let Some(group_info) = bus.get_group(self.group_id) {
            // Add missing nodes
            for member in &group_info.members {
                if !self
                    .nodes
                    .iter()
                    .any(|n| n.terminal_id == member.terminal_id)
                {
                    let radius = match member.role {
                        TerminalRole::Orchestrator => 45.0,
                        TerminalRole::Worker if member.status.is_active() => 35.0,
                        TerminalRole::Worker => 30.0,
                        _ => 30.0,
                    };
                    let center = Pos2::new(self.size.x / 2.0, self.size.y / 2.0);
                    let angle = self.nodes.len() as f32 * 2.094; // ~120 degrees
                    let dist = 120.0;
                    let pos = if member.role == TerminalRole::Orchestrator {
                        center
                    } else {
                        Pos2::new(center.x + angle.cos() * dist, center.y + angle.sin() * dist)
                    };

                    self.nodes.push(NetworkNode {
                        terminal_id: member.terminal_id,
                        pos,
                        radius,
                        role: member.role,
                        color: Color32::from_rgb(59, 130, 246),
                        status: member.status.label().to_string(),
                        active_task: None,
                        title: member.title.clone(),
                        activity: 0.0,
                    });
                } else {
                    // Update existing node
                    if let Some(node) = self
                        .nodes
                        .iter_mut()
                        .find(|n| n.terminal_id == member.terminal_id)
                    {
                        node.title = member.title.clone();
                        node.status = member.status.label().to_string();
                        node.role = member.role;
                    }
                }
            }

            // Remove nodes for terminals that left
            let member_ids: Vec<Uuid> = group_info.members.iter().map(|m| m.terminal_id).collect();
            self.nodes.retain(|n| member_ids.contains(&n.terminal_id));
        }
    }

    /// Process pending events.
    pub fn process_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match &event {
                BusEvent::CommandInjected {
                    source: Some(src),
                    target,
                    ..
                } => {
                    self.spawn_particle(*src, *target, EdgeType::Command);
                    self.total_commands += 1;
                }
                BusEvent::MessageSent { from, to, .. } => {
                    self.spawn_particle(*from, *to, EdgeType::Message);
                    self.total_messages += 1;
                }
                BusEvent::BroadcastSent { from, .. } => {
                    let targets: Vec<Uuid> = self
                        .nodes
                        .iter()
                        .filter(|n| n.terminal_id != *from)
                        .map(|n| n.terminal_id)
                        .collect();
                    for target in targets {
                        self.spawn_particle(*from, target, EdgeType::Broadcast);
                    }
                }
                BusEvent::TaskCreated { .. } | BusEvent::TaskStatusChanged { .. } => {
                    self.total_tasks += 1;
                }
                _ => {}
            }
        }
    }

    fn spawn_particle(&mut self, from: Uuid, to: Uuid, edge_type: EdgeType) {
        // Find or create edge
        let edge = self
            .edges
            .iter_mut()
            .find(|e| e.from == from && e.to == to && e.edge_type == edge_type);
        if let Some(edge) = edge {
            edge.event_count += 1;
            edge.particles.push(EdgeParticle {
                t: 0.0,
                speed: 0.8,
                size: 3.0,
                color: edge_type.color(),
            });
        } else {
            let mut edge = NetworkEdge {
                from,
                to,
                edge_type,
                event_count: 1,
                particles: Vec::new(),
                thickness: edge_type.base_thickness(),
            };
            edge.particles.push(EdgeParticle {
                t: 0.0,
                speed: 0.8,
                size: 3.0,
                color: edge_type.color(),
            });
            self.edges.push(edge);
        }
    }

    /// Run force-directed layout step.
    fn layout_step(&mut self) {
        let center = Pos2::new(
            self.size.x / 2.0,
            (self.size.y - TITLE_BAR_HEIGHT) / 2.0 + TITLE_BAR_HEIGHT,
        );
        let n = self.nodes.len();
        if n < 2 {
            // Pin single node to center
            if let Some(node) = self.nodes.first_mut() {
                node.pos = center;
            }
            return;
        }

        for _ in 0..ITERATIONS_PER_FRAME {
            let mut forces: Vec<Vec2> = vec![Vec2::ZERO; n];

            // Repulsion
            for i in 0..n {
                for j in (i + 1)..n {
                    let delta = self.nodes[i].pos - self.nodes[j].pos;
                    let dist_sq = delta.length_sq().max(1.0);
                    let force = delta.normalized() * (REPULSION / dist_sq);
                    forces[i] += force;
                    forces[j] -= force;
                }
            }

            // Attraction (connected pairs)
            for edge in &self.edges {
                let i = self.nodes.iter().position(|n| n.terminal_id == edge.from);
                let j = self.nodes.iter().position(|n| n.terminal_id == edge.to);
                if let (Some(i), Some(j)) = (i, j) {
                    let delta = self.nodes[j].pos - self.nodes[i].pos;
                    let force = delta * ATTRACTION;
                    forces[i] += force;
                    forces[j] -= force;
                }
            }

            // Center gravity
            for (i, node) in self.nodes.iter().enumerate() {
                let to_center = center - node.pos;
                forces[i] += to_center * CENTER_GRAVITY;
            }

            // Apply forces
            for (i, node) in self.nodes.iter_mut().enumerate() {
                if node.role == TerminalRole::Orchestrator {
                    node.pos = center;
                    continue;
                }
                let f = forces[i];
                let len = f.length();
                let clamped = if len > MAX_VELOCITY {
                    f * (MAX_VELOCITY / len)
                } else {
                    f
                };
                let velocity = clamped * DAMPING;
                node.pos += velocity;
            }
        }
    }

    /// Tick animations.
    fn tick_animations(&mut self, dt: f32) {
        self.anim_time += dt;

        // Advance particles
        for edge in &mut self.edges {
            edge.particles.retain_mut(|p| {
                p.t += p.speed * dt;
                p.t < 1.0
            });
        }

        // Decay node activity
        for node in &mut self.nodes {
            node.activity *= 0.95;
        }

        // Clean up old edges with no particles
        self.edges
            .retain(|e| !e.particles.is_empty() || e.event_count > 0);
    }

    /// Render the network panel.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        _transform: egui::emath::TSTransform,
        screen_clip: Rect,
    ) -> NetworkInteraction {
        let panel_rect = self.rect();

        if !screen_clip.intersects(panel_rect) {
            return NetworkInteraction::None;
        }

        let dt = ui.input(|i| i.stable_dt).min(0.1);

        // Process events and physics
        self.process_events();
        self.layout_step();
        self.tick_animations(dt);

        let painter = ui.painter();

        // Panel background
        painter.rect_filled(
            panel_rect.expand(2.0),
            BORDER_RADIUS + 1.0,
            Color32::from_rgba_premultiplied(0, 0, 0, 40),
        );
        painter.rect_filled(panel_rect, BORDER_RADIUS, NETWORK_BG);

        let border_color = if self.focused {
            Color32::from_rgb(168, 85, 247)
        } else {
            NETWORK_BORDER
        };
        painter.rect_stroke(
            panel_rect,
            BORDER_RADIUS,
            egui::Stroke::new(1.0, border_color),
        );

        // Title bar
        let title_rect = Rect::from_min_size(
            panel_rect.min,
            Vec2::new(panel_rect.width(), TITLE_BAR_HEIGHT),
        );
        painter.rect_filled(
            title_rect,
            egui::Rounding {
                nw: BORDER_RADIUS,
                ne: BORDER_RADIUS,
                sw: 0.0,
                se: 0.0,
            },
            Color32::from_rgb(30, 30, 33),
        );

        painter.text(
            Pos2::new(title_rect.min.x + 12.0, title_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "Network",
            egui::FontId::proportional(12.0),
            NODE_TEXT,
        );

        let title_resp = ui.interact(
            title_rect,
            egui::Id::new(self.id).with("network_title"),
            egui::Sense::drag(),
        );
        let mut interaction = NetworkInteraction::None;
        if title_resp.dragged() {
            interaction = NetworkInteraction::DragStart;
        }
        if title_resp.clicked() {
            interaction = NetworkInteraction::Clicked;
        }

        // Content area
        let _content_rect = Rect::from_min_max(
            Pos2::new(panel_rect.min.x, panel_rect.min.y + TITLE_BAR_HEIGHT),
            panel_rect.max,
        );

        // Draw edges
        for edge in &self.edges {
            let from_pos = self
                .nodes
                .iter()
                .find(|n| n.terminal_id == edge.from)
                .map(|n| Pos2::new(panel_rect.min.x + n.pos.x, panel_rect.min.y + n.pos.y));
            let to_pos = self
                .nodes
                .iter()
                .find(|n| n.terminal_id == edge.to)
                .map(|n| Pos2::new(panel_rect.min.x + n.pos.x, panel_rect.min.y + n.pos.y));

            if let (Some(from), Some(to)) = (from_pos, to_pos) {
                let color = edge.edge_type.color();
                let alpha = 100;
                let line_color =
                    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);
                painter.line_segment([from, to], egui::Stroke::new(edge.thickness, line_color));

                // Draw particles
                for particle in &edge.particles {
                    let pos = Pos2::new(
                        from.x + (to.x - from.x) * particle.t,
                        from.y + (to.y - from.y) * particle.t,
                    );
                    painter.circle_filled(pos, particle.size, particle.color);

                    // Trail
                    for i in 1..=3 {
                        let trail_t = (particle.t - 0.03 * i as f32).max(0.0);
                        let trail_pos = Pos2::new(
                            from.x + (to.x - from.x) * trail_t,
                            from.y + (to.y - from.y) * trail_t,
                        );
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
        }

        // Draw nodes
        for node in &self.nodes {
            let node_pos = Pos2::new(panel_rect.min.x + node.pos.x, panel_rect.min.y + node.pos.y);

            // Activity glow
            if node.activity > 0.05 {
                let glow_alpha = (node.activity * 80.0) as u8;
                let glow_color = Color32::from_rgba_unmultiplied(
                    node.color.r(),
                    node.color.g(),
                    node.color.b(),
                    glow_alpha,
                );
                painter.circle_filled(node_pos, node.radius + 6.0, glow_color);
            }

            // Node background
            let node_rect =
                Rect::from_center_size(node_pos, Vec2::new(node.radius * 2.0, node.radius * 1.6));
            painter.rect_filled(node_rect, 6.0, NODE_BG);
            painter.rect_stroke(node_rect, 6.0, egui::Stroke::new(1.0, NODE_BORDER));

            // Role indicator + title
            let indicator = node.role.indicator();
            let label = format!("{} {}", indicator, node.title);
            painter.text(
                Pos2::new(node_pos.x, node_pos.y - 6.0),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(10.0),
                NODE_TEXT,
            );

            // Status dot
            let status_color = match node.status.as_str() {
                "running" => Color32::from_rgb(59, 130, 246),
                "idle" => Color32::from_rgb(163, 163, 163),
                "done" => Color32::from_rgb(34, 197, 94),
                "error" => Color32::from_rgb(239, 68, 68),
                _ => Color32::GRAY,
            };
            painter.circle_filled(
                Pos2::new(node_pos.x - node.radius + 8.0, node_pos.y + 8.0),
                3.0,
                status_color,
            );
            painter.text(
                Pos2::new(node_pos.x - node.radius + 14.0, node_pos.y + 8.0),
                egui::Align2::LEFT_CENTER,
                &node.status,
                egui::FontId::proportional(9.0),
                NODE_TEXT_DIM,
            );

            // Click to focus terminal
            let node_resp = ui.interact(
                node_rect,
                egui::Id::new(self.id).with(node.terminal_id),
                egui::Sense::click(),
            );
            if node_resp.clicked() {
                return NetworkInteraction::FocusTerminal(node.terminal_id);
            }
        }

        // Legend
        let legend_y = panel_rect.max.y - 20.0;
        let legend_text = format!(
            "messages: {}  commands: {}  tasks: {}",
            self.total_messages, self.total_commands, self.total_tasks
        );
        painter.text(
            Pos2::new(panel_rect.min.x + 12.0, legend_y),
            egui::Align2::LEFT_CENTER,
            legend_text,
            egui::FontId::proportional(9.0),
            NODE_TEXT_DIM,
        );

        interaction
    }

    pub fn subscription_id(&self) -> Uuid {
        self.subscription_id
    }
}

#[derive(Debug)]
pub enum NetworkInteraction {
    None,
    Clicked,
    FocusTerminal(Uuid),
    DragStart,
    ResizeStart,
}
