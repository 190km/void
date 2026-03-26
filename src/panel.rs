// CanvasPanel — unified wrapper for panels on the canvas

use egui::{Color32, Pos2, Rect, Vec2};
use uuid::Uuid;

use crate::kanban::KanbanPanel;
use crate::network::NetworkPanel;
use crate::terminal::panel::{PanelInteraction, TerminalPanel};

pub enum CanvasPanel {
    Terminal(TerminalPanel),
    Kanban(KanbanPanel),
    Network(NetworkPanel),
}

impl CanvasPanel {
    pub fn id(&self) -> Uuid {
        match self {
            Self::Terminal(t) => t.id,
            Self::Kanban(k) => k.id,
            Self::Network(n) => n.id,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::Terminal(t) => &t.title,
            Self::Kanban(_) => "Kanban",
            Self::Network(_) => "Network",
        }
    }

    pub fn set_title(&mut self, title: String) {
        match self {
            Self::Terminal(t) => t.title = title,
            Self::Kanban(_) | Self::Network(_) => {} // no-op
        }
    }

    pub fn position(&self) -> Pos2 {
        match self {
            Self::Terminal(t) => t.position,
            Self::Kanban(k) => k.position,
            Self::Network(n) => n.position,
        }
    }

    pub fn set_position(&mut self, pos: Pos2) {
        match self {
            Self::Terminal(t) => t.position = pos,
            Self::Kanban(k) => k.position = pos,
            Self::Network(n) => n.position = pos,
        }
    }

    pub fn size(&self) -> Vec2 {
        match self {
            Self::Terminal(t) => t.size,
            Self::Kanban(k) => k.size,
            Self::Network(n) => n.size,
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            Self::Terminal(t) => t.color,
            Self::Kanban(_) => Color32::from_rgb(59, 130, 246), // blue
            Self::Network(_) => Color32::from_rgb(168, 85, 247), // purple
        }
    }

    pub fn z_index(&self) -> u32 {
        match self {
            Self::Terminal(t) => t.z_index,
            Self::Kanban(k) => k.z_index,
            Self::Network(n) => n.z_index,
        }
    }

    pub fn set_z_index(&mut self, z: u32) {
        match self {
            Self::Terminal(t) => t.z_index = z,
            Self::Kanban(k) => k.z_index = z,
            Self::Network(n) => n.z_index = z,
        }
    }

    pub fn focused(&self) -> bool {
        match self {
            Self::Terminal(t) => t.focused,
            Self::Kanban(k) => k.focused,
            Self::Network(n) => n.focused,
        }
    }

    pub fn set_focused(&mut self, f: bool) {
        match self {
            Self::Terminal(t) => t.focused = f,
            Self::Kanban(k) => k.focused = f,
            Self::Network(n) => n.focused = f,
        }
    }

    pub fn rect(&self) -> Rect {
        match self {
            Self::Terminal(t) => t.rect(),
            Self::Kanban(k) => k.rect(),
            Self::Network(n) => n.rect(),
        }
    }

    pub fn is_alive(&self) -> bool {
        match self {
            Self::Terminal(t) => t.is_alive(),
            Self::Kanban(_) => true,
            Self::Network(_) => true,
        }
    }

    pub fn drag_virtual_pos(&self) -> Option<Pos2> {
        match self {
            Self::Terminal(t) => t.drag_virtual_pos,
            Self::Kanban(k) => k.drag_virtual_pos,
            Self::Network(n) => n.drag_virtual_pos,
        }
    }

    pub fn set_drag_virtual_pos(&mut self, pos: Option<Pos2>) {
        match self {
            Self::Terminal(t) => t.drag_virtual_pos = pos,
            Self::Kanban(k) => k.drag_virtual_pos = pos,
            Self::Network(n) => n.drag_virtual_pos = pos,
        }
    }

    pub fn resize_virtual_rect(&self) -> Option<Rect> {
        match self {
            Self::Terminal(t) => t.resize_virtual_rect,
            Self::Kanban(k) => k.resize_virtual_rect,
            Self::Network(n) => n.resize_virtual_rect,
        }
    }

    pub fn set_resize_virtual_rect(&mut self, rect: Option<Rect>) {
        match self {
            Self::Terminal(t) => t.resize_virtual_rect = rect,
            Self::Kanban(k) => k.resize_virtual_rect = rect,
            Self::Network(n) => n.resize_virtual_rect = rect,
        }
    }

    pub fn apply_resize(&mut self, delta: Vec2) {
        match self {
            Self::Terminal(t) => t.apply_resize(delta),
            Self::Kanban(k) => {
                k.size.x = (k.size.x + delta.x).max(400.0);
                k.size.y = (k.size.y + delta.y).max(280.0);
            }
            Self::Network(n) => {
                n.size.x = (n.size.x + delta.x).max(400.0);
                n.size.y = (n.size.y + delta.y).max(280.0);
            }
        }
    }

    #[allow(dead_code)]
    pub fn apply_resize_left(&mut self, delta: Vec2) {
        match self {
            Self::Terminal(t) => t.apply_resize_left(delta),
            Self::Kanban(_) | Self::Network(_) => {} // no-op for now
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        transform: egui::emath::TSTransform,
        screen_clip: Rect,
    ) -> PanelInteraction {
        match self {
            Self::Terminal(t) => t.show(ui, transform, screen_clip),
            Self::Kanban(k) => {
                let ki = k.show(ui, transform, screen_clip);
                match ki {
                    crate::kanban::KanbanInteraction::DragStart => PanelInteraction {
                        dragging_title: true,
                        ..Default::default()
                    },
                    crate::kanban::KanbanInteraction::Clicked => PanelInteraction {
                        clicked: true,
                        ..Default::default()
                    },
                    _ => PanelInteraction::default(),
                }
            }
            Self::Network(n) => {
                let ni = n.show(ui, transform, screen_clip);
                match ni {
                    crate::network::NetworkInteraction::DragStart => PanelInteraction {
                        dragging_title: true,
                        ..Default::default()
                    },
                    crate::network::NetworkInteraction::Clicked => PanelInteraction {
                        clicked: true,
                        ..Default::default()
                    },
                    _ => PanelInteraction::default(),
                }
            }
        }
    }

    /// Serialize for persistence. Kanban and Network return None (not persisted).
    pub fn to_saved(&self) -> Option<crate::state::persistence::PanelState> {
        match self {
            Self::Terminal(t) => Some(t.to_saved()),
            Self::Kanban(_) => None,
            Self::Network(_) => None,
        }
    }

    pub fn scroll_hit_test(&self, canvas_pos: Pos2) -> bool {
        match self {
            Self::Terminal(t) => t.scroll_hit_test(canvas_pos),
            Self::Kanban(_) | Self::Network(_) => false,
        }
    }

    pub fn handle_scroll(&mut self, ctx: &egui::Context, scroll_y: f32) {
        match self {
            Self::Terminal(t) => t.handle_scroll(ctx, scroll_y),
            Self::Kanban(_) | Self::Network(_) => {} // no-op
        }
    }

    pub fn sync_title(&mut self) {
        match self {
            Self::Terminal(t) => t.sync_title(),
            Self::Kanban(_) | Self::Network(_) => {} // no-op
        }
    }

    pub fn handle_input(&mut self, ctx: &egui::Context) {
        match self {
            Self::Terminal(t) => t.handle_input(ctx),
            Self::Kanban(_) | Self::Network(_) => {} // no-op
        }
    }
}
