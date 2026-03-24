// CanvasPanel — unified wrapper for panels on the canvas

use egui::{Color32, Pos2, Rect, Vec2};
use uuid::Uuid;

use crate::terminal::panel::{PanelInteraction, TerminalPanel};

pub enum CanvasPanel {
    Terminal(TerminalPanel),
}

impl CanvasPanel {
    pub fn id(&self) -> Uuid {
        match self {
            Self::Terminal(t) => t.id,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::Terminal(t) => &t.title,
        }
    }

    pub fn set_title(&mut self, title: String) {
        match self {
            Self::Terminal(t) => t.title = title,
        }
    }

    pub fn position(&self) -> Pos2 {
        match self {
            Self::Terminal(t) => t.position,
        }
    }

    pub fn set_position(&mut self, pos: Pos2) {
        match self {
            Self::Terminal(t) => t.position = pos,
        }
    }

    pub fn size(&self) -> Vec2 {
        match self {
            Self::Terminal(t) => t.size,
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            Self::Terminal(t) => t.color,
        }
    }

    pub fn z_index(&self) -> u32 {
        match self {
            Self::Terminal(t) => t.z_index,
        }
    }

    pub fn set_z_index(&mut self, z: u32) {
        match self {
            Self::Terminal(t) => t.z_index = z,
        }
    }

    pub fn focused(&self) -> bool {
        match self {
            Self::Terminal(t) => t.focused,
        }
    }

    pub fn set_focused(&mut self, f: bool) {
        match self {
            Self::Terminal(t) => t.focused = f,
        }
    }

    pub fn rect(&self) -> Rect {
        match self {
            Self::Terminal(t) => t.rect(),
        }
    }

    pub fn is_alive(&self) -> bool {
        match self {
            Self::Terminal(t) => t.is_alive(),
        }
    }

    pub fn drag_virtual_pos(&self) -> Option<Pos2> {
        match self {
            Self::Terminal(t) => t.drag_virtual_pos,
        }
    }

    pub fn set_drag_virtual_pos(&mut self, pos: Option<Pos2>) {
        match self {
            Self::Terminal(t) => t.drag_virtual_pos = pos,
        }
    }

    pub fn resize_virtual_rect(&self) -> Option<Rect> {
        match self {
            Self::Terminal(t) => t.resize_virtual_rect,
        }
    }

    pub fn set_resize_virtual_rect(&mut self, rect: Option<Rect>) {
        match self {
            Self::Terminal(t) => t.resize_virtual_rect = rect,
        }
    }

    pub fn apply_resize(&mut self, delta: Vec2) {
        match self {
            Self::Terminal(t) => t.apply_resize(delta),
        }
    }

    #[allow(dead_code)]
    pub fn apply_resize_left(&mut self, delta: Vec2) {
        match self {
            Self::Terminal(t) => t.apply_resize_left(delta),
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        transform: egui::emath::TSTransform,
        screen_clip: Rect,
        font_size: f32,
    ) -> PanelInteraction {
        match self {
            Self::Terminal(t) => t.show(ui, transform, screen_clip, font_size),
        }
    }

    pub fn to_saved(&self) -> crate::state::persistence::PanelState {
        match self {
            Self::Terminal(t) => t.to_saved(),
        }
    }

    pub fn scroll_hit_test(&self, canvas_pos: Pos2) -> bool {
        match self {
            Self::Terminal(t) => t.scroll_hit_test(canvas_pos),
        }
    }

    pub fn handle_scroll(&mut self, ctx: &egui::Context, scroll_y: f32, font_size: f32) {
        match self {
            Self::Terminal(t) => t.handle_scroll(ctx, scroll_y, font_size),
        }
    }

    pub fn sync_title(&mut self) {
        match self {
            Self::Terminal(t) => t.sync_title(),
        }
    }

    pub fn handle_input(&mut self, ctx: &egui::Context) {
        match self {
            Self::Terminal(t) => t.handle_input(ctx),
        }
    }
}
