// CanvasPanel — unified wrapper for panels on the canvas

use egui::{Color32, Pos2, Rect, Vec2};
use uuid::Uuid;

use crate::application::panel::ApplicationPanel;
use crate::terminal::panel::{PanelInteraction, TerminalPanel};

pub enum CanvasPanel {
    Terminal(TerminalPanel),
    Application(ApplicationPanel),
}

impl CanvasPanel {
    pub fn id(&self) -> Uuid {
        match self {
            Self::Terminal(t) => t.id,
            Self::Application(a) => a.id,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::Terminal(t) => &t.title,
            Self::Application(a) => &a.title,
        }
    }

    pub fn set_title(&mut self, title: String) {
        match self {
            Self::Terminal(t) => t.title = title,
            Self::Application(a) => a.title = title,
        }
    }

    pub fn position(&self) -> Pos2 {
        match self {
            Self::Terminal(t) => t.position,
            Self::Application(a) => a.position,
        }
    }

    pub fn set_position(&mut self, pos: Pos2) {
        match self {
            Self::Terminal(t) => t.position = pos,
            Self::Application(a) => a.position = pos,
        }
    }

    pub fn size(&self) -> Vec2 {
        match self {
            Self::Terminal(t) => t.size,
            Self::Application(a) => a.size,
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            Self::Terminal(t) => t.color,
            Self::Application(a) => a.color,
        }
    }

    pub fn z_index(&self) -> u32 {
        match self {
            Self::Terminal(t) => t.z_index,
            Self::Application(a) => a.z_index,
        }
    }

    pub fn set_z_index(&mut self, z: u32) {
        match self {
            Self::Terminal(t) => t.z_index = z,
            Self::Application(a) => a.z_index = z,
        }
    }

    pub fn focused(&self) -> bool {
        match self {
            Self::Terminal(t) => t.focused,
            Self::Application(a) => a.focused,
        }
    }

    pub fn set_focused(&mut self, f: bool) {
        match self {
            Self::Terminal(t) => t.focused = f,
            Self::Application(a) => a.focused = f,
        }
    }

    pub fn rect(&self) -> Rect {
        match self {
            Self::Terminal(t) => t.rect(),
            Self::Application(a) => a.rect(),
        }
    }

    pub fn is_alive(&self) -> bool {
        match self {
            Self::Terminal(t) => t.is_alive(),
            Self::Application(a) => a.is_alive(),
        }
    }

    pub fn drag_virtual_pos(&self) -> Option<Pos2> {
        match self {
            Self::Terminal(t) => t.drag_virtual_pos,
            Self::Application(a) => a.drag_virtual_pos,
        }
    }

    pub fn set_drag_virtual_pos(&mut self, pos: Option<Pos2>) {
        match self {
            Self::Terminal(t) => t.drag_virtual_pos = pos,
            Self::Application(a) => a.drag_virtual_pos = pos,
        }
    }

    pub fn resize_virtual_rect(&self) -> Option<Rect> {
        match self {
            Self::Terminal(t) => t.resize_virtual_rect,
            Self::Application(a) => a.resize_virtual_rect,
        }
    }

    pub fn set_resize_virtual_rect(&mut self, rect: Option<Rect>) {
        match self {
            Self::Terminal(t) => t.resize_virtual_rect = rect,
            Self::Application(a) => a.resize_virtual_rect = rect,
        }
    }

    pub fn apply_resize(&mut self, delta: Vec2) {
        match self {
            Self::Terminal(t) => t.apply_resize(delta),
            Self::Application(a) => a.apply_resize(delta),
        }
    }

    #[allow(dead_code)]
    pub fn apply_resize_left(&mut self, delta: Vec2) {
        match self {
            Self::Terminal(t) => t.apply_resize_left(delta),
            Self::Application(a) => a.apply_resize_left(delta),
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        transform: egui::emath::TSTransform,
        screen_clip: Rect,
        #[cfg(windows)] void_hwnd: Option<windows::Win32::Foundation::HWND>,
    ) -> PanelInteraction {
        match self {
            Self::Terminal(t) => t.show(ui, transform, screen_clip),
            Self::Application(a) => a.show(
                ui,
                transform,
                screen_clip,
                #[cfg(windows)]
                void_hwnd,
            ),
        }
    }

    pub fn to_saved(&self) -> crate::state::persistence::PanelState {
        match self {
            Self::Terminal(t) => t.to_saved(),
            Self::Application(a) => a.to_saved(),
        }
    }

    pub fn scroll_hit_test(&self, canvas_pos: Pos2) -> bool {
        match self {
            Self::Terminal(t) => t.scroll_hit_test(canvas_pos),
            Self::Application(a) => a.scroll_hit_test(canvas_pos),
        }
    }

    pub fn handle_scroll(&mut self, ctx: &egui::Context, scroll_y: f32) {
        match self {
            Self::Terminal(t) => t.handle_scroll(ctx, scroll_y),
            Self::Application(_) => {} // Apps handle their own scroll
        }
    }

    pub fn sync_title(&mut self) {
        match self {
            Self::Terminal(t) => t.sync_title(),
            Self::Application(a) => a.sync_title(),
        }
    }

    pub fn handle_input(&mut self, ctx: &egui::Context) {
        match self {
            Self::Terminal(t) => t.handle_input(ctx),
            Self::Application(_) => {} // Apps handle their own input
        }
    }
}
