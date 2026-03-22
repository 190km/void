// Canvas input: pan + zoom (inspired by Horizon's approach)

use super::viewport::Viewport;
use egui::{self, Ui};

/// Handle canvas pan/zoom input.
/// `terminal_hovered` = true when the pointer is over a terminal, so wheel input should not pan the canvas.
pub fn handle_canvas_input(
    ui: &Ui,
    viewport: &mut Viewport,
    screen_rect: egui::Rect,
    terminal_hovered: bool,
) {
    let ctx = ui.ctx();

    ctx.input(|input| {
        // --- Pan: middle mouse drag ---
        if input.pointer.middle_down() {
            let delta = input.pointer.delta();
            if delta.length() > 0.0 {
                viewport.pan += delta;
            }
        }

        // --- Zoom: Ctrl + scroll OR trackpad pinch ---
        // egui's zoom_delta() captures pinch gestures and Ctrl+scroll
        let zoom_delta = input.zoom_delta();
        if zoom_delta != 1.0 {
            if let Some(pointer_pos) = input.pointer.hover_pos() {
                if screen_rect.contains(pointer_pos) {
                    viewport.zoom_around(pointer_pos, screen_rect, zoom_delta);
                }
            }
        }

        // --- Scroll to pan (when not zooming and no terminal is scrolling) ---
        if zoom_delta == 1.0 && !input.modifiers.ctrl && !terminal_hovered {
            let scroll = input.smooth_scroll_delta;
            if scroll.length() > 0.0 {
                viewport.pan += scroll;
            }
        }

        // --- Zoom: keyboard Ctrl+= / Ctrl+- / Ctrl+0 ---
        if input.modifiers.ctrl {
            if input.key_pressed(egui::Key::Equals) || input.key_pressed(egui::Key::Plus) {
                let center = screen_rect.center();
                viewport.zoom_around(center, screen_rect, 1.15);
            }
            if input.key_pressed(egui::Key::Minus) {
                let center = screen_rect.center();
                viewport.zoom_around(center, screen_rect, 1.0 / 1.15);
            }
            if input.key_pressed(egui::Key::Num0) {
                viewport.zoom = 1.0;
                viewport.pan = egui::Vec2::ZERO;
            }
        }
    });
}
