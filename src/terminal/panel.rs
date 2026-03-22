// TerminalPanel — canvas space, TSTransform zoom, full terminal interaction.

use egui::{Color32, Key, Modifiers, Pos2, Rect, Vec2};
use uuid::Uuid;
use crate::terminal::pty::PtyHandle;

const TITLE_BAR_HEIGHT: f32 = 36.0;
const BORDER_RADIUS: f32 = 10.0;
const RESIZE_HANDLE: f32 = 14.0;
const MIN_WIDTH: f32 = 320.0;
const MIN_HEIGHT: f32 = 220.0;

const PANEL_BG: Color32 = Color32::from_rgb(17, 17, 17);
const BORDER_DEFAULT: Color32 = Color32::from_rgb(40, 40, 40);
const BORDER_FOCUS: Color32 = Color32::from_rgb(70, 70, 70);
const FG: Color32 = Color32::from_rgb(200, 200, 200);
const FG_DIM: Color32 = Color32::from_rgb(90, 90, 90);
const SELECTION_BG: Color32 = Color32::from_rgba_premultiplied(80, 130, 200, 80);

pub const VOID_SHORTCUTS: &[(Modifiers, Key)] = &[
    (Modifiers { alt: false, ctrl: true, shift: false, mac_cmd: false, command: false }, Key::B),
    (Modifiers { alt: false, ctrl: true, shift: false, mac_cmd: false, command: false }, Key::M),
    (Modifiers { alt: false, ctrl: true, shift: false, mac_cmd: false, command: false }, Key::G),
    (Modifiers { alt: false, ctrl: true, shift: true,  mac_cmd: false, command: false }, Key::T),
];

pub struct TerminalPanel {
    pub id: Uuid, pub title: String, pub position: Pos2, pub size: Vec2,
    pub color: Color32, pub z_index: u32, pub focused: bool,
    pty: Option<PtyHandle>, last_cols: u16, last_rows: u16,
    scroll_accum: f32,
    // Selection state (cell coordinates)
    selection: Option<(usize, usize, usize, usize)>, // (start_col, start_row, end_col, end_row)
    selecting: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelAction { Close, Rename }

#[derive(Default)]
pub struct PanelInteraction {
    pub clicked: bool, pub dragging_title: bool, pub drag_delta: Vec2,
    pub resizing: bool, pub resize_delta: Vec2, pub action: Option<PanelAction>,
}

impl TerminalPanel {
    pub fn new_with_terminal(ctx: &egui::Context, position: Pos2, size: Vec2, color: Color32, cwd: Option<&std::path::Path>) -> Self {
        let (cols, rows) = crate::terminal::renderer::compute_grid_size(size.x, size.y - TITLE_BAR_HEIGHT);
        let title = cwd.and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(default_shell_title);
        let pty = PtyHandle::spawn(ctx, rows, cols, &title, cwd).map_err(|e| log::error!("Failed to spawn terminal: {e}")).ok();
        Self { id: Uuid::new_v4(), title, position, size, color, z_index: 0, focused: false, pty,
               last_cols: cols, last_rows: rows, scroll_accum: 0.0, selection: None, selecting: false }
    }

    #[allow(dead_code)]
    pub fn new(title: impl Into<String>, position: Pos2, size: Vec2, color: Color32) -> Self {
        Self { id: Uuid::new_v4(), title: title.into(), position, size, color, z_index: 0, focused: false,
               pty: None, last_cols: 80, last_rows: 24, scroll_accum: 0.0, selection: None, selecting: false }
    }

    pub fn rect(&self) -> Rect { Rect::from_min_size(self.position, self.size) }
    pub fn is_alive(&self) -> bool { self.pty.as_ref().map_or(false, |p| p.is_alive()) }

    pub fn handle_input(&self, ctx: &egui::Context) {
        if !self.focused { return; }
        if let Some(pty) = &self.pty {
            let bytes = crate::terminal::input::process_input(ctx, VOID_SHORTCUTS);
            if !bytes.is_empty() { pty.write(&bytes); }
        }
    }

    pub fn check_resize(&mut self) {
        if let Some(pty) = &self.pty {
            let (cols, rows) = crate::terminal::renderer::compute_grid_size(self.size.x, self.size.y - TITLE_BAR_HEIGHT);
            if cols != self.last_cols || rows != self.last_rows {
                self.last_cols = cols; self.last_rows = rows; pty.resize(rows, cols);
            }
        }
    }

    pub fn sync_title(&mut self) {
        if let Some(pty) = &self.pty {
            if let Ok(t) = pty.title.lock() { if *t != self.title { self.title = t.clone(); } }
        }
    }

    /// Convert pointer position to terminal cell (col, row), clamped to grid bounds.
    fn pos_to_cell(&self, pos: Pos2, body: Rect, ctx: &egui::Context) -> (usize, usize) {
        let (cw, ch) = crate::terminal::renderer::cell_size(ctx);
        let col = ((pos.x - body.min.x - crate::terminal::renderer::PAD_X) / cw).floor().clamp(0.0, (self.last_cols as f32) - 1.0) as usize;
        let row = ((pos.y - body.min.y - crate::terminal::renderer::PAD_Y) / ch).floor().clamp(0.0, (self.last_rows as f32) - 1.0) as usize;
        (col, row)
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> PanelInteraction {
        let mut ix = PanelInteraction::default();
        let pr = self.rect();
        let painter = ui.painter();
        let border_color = if self.focused { BORDER_FOCUS } else { BORDER_DEFAULT };

        // ========== PAINT CHROME ==========

        painter.rect_filled(pr.translate(Vec2::new(0.0, 3.0)).expand(2.0),
            BORDER_RADIUS + 2.0, Color32::from_rgba_premultiplied(0, 0, 0, 35));
        painter.rect_filled(pr, BORDER_RADIUS, PANEL_BG);
        painter.rect_stroke(pr, BORDER_RADIUS, egui::Stroke::new(1.0, border_color));

        let sep_y = pr.min.y + TITLE_BAR_HEIGHT;
        painter.line_segment(
            [Pos2::new(pr.min.x + 8.0, sep_y), Pos2::new(pr.max.x - 8.0, sep_y)],
            egui::Stroke::new(0.5, Color32::from_rgb(40, 40, 40)));

        let dot_x = pr.min.x + 12.0;
        let dot_y = pr.min.y + TITLE_BAR_HEIGHT * 0.5;
        painter.circle_filled(Pos2::new(dot_x, dot_y), 3.0,
            if self.focused { self.color } else { self.color.linear_multiply(0.4) });
        let status = if self.pty.is_some() { if self.is_alive() { "" } else { "exited · " } } else { "" };
        let tc = if self.is_alive() || self.pty.is_none() { FG } else { FG_DIM };
        painter.text(Pos2::new(dot_x + 10.0, dot_y - 6.0), egui::Align2::LEFT_TOP,
            format!("{}{}", status, self.title), egui::FontId::proportional(12.0), tc);

        let close_center = Pos2::new(pr.max.x - 14.0, dot_y);
        let close_rect = Rect::from_center_size(close_center, Vec2::splat(16.0));
        let close_resp = ui.interact(close_rect, egui::Id::new(self.id).with("close"), egui::Sense::click());
        if close_resp.hovered() {
            painter.circle_filled(close_center, 4.0, Color32::from_rgb(200, 60, 60));
            let s = egui::Stroke::new(1.0, Color32::WHITE);
            painter.line_segment([Pos2::new(close_center.x-1.5, close_center.y-1.5), Pos2::new(close_center.x+1.5, close_center.y+1.5)], s);
            painter.line_segment([Pos2::new(close_center.x+1.5, close_center.y-1.5), Pos2::new(close_center.x-1.5, close_center.y+1.5)], s);
        }
        if close_resp.clicked() { ix.action = Some(PanelAction::Close); }

        // ========== TERMINAL CONTENT ==========

        let body = Rect::from_min_max(
            Pos2::new(pr.min.x + 1.0, sep_y + 1.0),
            Pos2::new(pr.max.x - 1.0, pr.max.y - 1.0));

        if let Some(pty) = &self.pty {
            if let Ok(term) = pty.term.lock() {
                crate::terminal::renderer::render_terminal(ui.ctx(), painter, &term, body);
            }
        }

        // Draw selection highlight (clipped to body)
        if let Some((sc, sr, ec, er)) = self.selection {
            let (cw, ch) = crate::terminal::renderer::cell_size(ui.ctx());
            let pad_x = crate::terminal::renderer::PAD_X;
            let pad_y = crate::terminal::renderer::PAD_Y;
            let max_col = self.last_cols as usize;
            let max_row = self.last_rows as usize;

            let (start_row, start_col, end_row, end_col) = if sr < er || (sr == er && sc <= ec) {
                (sr.min(max_row.saturating_sub(1)), sc.min(max_col.saturating_sub(1)),
                 er.min(max_row.saturating_sub(1)), ec.min(max_col.saturating_sub(1)))
            } else {
                (er.min(max_row.saturating_sub(1)), ec.min(max_col.saturating_sub(1)),
                 sr.min(max_row.saturating_sub(1)), sc.min(max_col.saturating_sub(1)))
            };

            let clipped = painter.with_clip_rect(body);
            for row in start_row..=end_row {
                let c0 = if row == start_row { start_col } else { 0 };
                let c1 = (if row == end_row { end_col + 1 } else { max_col }).min(max_col);
                let x0 = body.min.x + pad_x + c0 as f32 * cw;
                let y0 = body.min.y + pad_y + row as f32 * ch;
                let x1 = body.min.x + pad_x + c1 as f32 * cw;
                let sel_rect = Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y0 + ch));
                clipped.rect_filled(sel_rect, 0.0, SELECTION_BG);
            }
        }

        // ========== INTERACTIONS ==========

        // Body: click + drag (for selection) + focus
        let body_resp = ui.interact(body, egui::Id::new(self.id).with("body"), egui::Sense::click_and_drag());

        if body_resp.clicked_by(egui::PointerButton::Primary) {
            ix.clicked = true;
            self.selection = None; // Clear selection on click
        }

        // Text selection via drag
        if body_resp.drag_started_by(egui::PointerButton::Primary) {
            ix.clicked = true;
            if let Some(pos) = body_resp.interact_pointer_pos() {
                // interact_pointer_pos is already in canvas space (egui applies inverse transform)
                let (col, row) = self.pos_to_cell(pos, body, ui.ctx());
                self.selection = Some((col, row, col, row));
                self.selecting = true;
            }
        }
        if self.selecting && body_resp.dragged_by(egui::PointerButton::Primary) {
            // Use hover_pos() from the response — it's in canvas space (transformed)
            if let Some(pos) = body_resp.hover_pos() {
                let (col, row) = self.pos_to_cell(pos, body, ui.ctx());
                if let Some(ref mut sel) = self.selection {
                    sel.2 = col;
                    sel.3 = row;
                }
            } else if let Some(pos) = body_resp.interact_pointer_pos() {
                // Fallback: interact_pointer_pos also in canvas space
                let (col, row) = self.pos_to_cell(pos, body, ui.ctx());
                if let Some(ref mut sel) = self.selection {
                    sel.2 = col;
                    sel.3 = row;
                }
            }
        }
        if body_resp.drag_stopped() {
            self.selecting = false;
            // Copy selection to clipboard
            if let Some((sc, sr, ec, er)) = self.selection {
                if let Some(pty) = &self.pty {
                    if let Ok(term) = pty.term.lock() {
                        let text = extract_selection_text(&term, sc, sr, ec, er);
                        if !text.is_empty() {
                            ui.output_mut(|o| o.copied_text = text);
                        }
                    }
                }
            }
        }

        // Scroll with accumulation — if focused, always process scroll
        // (canvas pan is already blocked by terminal_has_scroll in app.rs)
        if self.focused {
            if let Some(pty) = &self.pty {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0.0 {
                    self.scroll_accum += scroll;
                    let line_height = 20.0; // approximate pixels per terminal line
                    let lines = (self.scroll_accum / line_height) as i32;
                    if lines != 0 {
                        self.scroll_accum -= lines as f32 * line_height;
                        use alacritty_terminal::grid::Dimensions;
                        if let Ok(mut term) = pty.term.lock() {
                            let display = term.grid().display_offset();
                            let max = term.grid().history_size();
                            let new = (display as i32 + lines).clamp(0, max as i32) as usize;
                            term.grid_mut().scroll_display(
                                alacritty_terminal::grid::Scroll::Delta(new as i32 - display as i32));
                        }
                    }
                }
            }
        }

        // Title: drag to move, click to focus
        let tb = Rect::from_min_size(pr.min, Vec2::new(pr.width(), TITLE_BAR_HEIGHT));
        let title_resp = ui.interact(tb, egui::Id::new(self.id).with("title"), egui::Sense::click_and_drag());
        if title_resp.clicked_by(egui::PointerButton::Primary) { ix.clicked = true; }
        if title_resp.dragged_by(egui::PointerButton::Primary) {
            ix.dragging_title = true;
            ix.drag_delta = title_resp.drag_delta();
            ix.clicked = true;
        }

        // Resize handles
        let rbr = Rect::from_min_size(Pos2::new(pr.max.x-RESIZE_HANDLE, pr.max.y-RESIZE_HANDLE), Vec2::splat(RESIZE_HANDLE));
        let rr = Rect::from_min_size(Pos2::new(pr.max.x-RESIZE_HANDLE, pr.min.y+TITLE_BAR_HEIGHT), Vec2::new(RESIZE_HANDLE, pr.height()-TITLE_BAR_HEIGHT-RESIZE_HANDLE));
        let rb = Rect::from_min_size(Pos2::new(pr.min.x, pr.max.y-RESIZE_HANDLE), Vec2::new(pr.width()-RESIZE_HANDLE, RESIZE_HANDLE));

        let brr = ui.interact(rbr, egui::Id::new(self.id).with("rbr"), egui::Sense::drag());
        if brr.dragged() { ix.resizing = true; ix.resize_delta = brr.drag_delta(); }
        if !ix.resizing {
            let rrs = ui.interact(rr, egui::Id::new(self.id).with("rr"), egui::Sense::drag());
            if rrs.dragged() { ix.resizing = true; ix.resize_delta = Vec2::new(rrs.drag_delta().x, 0.0); }
        }
        if !ix.resizing {
            let rbs = ui.interact(rb, egui::Id::new(self.id).with("rb"), egui::Sense::drag());
            if rbs.dragged() { ix.resizing = true; ix.resize_delta = Vec2::new(0.0, rbs.drag_delta().y); }
        }

        let hp = ui.input(|i| i.pointer.hover_pos().unwrap_or_default());
        if rbr.contains(hp) { ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe); }
        else if rr.contains(hp) { ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal); }
        else if rb.contains(hp) { ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical); }

        body_resp.context_menu(|ui| {
            if ui.button("Rename").clicked() { ix.action = Some(PanelAction::Rename); ui.close_menu(); }
            if ui.button("Close").clicked() { ix.action = Some(PanelAction::Close); ui.close_menu(); }
        });

        ix
    }

    pub fn apply_drag(&mut self, delta: Vec2) { self.position += delta; }
    pub fn apply_resize(&mut self, delta: Vec2) {
        self.size = Vec2::new((self.size.x + delta.x).max(MIN_WIDTH), (self.size.y + delta.y).max(MIN_HEIGHT));
        self.check_resize();
    }
}

/// Extract text from terminal grid within a cell range.
fn extract_selection_text(
    term: &alacritty_terminal::Term<crate::terminal::pty::EventProxy>,
    sc: usize, sr: usize, ec: usize, er: usize,
) -> String {
    use alacritty_terminal::index::{Column, Line};
    use alacritty_terminal::grid::Dimensions;

    let (start_row, start_col, end_row, end_col) = if sr < er || (sr == er && sc <= ec) {
        (sr, sc, er, ec)
    } else {
        (er, ec, sr, sc)
    };

    let mut text = String::new();
    let cols = term.columns();

    for row in start_row..=end_row {
        let c0 = if row == start_row { start_col } else { 0 };
        let c1 = if row == end_row { end_col + 1 } else { cols };
        let c1 = c1.min(cols);

        for col in c0..c1 {
            let cell = &term.grid()[Line(row as i32)][Column(col)];
            text.push(cell.c);
        }

        // Trim trailing spaces on each line and add newline
        if row < end_row {
            let trimmed = text.trim_end_matches(' ');
            text.truncate(trimmed.len());
            text.push('\n');
        }
    }

    text.trim_end().to_string()
}

fn default_shell_title() -> String {
    #[cfg(windows)]
    { std::env::var("COMSPEC").map(|s| std::path::Path::new(&s).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or(s)).unwrap_or_else(|_| "cmd.exe".to_string()) }
    #[cfg(not(windows))]
    { std::env::var("SHELL").map(|s| std::path::Path::new(&s).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or(s)).unwrap_or_else(|_| "sh".to_string()) }
}
