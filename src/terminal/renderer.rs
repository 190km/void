// Terminal renderer — renders at fixed font size in canvas space.
// Uses painter_at() to clip text to the panel body (no overflow).
// GPU TSTransform handles zoom scaling of the rasterized glyphs.

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

/// Public cell size for mouse coordinate mapping.
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

/// Render terminal in canvas space, clipped to body_rect.
pub fn render_terminal(
    ctx: &egui::Context,
    painter: &egui::Painter,
    term: &Term<EventProxy>,
    body_rect: Rect,
    focused: bool,
    hide_cursor_for_output: bool,
) {
    let font = FontId::monospace(FONT_SIZE);
    let (cw, ch) = measure_cell(ctx);
    if cw < 1.0 || ch < 1.0 {
        return;
    }

    // Use painter_at to CLIP everything to the body rect — no text overflow
    let clipped = painter.with_clip_rect(body_rect);

    let content = term.renderable_content();
    let display_offset = content.display_offset;
    let colors = content.colors;

    // Keep the rendered viewport aligned with the PTY grid sizing.
    let (max_col, max_row) =
        grid_size_for_cell_metrics(body_rect.width(), body_rect.height(), cw, ch);

    // --- Backgrounds ---
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

    // --- Text ---
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

        let x = body_rect.min.x + PAD_X + col as f32 * cw;
        let y = body_rect.min.y + PAD_Y + row as f32 * ch;

        let cell = &indexed.cell;
        let fl = cell.flags;
        if fl.contains(Flags::WIDE_CHAR_SPACER) {
            continue;
        }
        let c = cell.c;
        if c == ' ' || c == '\0' {
            continue;
        }

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

        // Italic: faux-italic via slight x-offset on top (skew effect)
        let is_italic = fl.contains(Flags::ITALIC);
        let text_x = if is_italic { x + 1.5 } else { x };

        clipped.text(
            Pos2::new(text_x, y),
            egui::Align2::LEFT_TOP,
            c.to_string(),
            font.clone(),
            fg,
        );

        // Underline decoration
        if fl.contains(Flags::UNDERLINE) {
            let underline_y = y + ch - 1.0;
            clipped.line_segment(
                [Pos2::new(x, underline_y), Pos2::new(x + cw, underline_y)],
                egui::Stroke::new(1.0, fg),
            );
        }

        // Strikethrough decoration
        if fl.contains(Flags::STRIKEOUT) {
            let strike_y = y + ch * 0.5;
            clipped.line_segment(
                [Pos2::new(x, strike_y), Pos2::new(x + cw, strike_y)],
                egui::Stroke::new(1.0, fg),
            );
        }
    }

    // --- Cursor ---
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
            let cx = body_rect.min.x + PAD_X + col as f32 * cw;
            let cy = body_rect.min.y + PAD_Y + row as f32 * ch;
            let cr = Rect::from_min_size(Pos2::new(cx, cy), Vec2::new(cw, ch));
            let cc = Color32::from_rgb(196, 223, 255);
            match cursor.shape {
                CursorShape::Block => {
                    clipped.rect_filled(
                        cr,
                        0.0,
                        Color32::from_rgba_premultiplied(cc.r(), cc.g(), cc.b(), 180),
                    );
                }
                CursorShape::Beam => {
                    clipped.rect_filled(
                        Rect::from_min_size(cr.left_top(), Vec2::new(2.0, ch)),
                        0.0,
                        cc,
                    );
                }
                CursorShape::Underline => {
                    clipped.rect_filled(
                        Rect::from_min_size(Pos2::new(cx, cy + ch - 2.0), Vec2::new(cw, 2.0)),
                        0.0,
                        cc,
                    );
                }
                CursorShape::HollowBlock => {
                    clipped.rect_stroke(cr, 0.0, egui::Stroke::new(1.0, cc));
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
