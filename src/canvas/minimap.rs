// Minimap widget: small overlay showing all panels and current viewport

use egui::{Color32, Pos2, Rect, Vec2, Ui};
use crate::canvas::viewport::Viewport;
use crate::terminal::panel::TerminalPanel;

const MINIMAP_WIDTH: f32 = 200.0;
const MINIMAP_HEIGHT: f32 = 150.0;
const MINIMAP_PADDING: f32 = 10.0;
const MINIMAP_BG: Color32 = Color32::from_rgba_premultiplied(15, 15, 15, 200);
const VIEWPORT_BORDER: Color32 = Color32::from_rgb(100, 100, 100);

/// Render the minimap overlay in the bottom-right corner.
/// Returns Some(canvas_pos) if the user clicked the minimap to navigate.
pub fn draw_minimap(
    ui: &Ui,
    viewport: &Viewport,
    screen_rect: Rect,
    panels: &[TerminalPanel],
) -> Option<Pos2> {
    if panels.is_empty() {
        return None;
    }

    let painter = ui.painter_at(screen_rect);

    // Minimap position: bottom-right corner
    let minimap_rect = Rect::from_min_size(
        Pos2::new(
            screen_rect.max.x - MINIMAP_WIDTH - MINIMAP_PADDING,
            screen_rect.max.y - MINIMAP_HEIGHT - MINIMAP_PADDING,
        ),
        Vec2::new(MINIMAP_WIDTH, MINIMAP_HEIGHT),
    );

    // Background
    painter.rect_filled(minimap_rect, 4.0, MINIMAP_BG);
    painter.rect_stroke(minimap_rect, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(40, 40, 40)));

    // Compute bounding box of all panels in canvas space
    let mut bounds_min = Pos2::new(f32::MAX, f32::MAX);
    let mut bounds_max = Pos2::new(f32::MIN, f32::MIN);

    for panel in panels {
        let r = panel.rect();
        bounds_min.x = bounds_min.x.min(r.min.x);
        bounds_min.y = bounds_min.y.min(r.min.y);
        bounds_max.x = bounds_max.x.max(r.max.x);
        bounds_max.y = bounds_max.y.max(r.max.y);
    }

    // Add some padding around bounds and include the viewport
    let visible = viewport.visible_canvas_rect(screen_rect);
    bounds_min.x = bounds_min.x.min(visible.min.x);
    bounds_min.y = bounds_min.y.min(visible.min.y);
    bounds_max.x = bounds_max.x.max(visible.max.x);
    bounds_max.y = bounds_max.y.max(visible.max.y);

    let padding = 100.0;
    bounds_min -= Vec2::splat(padding);
    bounds_max += Vec2::splat(padding);

    let canvas_range = bounds_max - bounds_min;
    if canvas_range.x <= 0.0 || canvas_range.y <= 0.0 {
        return None;
    }

    // Fit the canvas bounds into the minimap rect with aspect ratio preserved
    let inner = minimap_rect.shrink(6.0);
    let scale_x = inner.width() / canvas_range.x;
    let scale_y = inner.height() / canvas_range.y;
    let scale = scale_x.min(scale_y);

    let map_offset = Vec2::new(
        inner.min.x + (inner.width() - canvas_range.x * scale) * 0.5,
        inner.min.y + (inner.height() - canvas_range.y * scale) * 0.5,
    );

    let canvas_to_minimap = |p: Pos2| -> Pos2 {
        Pos2::new(
            (p.x - bounds_min.x) * scale + map_offset.x,
            (p.y - bounds_min.y) * scale + map_offset.y,
        )
    };

    // Draw panels as colored rectangles
    for panel in panels {
        let r = panel.rect();
        let mini_min = canvas_to_minimap(r.min);
        let mini_max = canvas_to_minimap(r.max);
        let mini_rect = Rect::from_min_max(mini_min, mini_max);

        // Use panel color but slightly dimmed
        let color = panel.color.linear_multiply(0.7);
        painter.rect_filled(mini_rect, 1.0, color);
    }

    // Draw current viewport rectangle
    let vp_min = canvas_to_minimap(visible.min);
    let vp_max = canvas_to_minimap(visible.max);
    let vp_rect = Rect::from_min_max(vp_min, vp_max);
    painter.rect_stroke(
        vp_rect,
        1.0,
        egui::Stroke::new(1.5, VIEWPORT_BORDER),
    );

    // Zoom label
    let zoom_text = format!("{:.0}%", viewport.zoom * 100.0);
    painter.text(
        Pos2::new(minimap_rect.center().x, minimap_rect.max.y - 4.0),
        egui::Align2::CENTER_BOTTOM,
        zoom_text,
        egui::FontId::proportional(10.0),
        Color32::from_rgb(120, 120, 120),
    );

    // Handle click-to-navigate on minimap
    let minimap_response = ui.interact(
        minimap_rect,
        egui::Id::new("minimap_interact"),
        egui::Sense::click_and_drag(),
    );

    if minimap_response.clicked() || minimap_response.dragged() {
        if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
            if minimap_rect.contains(pos) {
                // Convert minimap position back to canvas coordinates
                let canvas_x = (pos.x - map_offset.x) / scale + bounds_min.x;
                let canvas_y = (pos.y - map_offset.y) / scale + bounds_min.y;
                return Some(Pos2::new(canvas_x, canvas_y));
            }
        }
    }

    None
}
