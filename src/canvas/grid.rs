// Background dot grid

use super::config::{DOT_RADIUS, GRID_COLOR, GRID_SPACING};
use super::viewport::Viewport;
use egui::{Pos2, Rect, Ui};

pub fn draw_dot_grid(ui: &Ui, viewport: &Viewport, screen_rect: Rect) {
    let painter = ui.painter_at(screen_rect);
    let visible = viewport.visible_canvas_rect(screen_rect);

    let start_x = (visible.min.x / GRID_SPACING).floor() as i32;
    let end_x = (visible.max.x / GRID_SPACING).ceil() as i32;
    let start_y = (visible.min.y / GRID_SPACING).floor() as i32;
    let end_y = (visible.max.y / GRID_SPACING).ceil() as i32;

    let count = ((end_x - start_x) as i64) * ((end_y - start_y) as i64);
    if count > 15_000 {
        return;
    }

    let dot_size = (DOT_RADIUS * viewport.zoom).clamp(0.3, 2.0);

    for gx in start_x..=end_x {
        for gy in start_y..=end_y {
            let canvas_pos = Pos2::new(gx as f32 * GRID_SPACING, gy as f32 * GRID_SPACING);
            let screen_pos = viewport.canvas_to_screen(canvas_pos, screen_rect);
            if screen_rect.contains(screen_pos) {
                painter.circle_filled(screen_pos, dot_size, GRID_COLOR);
            }
        }
    }
}
