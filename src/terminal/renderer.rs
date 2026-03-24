// Terminal renderer — backgrounds in canvas space, text in screen space for crisp zoom.

use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::point_to_viewport;
use alacritty_terminal::term::Term;
use alacritty_terminal::vte::ansi::CursorShape;
use egui::{Color32, FontId, Pos2, Rect, Vec2};
use std::time::Duration;

use crate::terminal::colors::{self, DEFAULT_BG};
use crate::terminal::pty::EventProxy;

pub const FONT_SIZE: f32 = 18.0;
pub const PAD_X: f32 = 10.0;
pub const PAD_Y: f32 = 6.0;
const CELL_WIDTH_ESTIMATE: f32 = FONT_SIZE * 0.6;
const CELL_HEIGHT_ESTIMATE: f32 = FONT_SIZE * 1.25;
const CURSOR_BLINK_ON_SECONDS: f64 = 0.6;
const CURSOR_BLINK_OFF_SECONDS: f64 = 0.4;

#[allow(dead_code)]
pub fn cell_size(ctx: &egui::Context) -> (f32, f32) {
    measure_cell(ctx)
}

fn measure_cell(ctx: &egui::Context) -> (f32, f32) {
    let font = FontId::monospace(FONT_SIZE);
    ctx.fonts(|fonts| {
        let g = fonts.layout_no_wrap("M".to_string(), font, Color32::WHITE);
        (g.rect.width(), g.rect.height())
    })
}

fn grid_size_for_cell_metrics(
    body_width: f32,
    body_height: f32,
    cell_width: f32,
    cell_height: f32,
) -> (usize, usize) {
    let cols = ((body_width - PAD_X * 2.0) / cell_width).floor().max(2.0) as usize;
    let rows = ((body_height - PAD_Y * 2.0) / cell_height).floor().max(1.0) as usize;
    (cols, rows)
}

pub fn compute_grid_size(body_width: f32, body_height: f32) -> (u16, u16) {
    let (cols, rows) = grid_size_for_cell_metrics(
        body_width,
        body_height,
        CELL_WIDTH_ESTIMATE,
        CELL_HEIGHT_ESTIMATE,
    );
    (cols as u16, rows as u16)
}

pub fn compute_grid_size_from_ctx(
    ctx: &egui::Context,
    body_width: f32,
    body_height: f32,
) -> (u16, u16) {
    let (cw, ch) = measure_cell(ctx);
    let (cols, rows) = grid_size_for_cell_metrics(body_width, body_height, cw, ch);
    (cols as u16, rows as u16)
}

/// Render terminal content. Backgrounds in canvas space (GPU scales fine),
/// text + cursor in screen space (crisp at any zoom level).
#[allow(clippy::too_many_arguments)]
pub fn render_terminal(
    ctx: &egui::Context,
    painter: &egui::Painter,
    term: &Term<EventProxy>,
    body_rect: Rect,
    focused: bool,
    hide_cursor_for_output: bool,
    transform: egui::emath::TSTransform,
    screen_clip: Rect,
    panel_id: uuid::Uuid,
) {
    let (cw, ch) = measure_cell(ctx);
    if cw < 1.0 || ch < 1.0 {
        return;
    }

    let zoom = transform.scaling;
    let clipped = painter.with_clip_rect(body_rect);
    let content = term.renderable_content();
    let display_offset = content.display_offset;
    let colors = content.colors;
    let (max_col, max_row) =
        grid_size_for_cell_metrics(body_rect.width(), body_rect.height(), cw, ch);

    // --- Backgrounds (canvas space) ---
    let mut run: Option<(Color32, f32, f32, f32)> = None;
    for indexed in content.display_iter {
        let Some(viewport_point) = point_to_viewport(display_offset, indexed.point) else {
            continue;
        };
        let col = viewport_point.column.0;
        let row = viewport_point.line;
        if row >= max_row || col >= max_col {
            continue;
        }
        let x = body_rect.min.x + PAD_X + col as f32 * cw;
        let y = body_rect.min.y + PAD_Y + row as f32 * ch;
        let cell = &indexed.cell;
        let fl = cell.flags;
        if fl.contains(Flags::WIDE_CHAR_SPACER) {
            if let Some(ref mut r) = run {
                r.3 += cw;
            }
            continue;
        }
        let bg = if fl.contains(Flags::INVERSE) {
            colors::to_egui_color(cell.fg, colors)
        } else {
            colors::to_egui_color(cell.bg, colors)
        };
        let w = if fl.contains(Flags::WIDE_CHAR) {
            cw * 2.0
        } else {
            cw
        };
        if bg != DEFAULT_BG {
            if let Some(ref mut r) = run {
                if r.0 == bg && (r.2 - y).abs() < 0.1 && (r.1 + r.3 - x).abs() < 0.5 {
                    r.3 += w;
                    continue;
                }
                clipped.rect_filled(
                    Rect::from_min_size(Pos2::new(r.1, r.2), Vec2::new(r.3, ch)),
                    0.0,
                    r.0,
                );
            }
            run = Some((bg, x, y, w));
        } else if let Some(r) = run.take() {
            clipped.rect_filled(
                Rect::from_min_size(Pos2::new(r.1, r.2), Vec2::new(r.3, ch)),
                0.0,
                r.0,
            );
        }
    }
    if let Some(r) = run.take() {
        clipped.rect_filled(
            Rect::from_min_size(Pos2::new(r.1, r.2), Vec2::new(r.3, ch)),
            0.0,
            r.0,
        );
    }

    // --- Text (screen space — crisp at any zoom) ---
    let screen_font_size = FONT_SIZE * zoom;
    let screen_font = FontId::monospace(screen_font_size);
    let screen_cw = cw * zoom;
    let screen_ch = ch * zoom;
    let screen_pad_x = PAD_X * zoom;
    let screen_pad_y = PAD_Y * zoom;

    let screen_body = Rect::from_min_max(transform * body_rect.min, transform * body_rect.max)
        .intersect(screen_clip);

    let text_layer = egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("term_text").with(panel_id),
    );
    let text_painter = ctx.layer_painter(text_layer).with_clip_rect(screen_body);
    let screen_body_min = transform * body_rect.min;

    let content2 = term.renderable_content();
    let display_offset = content2.display_offset;
    for indexed in content2.display_iter {
        let Some(viewport_point) = point_to_viewport(display_offset, indexed.point) else {
            continue;
        };
        let col = viewport_point.column.0;
        let row = viewport_point.line;
        if row >= max_row || col >= max_col {
            continue;
        }
        let cell = &indexed.cell;
        let fl = cell.flags;
        if fl.contains(Flags::WIDE_CHAR_SPACER) {
            continue;
        }
        let c = cell.c;
        if c == ' ' || c == '\0' {
            continue;
        }
        let sx = screen_body_min.x + screen_pad_x + col as f32 * screen_cw;
        let sy = screen_body_min.y + screen_pad_y + row as f32 * screen_ch;

        let mut fg = if fl.contains(Flags::INVERSE) {
            colors::to_egui_color(cell.bg, colors)
        } else {
            colors::to_egui_color(cell.fg, colors)
        };
        if fl.contains(Flags::DIM) {
            fg = Color32::from_rgb(
                (fg.r() as f32 * 0.67) as u8,
                (fg.g() as f32 * 0.67) as u8,
                (fg.b() as f32 * 0.67) as u8,
            );
        }
        if fl.contains(Flags::HIDDEN) {
            fg = if fl.contains(Flags::INVERSE) {
                colors::to_egui_color(cell.fg, colors)
            } else {
                colors::to_egui_color(cell.bg, colors)
            };
        }
        if fl.contains(Flags::BOLD) && !fl.contains(Flags::DIM) {
            fg = brighten(fg);
        }
        let is_italic = fl.contains(Flags::ITALIC);
        let text_x = if is_italic { sx + 1.5 * zoom } else { sx };

        text_painter.text(
            Pos2::new(text_x, sy),
            egui::Align2::LEFT_TOP,
            c.to_string(),
            screen_font.clone(),
            fg,
        );
        if fl.contains(Flags::UNDERLINE) {
            let uy = sy + screen_ch - zoom;
            text_painter.line_segment(
                [Pos2::new(sx, uy), Pos2::new(sx + screen_cw, uy)],
                egui::Stroke::new(zoom, fg),
            );
        }
        if fl.contains(Flags::STRIKEOUT) {
            let sy2 = sy + screen_ch * 0.5;
            text_painter.line_segment(
                [Pos2::new(sx, sy2), Pos2::new(sx + screen_cw, sy2)],
                egui::Stroke::new(zoom, fg),
            );
        }
    }

    // --- Cursor (screen space) ---
    let cursor = content2.cursor;
    let cursor_style = term.cursor_style();
    let draw_cursor = should_draw_cursor(
        cursor.shape,
        cursor_style.blinking,
        focused,
        hide_cursor_for_output,
        ctx.input(|i| i.time),
    );
    if draw_cursor {
        if cursor_style.blinking {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
    } else if hide_cursor_for_output {
        ctx.request_repaint_after(Duration::from_millis(250));
    }
    if draw_cursor {
        let Some(cursor_point) = point_to_viewport(display_offset, cursor.point) else {
            return;
        };
        let row = cursor_point.line;
        let col = cursor_point.column.0;
        if row < max_row && col < max_col {
            let cx = screen_body_min.x + screen_pad_x + col as f32 * screen_cw;
            let cy = screen_body_min.y + screen_pad_y + row as f32 * screen_ch;
            let cr = Rect::from_min_size(Pos2::new(cx, cy), Vec2::new(screen_cw, screen_ch));
            let cc = Color32::from_rgb(196, 223, 255);
            match cursor.shape {
                CursorShape::Block => {
                    text_painter.rect_filled(
                        cr,
                        0.0,
                        Color32::from_rgba_premultiplied(cc.r(), cc.g(), cc.b(), 180),
                    );
                }
                CursorShape::Beam => {
                    text_painter.rect_filled(
                        Rect::from_min_size(cr.left_top(), Vec2::new(2.0 * zoom, screen_ch)),
                        0.0,
                        cc,
                    );
                }
                CursorShape::Underline => {
                    text_painter.rect_filled(
                        Rect::from_min_size(
                            Pos2::new(cx, cy + screen_ch - 2.0 * zoom),
                            Vec2::new(screen_cw, 2.0 * zoom),
                        ),
                        0.0,
                        cc,
                    );
                }
                CursorShape::HollowBlock => {
                    text_painter.rect_stroke(cr, 0.0, egui::Stroke::new(zoom, cc));
                }
                CursorShape::Hidden => {}
            }
        }
    }
}

fn should_draw_cursor(
    shape: CursorShape,
    blinking: bool,
    focused: bool,
    hide_cursor_for_output: bool,
    time_seconds: f64,
) -> bool {
    if !focused || shape == CursorShape::Hidden || hide_cursor_for_output {
        return false;
    }
    if !blinking {
        return true;
    }
    blink_phase_visible(time_seconds)
}

fn blink_phase_visible(time_seconds: f64) -> bool {
    let cycle = CURSOR_BLINK_ON_SECONDS + CURSOR_BLINK_OFF_SECONDS;
    (time_seconds % cycle) < CURSOR_BLINK_ON_SECONDS
}

fn brighten(c: Color32) -> Color32 {
    Color32::from_rgb(
        (c.r() as u16 * 4 / 3).min(255) as u8,
        (c.g() as u16 * 4 / 3).min(255) as u8,
        (c.b() as u16 * 4 / 3).min(255) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unfocused_cursor_is_hidden() {
        assert!(!should_draw_cursor(
            CursorShape::Block,
            false,
            false,
            false,
            0.0
        ));
    }

    #[test]
    fn blinking_cursor_turns_off_during_hidden_phase() {
        assert!(blink_phase_visible(0.2));
        assert!(!blink_phase_visible(0.8));
    }

    #[test]
    fn streaming_output_hides_cursor() {
        assert!(!should_draw_cursor(
            CursorShape::Block,
            false,
            true,
            true,
            0.0
        ));
    }
}
