// TerminalPanel — canvas space, TSTransform zoom, full terminal interaction.

use crate::terminal::pty::PtyHandle;
use alacritty_terminal::term::TermMode;
use egui::{Color32, Key, Modifiers, Pos2, Rect, Vec2};
use uuid::Uuid;

const TITLE_BAR_HEIGHT: f32 = 36.0;
const BORDER_RADIUS: f32 = 10.0;
const RESIZE_HANDLE: f32 = 14.0;
const MIN_WIDTH: f32 = 320.0;
const MIN_HEIGHT: f32 = 220.0;
const SCROLLBAR_WIDTH: f32 = 8.0;
const SCROLLBAR_GAP: f32 = 8.0;
const SCROLLBAR_MARGIN: f32 = 6.0;
const SCROLLBAR_MIN_THUMB_HEIGHT: f32 = 28.0;

const PANEL_BG: Color32 = Color32::from_rgb(17, 17, 17);
const BORDER_DEFAULT: Color32 = Color32::from_rgb(40, 40, 40);
const BORDER_FOCUS: Color32 = Color32::from_rgb(70, 70, 70);
const FG: Color32 = Color32::from_rgb(200, 200, 200);
const FG_DIM: Color32 = Color32::from_rgb(90, 90, 90);
const SELECTION_BG: Color32 = Color32::from_rgba_premultiplied(80, 130, 200, 80);
const SCROLLBAR_TRACK: Color32 = Color32::from_rgb(24, 24, 24);
const SCROLLBAR_THUMB: Color32 = Color32::from_rgb(78, 78, 78);
const SCROLLBAR_THUMB_HOVER: Color32 = Color32::from_rgb(110, 110, 110);

pub const VOID_SHORTCUTS: &[(Modifiers, Key)] = &[
    (
        Modifiers {
            alt: false,
            ctrl: true,
            shift: false,
            mac_cmd: false,
            command: false,
        },
        Key::B,
    ),
    (
        Modifiers {
            alt: false,
            ctrl: true,
            shift: false,
            mac_cmd: false,
            command: false,
        },
        Key::M,
    ),
    (
        Modifiers {
            alt: false,
            ctrl: true,
            shift: false,
            mac_cmd: false,
            command: false,
        },
        Key::G,
    ),
    (
        Modifiers {
            alt: false,
            ctrl: true,
            shift: true,
            mac_cmd: false,
            command: false,
        },
        Key::T,
    ),
];

pub struct TerminalPanel {
    pub id: Uuid,
    pub title: String,
    pub position: Pos2,
    pub size: Vec2,
    pub color: Color32,
    pub z_index: u32,
    pub focused: bool,
    pty: Option<PtyHandle>,
    last_cols: u16,
    last_rows: u16,
    spawn_error: Option<String>,
    // Selection state (cell coordinates)
    selection: Option<(usize, usize, usize, usize)>, // (start_col, start_row, end_col, end_row)
    selecting: bool,
    scroll_remainder: f32,
    scrollbar_grab_offset: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelAction {
    Close,
    Rename,
}

#[derive(Default)]
pub struct PanelInteraction {
    pub clicked: bool,
    pub dragging_title: bool,
    pub drag_delta: Vec2,
    pub resizing: bool,
    pub resize_delta: Vec2,
    pub action: Option<PanelAction>,
}

#[derive(Clone, Copy, Debug)]
struct ScrollbarState {
    history_size: usize,
    display_offset: usize,
    screen_lines: usize,
}

impl ScrollbarState {
    fn has_history(self) -> bool {
        self.history_size > 0
    }

    fn total_lines(self) -> usize {
        self.history_size + self.screen_lines
    }

    fn thumb_height(self, track: Rect) -> f32 {
        if self.total_lines() == 0 {
            return track.height();
        }

        (track.height() * (self.screen_lines as f32 / self.total_lines() as f32))
            .clamp(SCROLLBAR_MIN_THUMB_HEIGHT, track.height())
    }

    fn thumb_rect(self, track: Rect) -> Rect {
        let thumb_height = self.thumb_height(track);
        let travel = (track.height() - thumb_height).max(0.0);
        let ratio = if self.history_size == 0 {
            1.0
        } else {
            1.0 - self.display_offset as f32 / self.history_size as f32
        };
        let top = track.top() + travel * ratio.clamp(0.0, 1.0);
        Rect::from_min_max(
            Pos2::new(track.left(), top),
            Pos2::new(track.right(), top + thumb_height),
        )
    }

    fn offset_for_thumb_top(self, track: Rect, thumb_height: f32, thumb_top: f32) -> usize {
        if self.history_size == 0 {
            return 0;
        }

        let travel = (track.height() - thumb_height).max(0.0);
        if travel <= f32::EPSILON {
            return self.history_size;
        }

        let ratio = ((thumb_top - track.top()) / travel).clamp(0.0, 1.0);
        ((1.0 - ratio) * self.history_size as f32).round() as usize
    }
}

impl TerminalPanel {
    pub fn new_with_terminal(
        ctx: &egui::Context,
        position: Pos2,
        size: Vec2,
        color: Color32,
        cwd: Option<&std::path::Path>,
    ) -> Self {
        let content_size = Self::terminal_content_size(size);
        let (cols, rows) =
            crate::terminal::renderer::compute_grid_size(content_size.x, content_size.y);
        let mut title = cwd
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(default_shell_title);
        let (pty, spawn_error) = match PtyHandle::spawn(ctx, rows, cols, &title, cwd) {
            Ok(pty) => (Some(pty), None),
            Err(err) => {
                let message = err.to_string();
                log::error!("Failed to spawn terminal: {message}");
                title = format!("spawn failed: {title}");
                (None, Some(message))
            }
        };
        Self {
            id: Uuid::new_v4(),
            title,
            position,
            size,
            color,
            z_index: 0,
            focused: false,
            pty,
            last_cols: cols,
            last_rows: rows,
            spawn_error,
            selection: None,
            selecting: false,
            scroll_remainder: 0.0,
            scrollbar_grab_offset: None,
        }
    }

    #[allow(dead_code)]
    pub fn new(title: impl Into<String>, position: Pos2, size: Vec2, color: Color32) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            position,
            size,
            color,
            z_index: 0,
            focused: false,
            pty: None,
            last_cols: 80,
            last_rows: 24,
            spawn_error: None,
            selection: None,
            selecting: false,
            scroll_remainder: 0.0,
            scrollbar_grab_offset: None,
        }
    }

    pub fn rect(&self) -> Rect {
        Rect::from_min_size(self.position, self.size)
    }
    pub fn is_alive(&self) -> bool {
        self.pty.as_ref().is_some_and(|p| p.is_alive())
    }

    fn terminal_body_rect(panel_rect: Rect) -> Rect {
        Rect::from_min_max(
            Pos2::new(
                panel_rect.min.x + 1.0,
                panel_rect.min.y + TITLE_BAR_HEIGHT + 1.0,
            ),
            Pos2::new(panel_rect.max.x - 1.0, panel_rect.max.y - 1.0),
        )
    }

    fn scrollbar_track_rect(body: Rect) -> Rect {
        Rect::from_min_max(
            Pos2::new(
                body.max.x - SCROLLBAR_MARGIN - SCROLLBAR_WIDTH,
                body.min.y + SCROLLBAR_MARGIN,
            ),
            Pos2::new(body.max.x - SCROLLBAR_MARGIN, body.max.y - SCROLLBAR_MARGIN),
        )
    }

    fn terminal_content_rect(body: Rect) -> Rect {
        let track = Self::scrollbar_track_rect(body);
        Rect::from_min_max(body.min, Pos2::new(track.min.x - SCROLLBAR_GAP, body.max.y))
    }

    fn terminal_content_size(size: Vec2) -> Vec2 {
        let body_width = (size.x - 2.0).max(1.0);
        let body_height = (size.y - TITLE_BAR_HEIGHT - 2.0).max(1.0);
        let content_width =
            (body_width - SCROLLBAR_MARGIN * 2.0 - SCROLLBAR_WIDTH - SCROLLBAR_GAP).max(1.0);
        Vec2::new(content_width, body_height)
    }

    fn input_mode(&self) -> crate::terminal::input::InputMode {
        let Some(pty) = &self.pty else {
            return crate::terminal::input::InputMode::default();
        };
        let Ok(term) = pty.term.lock() else {
            return crate::terminal::input::InputMode::default();
        };

        let mode = *term.mode();
        crate::terminal::input::InputMode {
            app_cursor: mode.contains(TermMode::APP_CURSOR),
            bracketed_paste: mode.contains(TermMode::BRACKETED_PASTE),
        }
    }

    fn local_interactions_enabled(&self) -> bool {
        let Some(pty) = &self.pty else {
            return true;
        };
        let Ok(term) = pty.term.lock() else {
            return true;
        };

        let mode = *term.mode();
        !mode.intersects(TermMode::ALT_SCREEN | TermMode::MOUSE_MODE)
    }

    pub fn scroll_hit_test(&self, canvas_pos: Pos2) -> bool {
        Self::terminal_body_rect(self.rect()).contains(canvas_pos)
    }

    pub fn handle_scroll(&mut self, ctx: &egui::Context, scroll_y: f32) {
        let Some(pty) = &self.pty else {
            return;
        };
        if !self.local_interactions_enabled() {
            return;
        }
        if scroll_y == 0.0 {
            return;
        }

        let (_, row_height) = crate::terminal::renderer::cell_size(ctx);
        let row_height = row_height.max(1.0);

        self.scroll_remainder += scroll_y;
        let lines = (self.scroll_remainder / row_height).trunc() as i32;
        if lines == 0 {
            return;
        }

        self.scroll_remainder -= lines as f32 * row_height;

        if let Ok(mut term) = pty.term.lock() {
            term.scroll_display(alacritty_terminal::grid::Scroll::Delta(lines));
        }
    }

    fn scroll_to_offset(&mut self, offset: usize) {
        let Some(pty) = &self.pty else {
            return;
        };

        if let Ok(mut term) = pty.term.lock() {
            use alacritty_terminal::grid::Dimensions;

            let target = offset.min(term.grid().history_size());
            let current = term.grid().display_offset();
            let delta = target as i32 - current as i32;
            if delta != 0 {
                term.scroll_display(alacritty_terminal::grid::Scroll::Delta(delta));
            }
        }
    }

    pub fn handle_input(&self, ctx: &egui::Context) {
        if !self.focused {
            return;
        }
        if let Some(pty) = &self.pty {
            let input = crate::terminal::input::process_input(
                ctx,
                VOID_SHORTCUTS,
                self.input_mode(),
                self.selection.is_some(),
            );
            if input.copy_selection {
                if let Some(text) = self.selected_text() {
                    ctx.copy_text(text);
                }
            }
            if !input.bytes.is_empty() {
                pty.write(&input.bytes);
            }
        }
    }

    pub fn check_resize(&mut self, ctx: &egui::Context) {
        if let Some(pty) = &self.pty {
            let content_size = Self::terminal_content_size(self.size);
            let (cols, rows) = crate::terminal::renderer::compute_grid_size_from_ctx(
                ctx,
                content_size.x,
                content_size.y,
            );
            if cols != self.last_cols || rows != self.last_rows {
                self.last_cols = cols;
                self.last_rows = rows;
                pty.resize(rows, cols);
            }
        }
    }

    pub fn sync_title(&mut self) {
        if let Some(pty) = &self.pty {
            if let Ok(t) = pty.title.lock() {
                if *t != self.title {
                    self.title = t.clone();
                }
            }
        }
    }

    /// Convert pointer position to terminal cell (col, row), clamped to grid bounds.
    fn pos_to_cell(&self, pos: Pos2, body: Rect, ctx: &egui::Context) -> (usize, usize) {
        let (cw, ch) = crate::terminal::renderer::cell_size(ctx);
        let col = ((pos.x - body.min.x - crate::terminal::renderer::PAD_X) / cw)
            .floor()
            .clamp(0.0, (self.last_cols as f32) - 1.0) as usize;
        let row = ((pos.y - body.min.y - crate::terminal::renderer::PAD_Y) / ch)
            .floor()
            .clamp(0.0, (self.last_rows as f32) - 1.0) as usize;
        (col, row)
    }

    fn selected_text(&self) -> Option<String> {
        let (sc, sr, ec, er) = self.selection?;
        let pty = self.pty.as_ref()?;
        let term = pty.term.lock().ok()?;
        let text = extract_selection_text(&term, sc, sr, ec, er);
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> PanelInteraction {
        self.check_resize(ui.ctx());
        let mut ix = PanelInteraction::default();
        let pr = self.rect();
        let painter = ui.painter();
        let border_color = if self.focused {
            BORDER_FOCUS
        } else {
            BORDER_DEFAULT
        };

        // ========== PAINT CHROME ==========

        painter.rect_filled(
            pr.translate(Vec2::new(0.0, 3.0)).expand(2.0),
            BORDER_RADIUS + 2.0,
            Color32::from_rgba_premultiplied(0, 0, 0, 35),
        );
        painter.rect_filled(pr, BORDER_RADIUS, PANEL_BG);
        painter.rect_stroke(pr, BORDER_RADIUS, egui::Stroke::new(1.0, border_color));

        let sep_y = pr.min.y + TITLE_BAR_HEIGHT;
        painter.line_segment(
            [
                Pos2::new(pr.min.x + 8.0, sep_y),
                Pos2::new(pr.max.x - 8.0, sep_y),
            ],
            egui::Stroke::new(0.5, Color32::from_rgb(40, 40, 40)),
        );

        let dot_x = pr.min.x + 12.0;
        let dot_y = pr.min.y + TITLE_BAR_HEIGHT * 0.5;
        painter.circle_filled(
            Pos2::new(dot_x, dot_y),
            3.0,
            if self.focused {
                self.color
            } else {
                self.color.linear_multiply(0.4)
            },
        );
        let status = if self.pty.is_some() {
            if self.is_alive() {
                ""
            } else {
                "exited · "
            }
        } else {
            ""
        };
        let tc = if self.is_alive() || self.pty.is_none() {
            FG
        } else {
            FG_DIM
        };
        painter.text(
            Pos2::new(dot_x + 10.0, dot_y - 6.0),
            egui::Align2::LEFT_TOP,
            format!("{}{}", status, self.title),
            egui::FontId::proportional(12.0),
            tc,
        );

        let close_center = Pos2::new(pr.max.x - 14.0, dot_y);
        let close_rect = Rect::from_center_size(close_center, Vec2::splat(16.0));
        let close_resp = ui.interact(
            close_rect,
            egui::Id::new(self.id).with("close"),
            egui::Sense::click(),
        );
        if close_resp.hovered() {
            painter.circle_filled(close_center, 4.0, Color32::from_rgb(200, 60, 60));
            let s = egui::Stroke::new(1.0, Color32::WHITE);
            painter.line_segment(
                [
                    Pos2::new(close_center.x - 1.5, close_center.y - 1.5),
                    Pos2::new(close_center.x + 1.5, close_center.y + 1.5),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(close_center.x + 1.5, close_center.y - 1.5),
                    Pos2::new(close_center.x - 1.5, close_center.y + 1.5),
                ],
                s,
            );
        }
        if close_resp.clicked() {
            ix.action = Some(PanelAction::Close);
        }

        // ========== TERMINAL CONTENT ==========

        let body = Self::terminal_body_rect(pr);
        let content_rect = Self::terminal_content_rect(body);
        let scrollbar_rect = Self::scrollbar_track_rect(body);
        let mut scrollbar_state = None;
        let mut local_interactions_enabled = true;
        let hide_cursor_for_output = self
            .pty
            .as_ref()
            .is_some_and(|pty| pty.should_hide_cursor_for_streaming_output());

        if let Some(pty) = &self.pty {
            if let Ok(term) = pty.term.lock() {
                use alacritty_terminal::grid::Dimensions;

                local_interactions_enabled = !term
                    .mode()
                    .intersects(TermMode::ALT_SCREEN | TermMode::MOUSE_MODE);
                crate::terminal::renderer::render_terminal(
                    ui.ctx(),
                    painter,
                    &term,
                    content_rect,
                    self.focused,
                    hide_cursor_for_output,
                );
                scrollbar_state = Some(ScrollbarState {
                    history_size: term.grid().history_size(),
                    display_offset: term.grid().display_offset(),
                    screen_lines: term.screen_lines(),
                });
            }
        }

        if let Some(error) = &self.spawn_error {
            let clipped = painter.with_clip_rect(content_rect);
            clipped.text(
                content_rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("Failed to start shell\n{error}"),
                egui::FontId::monospace(12.0),
                Color32::from_rgb(220, 120, 120),
            );
        }

        if let Some(state) = scrollbar_state {
            painter.rect_filled(scrollbar_rect, SCROLLBAR_WIDTH * 0.5, SCROLLBAR_TRACK);

            if state.has_history() {
                let thumb_rect = state.thumb_rect(scrollbar_rect);
                let scrollbar_resp = ui.interact(
                    scrollbar_rect,
                    egui::Id::new(self.id).with("scrollbar"),
                    egui::Sense::click_and_drag(),
                );
                let thumb_hovered = scrollbar_resp.hovered()
                    && ui
                        .input(|i| i.pointer.hover_pos())
                        .is_some_and(|pos| thumb_rect.contains(pos));
                let thumb_color = if thumb_hovered || scrollbar_resp.dragged() {
                    SCROLLBAR_THUMB_HOVER
                } else {
                    SCROLLBAR_THUMB
                };
                painter.rect_filled(thumb_rect, SCROLLBAR_WIDTH * 0.5, thumb_color);

                if scrollbar_resp.hovered() || scrollbar_resp.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                }

                if scrollbar_resp.clicked_by(egui::PointerButton::Primary) {
                    ix.clicked = true;
                    if let Some(pos) = scrollbar_resp.interact_pointer_pos() {
                        let grab_offset = if thumb_rect.contains(pos) {
                            pos.y - thumb_rect.top()
                        } else {
                            thumb_rect.height() * 0.5
                        };
                        self.scrollbar_grab_offset = Some(grab_offset);
                        let target_top = (pos.y - grab_offset).clamp(
                            scrollbar_rect.top(),
                            scrollbar_rect.bottom() - thumb_rect.height(),
                        );
                        let target_offset = state.offset_for_thumb_top(
                            scrollbar_rect,
                            thumb_rect.height(),
                            target_top,
                        );
                        self.scroll_to_offset(target_offset);
                    }
                }

                if scrollbar_resp.drag_started_by(egui::PointerButton::Primary) {
                    ix.clicked = true;
                    if let Some(pos) = scrollbar_resp.interact_pointer_pos() {
                        let grab_offset = if thumb_rect.contains(pos) {
                            pos.y - thumb_rect.top()
                        } else {
                            thumb_rect.height() * 0.5
                        };
                        self.scrollbar_grab_offset = Some(grab_offset);
                    }
                }

                if scrollbar_resp.dragged_by(egui::PointerButton::Primary) {
                    ix.clicked = true;
                    if let Some(pos) = scrollbar_resp.interact_pointer_pos() {
                        let grab_offset = self
                            .scrollbar_grab_offset
                            .unwrap_or(thumb_rect.height() * 0.5);
                        let target_top = (pos.y - grab_offset).clamp(
                            scrollbar_rect.top(),
                            scrollbar_rect.bottom() - thumb_rect.height(),
                        );
                        let target_offset = state.offset_for_thumb_top(
                            scrollbar_rect,
                            thumb_rect.height(),
                            target_top,
                        );
                        self.scroll_to_offset(target_offset);
                    }
                }

                if scrollbar_resp.drag_stopped() {
                    self.scrollbar_grab_offset = None;
                }

                if !ui.input(|i| i.pointer.primary_down()) {
                    self.scrollbar_grab_offset = None;
                }
            } else {
                self.scrollbar_grab_offset = None;
            }
        }

        // Draw selection highlight (clipped to body).
        if local_interactions_enabled {
            if let Some((sc, sr, ec, er)) = self.selection {
                let (cw, ch) = crate::terminal::renderer::cell_size(ui.ctx());
                let pad_x = crate::terminal::renderer::PAD_X;
                let pad_y = crate::terminal::renderer::PAD_Y;
                let max_col = self.last_cols as usize;
                let max_row = self.last_rows as usize;

                let (start_row, start_col, end_row, end_col) = if sr < er || (sr == er && sc <= ec)
                {
                    (
                        sr.min(max_row.saturating_sub(1)),
                        sc.min(max_col.saturating_sub(1)),
                        er.min(max_row.saturating_sub(1)),
                        ec.min(max_col.saturating_sub(1)),
                    )
                } else {
                    (
                        er.min(max_row.saturating_sub(1)),
                        ec.min(max_col.saturating_sub(1)),
                        sr.min(max_row.saturating_sub(1)),
                        sc.min(max_col.saturating_sub(1)),
                    )
                };

                let clipped = painter.with_clip_rect(content_rect);
                for row in start_row..=end_row {
                    let c0 = if row == start_row { start_col } else { 0 };
                    let c1 = (if row == end_row { end_col + 1 } else { max_col }).min(max_col);
                    let x0 = content_rect.min.x + pad_x + c0 as f32 * cw;
                    let y0 = content_rect.min.y + pad_y + row as f32 * ch;
                    let x1 = content_rect.min.x + pad_x + c1 as f32 * cw;
                    let sel_rect = Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y0 + ch));
                    clipped.rect_filled(sel_rect, 0.0, SELECTION_BG);
                }
            }
        } else {
            self.selecting = false;
        }

        // ========== INTERACTIONS ==========

        // Body: click + drag (for selection) + focus
        let body_resp = ui.interact(
            content_rect,
            egui::Id::new(self.id).with("body"),
            if local_interactions_enabled {
                egui::Sense::click_and_drag()
            } else {
                egui::Sense::click()
            },
        );

        if body_resp.clicked_by(egui::PointerButton::Primary) {
            ix.clicked = true;
            self.selection = None; // Clear selection on click
        }

        if body_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
        }

        // Text selection via drag
        if local_interactions_enabled && body_resp.drag_started_by(egui::PointerButton::Primary) {
            ix.clicked = true;
            if let Some(pos) = body_resp.interact_pointer_pos() {
                // interact_pointer_pos is already in canvas space (egui applies inverse transform)
                let (col, row) = self.pos_to_cell(pos, content_rect, ui.ctx());
                self.selection = Some((col, row, col, row));
                self.selecting = true;
            }
        }
        if local_interactions_enabled
            && self.selecting
            && body_resp.dragged_by(egui::PointerButton::Primary)
        {
            // Use hover_pos() from the response — it's in canvas space (transformed)
            if let Some(pos) = body_resp.hover_pos() {
                let (col, row) = self.pos_to_cell(pos, content_rect, ui.ctx());
                if let Some(ref mut sel) = self.selection {
                    sel.2 = col;
                    sel.3 = row;
                }
            } else if let Some(pos) = body_resp.interact_pointer_pos() {
                // Fallback: interact_pointer_pos also in canvas space
                let (col, row) = self.pos_to_cell(pos, content_rect, ui.ctx());
                if let Some(ref mut sel) = self.selection {
                    sel.2 = col;
                    sel.3 = row;
                }
            }
        }
        if local_interactions_enabled && body_resp.drag_stopped() {
            self.selecting = false;
        }

        // Title: drag to move, click to focus
        let tb = Rect::from_min_size(pr.min, Vec2::new(pr.width(), TITLE_BAR_HEIGHT));
        let title_resp = ui.interact(
            tb,
            egui::Id::new(self.id).with("title"),
            egui::Sense::click_and_drag(),
        );
        if title_resp.clicked_by(egui::PointerButton::Primary) {
            ix.clicked = true;
        }
        if title_resp.dragged_by(egui::PointerButton::Primary) {
            ix.dragging_title = true;
            ix.drag_delta = title_resp.drag_delta();
            ix.clicked = true;
        }
        if title_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
        }

        // Resize handles
        let rbr = Rect::from_min_size(
            Pos2::new(pr.max.x - RESIZE_HANDLE, pr.max.y - RESIZE_HANDLE),
            Vec2::splat(RESIZE_HANDLE),
        );
        let rr = Rect::from_min_size(
            Pos2::new(pr.max.x - RESIZE_HANDLE, pr.min.y + TITLE_BAR_HEIGHT),
            Vec2::new(
                RESIZE_HANDLE,
                pr.height() - TITLE_BAR_HEIGHT - RESIZE_HANDLE,
            ),
        );
        let rb = Rect::from_min_size(
            Pos2::new(pr.min.x, pr.max.y - RESIZE_HANDLE),
            Vec2::new(pr.width() - RESIZE_HANDLE, RESIZE_HANDLE),
        );

        let brr = ui.interact(rbr, egui::Id::new(self.id).with("rbr"), egui::Sense::drag());
        if brr.dragged() {
            ix.resizing = true;
            ix.resize_delta = brr.drag_delta();
        }
        if !ix.resizing {
            let rrs = ui.interact(rr, egui::Id::new(self.id).with("rr"), egui::Sense::drag());
            if rrs.dragged() {
                ix.resizing = true;
                ix.resize_delta = Vec2::new(rrs.drag_delta().x, 0.0);
            }
        }
        if !ix.resizing {
            let rbs = ui.interact(rb, egui::Id::new(self.id).with("rb"), egui::Sense::drag());
            if rbs.dragged() {
                ix.resizing = true;
                ix.resize_delta = Vec2::new(0.0, rbs.drag_delta().y);
            }
        }

        let hp = ui.input(|i| i.pointer.hover_pos().unwrap_or_default());
        if rbr.contains(hp) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
        } else if rr.contains(hp) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        } else if rb.contains(hp) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }

        body_resp.context_menu(|ui| {
            if ui.button("Rename").clicked() {
                ix.action = Some(PanelAction::Rename);
                ui.close_menu();
            }
            if ui.button("Close").clicked() {
                ix.action = Some(PanelAction::Close);
                ui.close_menu();
            }
        });

        ix
    }

    pub fn apply_drag(&mut self, delta: Vec2) {
        self.position += delta;
    }
    pub fn apply_resize(&mut self, delta: Vec2) {
        self.size = Vec2::new(
            (self.size.x + delta.x).max(MIN_WIDTH),
            (self.size.y + delta.y).max(MIN_HEIGHT),
        );
    }
}

/// Extract text from terminal grid within a cell range.
fn extract_selection_text(
    term: &alacritty_terminal::Term<crate::terminal::pty::EventProxy>,
    sc: usize,
    sr: usize,
    ec: usize,
    er: usize,
) -> String {
    use alacritty_terminal::grid::Dimensions;
    use alacritty_terminal::index::{Column, Point};
    use alacritty_terminal::term::viewport_to_point;

    let (start_row, start_col, end_row, end_col) = if sr < er || (sr == er && sc <= ec) {
        (sr, sc, er, ec)
    } else {
        (er, ec, sr, sc)
    };

    let mut text = String::new();
    let cols = term.columns();
    let display_offset = term.grid().display_offset();

    for row in start_row..=end_row {
        let c0 = if row == start_row { start_col } else { 0 };
        let c1 = if row == end_row { end_col + 1 } else { cols };
        let c1 = c1.min(cols);

        for col in c0..c1 {
            let point = viewport_to_point(display_offset, Point::new(row, Column(col)));
            let cell = &term.grid()[point];
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
    {
        std::env::var("COMSPEC")
            .map(|s| {
                std::path::Path::new(&s)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or(s)
            })
            .unwrap_or_else(|_| "cmd.exe".to_string())
    }
    #[cfg(not(windows))]
    {
        std::env::var("SHELL")
            .map(|s| {
                std::path::Path::new(&s)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or(s)
            })
            .unwrap_or_else(|_| "sh".to_string())
    }
}
