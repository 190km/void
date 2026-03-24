// TerminalPanel — canvas space, TSTransform zoom, full terminal interaction.

use std::sync::atomic::Ordering;

use crate::terminal::pty::PtyHandle;
use alacritty_terminal::term::TermMode;
use egui::{Color32, Key, Modifiers, Pos2, Rect, Vec2};
use uuid::Uuid;

pub(crate) const TITLE_BAR_HEIGHT: f32 = 36.0;
pub(crate) const BORDER_RADIUS: f32 = 10.0;
const MIN_WIDTH: f32 = 400.0;
const MIN_HEIGHT: f32 = 280.0;
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
    // Selection state — viewport cell coordinates at the time of selection.
    // Rows are relative to the viewport when selection_display_offset was captured.
    selection: Option<(usize, usize, usize, usize)>, // (start_col, start_row, end_col, end_row)
    selection_display_offset: usize,
    selecting: bool,
    scroll_remainder: f32,
    scrollbar_grab_offset: Option<f32>,
    last_click_time: f64,
    click_count: u8,
    // Tracks where the user intends the panel to be during drag (unsnapped).
    // Snap is computed from this, so accumulated movement can escape snap zones.
    pub drag_virtual_pos: Option<Pos2>,
    /// Same as drag_virtual_pos but for resize — tracks unsnapped size/position
    /// so accumulated movement can escape snap zones naturally.
    pub resize_virtual_rect: Option<Rect>,
    bell_flash_until: f64,
    /// When set, reset terminal modes (ALT_SCREEN, MOUSE_MODE) after this time.
    /// Triggered when Ctrl+C is sent while in ALT_SCREEN — the TUI app is likely
    /// being killed and won't send cleanup escape sequences.
    pending_mode_reset: Option<f64>,
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
    pub resize_left: bool,
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
            selection_display_offset: 0,
            selecting: false,
            scroll_remainder: 0.0,
            scrollbar_grab_offset: None,
            last_click_time: 0.0,
            click_count: 0,
            drag_virtual_pos: None,
            resize_virtual_rect: None,
            bell_flash_until: 0.0,
            pending_mode_reset: None,
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
            selection_display_offset: 0,
            selecting: false,
            scroll_remainder: 0.0,
            scrollbar_grab_offset: None,
            last_click_time: 0.0,
            click_count: 0,
            drag_virtual_pos: None,
            resize_virtual_rect: None,
            bell_flash_until: 0.0,
            pending_mode_reset: None,
        }
    }

    /// Create a panel from saved state, spawning a new terminal process.
    /// Uses the panel's saved CWD if available, falls back to workspace CWD.
    pub fn from_saved(
        ctx: &egui::Context,
        state: &crate::state::persistence::PanelState,
        workspace_cwd: Option<&std::path::Path>,
    ) -> Self {
        let position = Pos2::new(state.position[0], state.position[1]);
        let size = Vec2::new(state.size[0], state.size[1]);
        let color = Color32::from_rgb(state.color[0], state.color[1], state.color[2]);

        // Prefer per-panel CWD (from last session), fall back to workspace CWD
        let cwd = state.cwd.as_deref().or(workspace_cwd);
        let mut panel = Self::new_with_terminal(ctx, position, size, color, cwd);
        panel.z_index = state.z_index;
        panel.focused = state.focused;
        panel
    }

    /// Snapshot the panel layout for persistence (includes CWD if available).
    pub fn to_saved(&self) -> crate::state::persistence::PanelState {
        let cwd = self.pty.as_ref().and_then(|pty| pty.current_cwd());
        crate::state::persistence::PanelState {
            title: self.title.clone(),
            position: [self.position.x, self.position.y],
            size: [self.size.x, self.size.y],
            color: [self.color.r(), self.color.g(), self.color.b()],
            z_index: self.z_index,
            focused: self.focused,
            cwd,
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

    pub fn scroll_hit_test(&self, canvas_pos: Pos2) -> bool {
        Self::terminal_body_rect(self.rect()).contains(canvas_pos)
    }

    pub fn handle_scroll(&mut self, ctx: &egui::Context, scroll_y: f32) {
        let Some(pty) = &self.pty else {
            return;
        };
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

        let mode = self
            .pty
            .as_ref()
            .and_then(|p| p.term.lock().ok())
            .map(|t| *t.mode())
            .unwrap_or(TermMode::empty());
        let in_alt_screen = mode.contains(TermMode::ALT_SCREEN);
        let in_mouse_mode = mode.intersects(TermMode::MOUSE_MODE | TermMode::SGR_MOUSE);

        if in_mouse_mode {
            // Mouse mode: send SGR scroll events (button 64=up, 65=down)
            let count = lines.unsigned_abs() as usize;
            let btn = if lines > 0 { 64 } else { 65 };
            for _ in 0..count {
                let seq = format!("\x1b[<{};1;1M", btn);
                pty.write(seq.as_bytes());
            }
        } else if in_alt_screen {
            // Alt screen: send arrow keys (for vim, less, etc.)
            let key = if lines > 0 { b"\x1b[A" } else { b"\x1b[B" };
            let count = lines.unsigned_abs() as usize;
            let mut bytes = Vec::with_capacity(count * 3);
            for _ in 0..count {
                bytes.extend_from_slice(key);
            }
            pty.write(&bytes);
        } else {
            // Normal mode: scroll display buffer
            if let Ok(mut term) = pty.term.lock() {
                term.scroll_display(alacritty_terminal::grid::Scroll::Delta(lines));
            }
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

    pub fn handle_input(&mut self, ctx: &egui::Context) {
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
                // Any keyboard input snaps scroll back to bottom (standard terminal behavior)
                let has_stale_modes = if let Ok(mut term) = pty.term.lock() {
                    let offset = term.grid().display_offset();
                    if offset != 0 {
                        term.scroll_display(alacritty_terminal::grid::Scroll::Bottom);
                    }
                    term.mode()
                        .intersects(TermMode::ALT_SCREEN | TermMode::MOUSE_MODE)
                } else {
                    false
                };

                // If the terminal has ALT_SCREEN or MOUSE_MODE set and the
                // user sends keyboard input, schedule a mode reset.
                // This catches ALL exit methods from TUI apps:
                // - Ctrl+C (SIGINT), normal quit, Escape, etc.
                // The 500ms delay lets the TUI app respond — if it's still
                // running it will keep its modes active.
                if has_stale_modes {
                    // Only trigger if there's been no PTY output recently
                    // (TUI apps constantly output; a quiet terminal = app exited)
                    let output_stale =
                        pty.time_since_last_output() > std::time::Duration::from_millis(500);

                    if output_stale || input.bytes.contains(&0x03) {
                        let now = ctx.input(|i| i.time);
                        self.pending_mode_reset = Some(now + 0.5);
                    }
                }

                pty.write(&input.bytes);
                self.selection = None;
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
        let text = extract_selection_text(&term, sc, sr, ec, er, self.selection_display_offset);
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        transform: egui::emath::TSTransform,
        screen_clip: Rect,
    ) -> PanelInteraction {
        self.check_resize(ui.ctx());
        let mut ix = PanelInteraction::default();
        let pr = self.rect();
        let painter = ui.painter();
        // Check for bell flash
        let now = ui.input(|i| i.time);
        if let Some(pty) = &self.pty {
            if pty.bell_fired.swap(false, Ordering::Relaxed) {
                self.bell_flash_until = now + 0.15;
            }
        }
        let bell_active = now < self.bell_flash_until;
        if bell_active {
            ui.ctx().request_repaint();
        }

        let border_color = if bell_active {
            Color32::from_rgb(255, 200, 80)
        } else if self.focused {
            BORDER_FOCUS
        } else {
            BORDER_DEFAULT
        };

        // ========== CANVAS CHROME (interactions only, visuals in Tooltip) ==========
        // The canvas layer provides hit-test areas. All visuals are in the shared
        // Tooltip layer so there's only ONE instance of each element (no double-rendering).

        let sep_y = pr.min.y + TITLE_BAR_HEIGHT;
        let dot_x = pr.min.x + 12.0;
        let dot_y = pr.min.y + TITLE_BAR_HEIGHT * 0.5;
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

        let close_center = Pos2::new(pr.max.x - 14.0, dot_y);
        let close_rect = Rect::from_center_size(close_center, Vec2::splat(16.0));
        let close_resp = ui.interact(
            close_rect,
            egui::Id::new(self.id).with("close"),
            egui::Sense::click(),
        );
        if close_resp.clicked() {
            ix.action = Some(PanelAction::Close);
        }

        // ========== TOOLTIP PANEL FILL (shared layer, z-ordered by paint order) ==========
        // Full-panel opaque fill so higher-z panels fully occlude lower-z panels.
        let zoom = transform.scaling;
        let screen_pr = Rect::from_min_max(transform * pr.min, transform * pr.max);
        let shared_layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("term_text"));
        {
            // Full-panel fill — occludes everything from lower-z panels.
            let cp = ui
                .ctx()
                .layer_painter(shared_layer)
                .with_clip_rect(screen_pr.expand(6.0 * zoom).intersect(screen_clip));
            cp.rect_filled(
                screen_pr
                    .translate(Vec2::new(0.0, 3.0 * zoom))
                    .expand(2.0 * zoom),
                (BORDER_RADIUS + 2.0) * zoom,
                Color32::from_rgba_premultiplied(0, 0, 0, 35),
            );
            cp.rect_filled(screen_pr, BORDER_RADIUS * zoom, PANEL_BG);
        }

        // ========== TERMINAL CONTENT ==========

        let body = Self::terminal_body_rect(pr);
        let content_rect = Self::terminal_content_rect(body);
        let scrollbar_rect = Self::scrollbar_track_rect(body);
        let mut scrollbar_state = None;
        let mut sb_thumb_color = SCROLLBAR_THUMB;
        let mut local_interactions_enabled = true;
        let hide_cursor_for_output = self
            .pty
            .as_ref()
            .is_some_and(|pty| pty.should_hide_cursor_for_streaming_output());

        if let Some(pty) = &self.pty {
            // Check pending mode reset: if Ctrl+C was sent while in ALT_SCREEN
            // and the timer has expired, feed reset sequences to the VTE parser.
            // This cleans up stale ALT_SCREEN/MOUSE_MODE from killed TUI apps.
            if let Some(reset_at) = self.pending_mode_reset {
                let now = ui.input(|i| i.time);
                if now >= reset_at {
                    self.pending_mode_reset = None;
                    if let Ok(mut term) = pty.term.lock() {
                        if term
                            .mode()
                            .intersects(TermMode::ALT_SCREEN | TermMode::MOUSE_MODE)
                        {
                            let mut processor: alacritty_terminal::vte::ansi::Processor =
                                alacritty_terminal::vte::ansi::Processor::new();
                            // Clear ALL stale modes:
                            // 1049  = exit alt screen
                            // 1000  = disable click mouse tracking
                            // 1002  = disable button-event mouse tracking
                            // 1003  = disable any-event mouse tracking
                            // 1006  = disable SGR mouse encoding
                            // 1    = reset cursor keys
                            // 2004  = disable bracketed paste
                            processor.advance(
                                &mut *term,
                                b"\x1b[?1049l\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1l\x1b[?2004l",
                            );
                        }
                    }
                }
            }

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
                    transform,
                    screen_clip,
                    self.id,
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
                    && scrollbar_resp
                        .hover_pos()
                        .is_some_and(|pos| thumb_rect.contains(pos));
                let thumb_color = if thumb_hovered || scrollbar_resp.dragged() {
                    SCROLLBAR_THUMB_HOVER
                } else {
                    SCROLLBAR_THUMB
                };
                painter.rect_filled(thumb_rect, SCROLLBAR_WIDTH * 0.5, thumb_color);
                sb_thumb_color = thumb_color;

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

        // Draw selection highlight in the shared Tooltip layer (above terminal text).
        // Selection rows are stored relative to the viewport at selection_display_offset.
        // We adjust by the scroll delta so the highlight follows content and scrolls off-screen.
        if local_interactions_enabled {
            if let Some((sc, sr, ec, er)) = self.selection {
                let (cw, ch) = crate::terminal::renderer::cell_size(ui.ctx());
                let pad_x = crate::terminal::renderer::PAD_X;
                let pad_y = crate::terminal::renderer::PAD_Y;
                let max_col = self.last_cols as usize;
                let max_row = self.last_rows as i32;

                let (start_row, start_col, end_row, end_col) = if sr < er || (sr == er && sc <= ec)
                {
                    (
                        sr,
                        sc.min(max_col.saturating_sub(1)),
                        er,
                        ec.min(max_col.saturating_sub(1)),
                    )
                } else {
                    (
                        er,
                        ec.min(max_col.saturating_sub(1)),
                        sr,
                        sc.min(max_col.saturating_sub(1)),
                    )
                };

                // Adjust rows for scroll: positive delta = scrolled back = rows move down
                let current_offset = scrollbar_state.map(|s| s.display_offset).unwrap_or(0) as i32;
                let offset_delta = current_offset - self.selection_display_offset as i32;
                let adj_start = start_row as i32 + offset_delta;
                let adj_end = end_row as i32 + offset_delta;

                let screen_content =
                    Rect::from_min_max(transform * content_rect.min, transform * content_rect.max)
                        .intersect(screen_clip);
                let sel_painter = ui
                    .ctx()
                    .layer_painter(shared_layer)
                    .with_clip_rect(screen_content);
                for row_i in adj_start..=adj_end {
                    if row_i < 0 || row_i >= max_row {
                        continue;
                    }
                    let row = row_i as usize;
                    let orig_row = row_i - offset_delta; // original selection row
                    let c0 = if orig_row == start_row as i32 {
                        start_col
                    } else {
                        0
                    };
                    let c1 = (if orig_row == end_row as i32 {
                        end_col + 1
                    } else {
                        max_col
                    })
                    .min(max_col);
                    let x0 = content_rect.min.x + pad_x + c0 as f32 * cw;
                    let y0 = content_rect.min.y + pad_y + row as f32 * ch;
                    let x1 = content_rect.min.x + pad_x + c1 as f32 * cw;
                    let sel_rect = Rect::from_min_max(
                        transform * Pos2::new(x0, y0),
                        transform * Pos2::new(x1, y0 + ch),
                    );
                    sel_painter.rect_filled(sel_rect, 0.0, SELECTION_BG);
                }
            }
        } else {
            self.selecting = false;
        }

        // ========== TOOLTIP CHROME OVERLAY ==========
        // All chrome in the shared Tooltip layer — title, border, close, scrollbar, grip.
        // Since the panel fill AND chrome are in the same layer, they move together
        // during zoom — no relative jitter.
        {
            let cp = ui
                .ctx()
                .layer_painter(shared_layer)
                .with_clip_rect(screen_pr.expand(2.0 * zoom).intersect(screen_clip));

            let snap = |p: Pos2| Pos2::new(p.x.round(), p.y.round());

            // Border
            cp.rect_stroke(
                screen_pr,
                BORDER_RADIUS * zoom,
                egui::Stroke::new(zoom.max(0.5), border_color),
            );
            // Separator
            let screen_sep_y = (transform * Pos2::new(0.0, sep_y)).y;
            cp.line_segment(
                [
                    Pos2::new(screen_pr.min.x + 8.0 * zoom, screen_sep_y),
                    Pos2::new(screen_pr.max.x - 8.0 * zoom, screen_sep_y),
                ],
                egui::Stroke::new((0.5 * zoom).max(0.5), Color32::from_rgb(40, 40, 40)),
            );
            // Colored dot
            let screen_dot = snap(transform * Pos2::new(dot_x, dot_y));
            cp.circle_filled(
                screen_dot,
                3.0 * zoom,
                if self.focused {
                    self.color
                } else {
                    self.color.linear_multiply(0.4)
                },
            );
            // Title text — rendered char-by-char on a fixed grid (same technique
            // as terminal text) so metrics changes don't cause relative jitter.
            {
                let title_str = format!("{}{}", status, self.title);
                let title_font = egui::FontId::monospace(13.0 * zoom);
                let title_base = transform * Pos2::new(dot_x + 10.0, dot_y - 7.0);
                // Measure char width at base size, then scale by zoom (constant base = no jitter)
                let title_cw = ui.ctx().fonts(|fonts| {
                    fonts
                        .layout_no_wrap(
                            "M".to_string(),
                            egui::FontId::monospace(13.0),
                            Color32::WHITE,
                        )
                        .rect
                        .width()
                }) * zoom;
                for (i, ch) in title_str.chars().enumerate() {
                    if ch == ' ' {
                        continue;
                    }
                    cp.text(
                        Pos2::new(title_base.x + i as f32 * title_cw, title_base.y),
                        egui::Align2::LEFT_TOP,
                        ch.to_string(),
                        title_font.clone(),
                        tc,
                    );
                }
            }
            // Close button
            {
                let sc = snap(transform * close_center);
                let (cc, cs) = if close_resp.hovered() {
                    cp.circle_filled(
                        sc,
                        (8.0 * zoom).round().max(1.0),
                        Color32::from_rgb(200, 60, 60),
                    );
                    (Color32::WHITE, (3.5 * zoom).round().max(1.0))
                } else {
                    (
                        Color32::from_rgb(100, 100, 100),
                        (3.0 * zoom).round().max(1.0),
                    )
                };
                let stroke = egui::Stroke::new((1.2 * zoom).max(0.5), cc);
                cp.line_segment(
                    [
                        Pos2::new(sc.x - cs, sc.y - cs),
                        Pos2::new(sc.x + cs, sc.y + cs),
                    ],
                    stroke,
                );
                cp.line_segment(
                    [
                        Pos2::new(sc.x + cs, sc.y - cs),
                        Pos2::new(sc.x - cs, sc.y + cs),
                    ],
                    stroke,
                );
            }
            // Scrollbar
            if let Some(state) = scrollbar_state {
                let screen_sb = Rect::from_min_max(
                    transform * scrollbar_rect.min,
                    transform * scrollbar_rect.max,
                );
                cp.rect_filled(screen_sb, SCROLLBAR_WIDTH * 0.5 * zoom, SCROLLBAR_TRACK);
                if state.has_history() {
                    let thumb = state.thumb_rect(scrollbar_rect);
                    let screen_thumb =
                        Rect::from_min_max(transform * thumb.min, transform * thumb.max);
                    cp.rect_filled(screen_thumb, SCROLLBAR_WIDTH * 0.5 * zoom, sb_thumb_color);
                }
            }
            // Resize grip
            {
                let grip_color = Color32::from_rgb(60, 60, 60);
                let gx = pr.max.x - 5.0;
                let gy = pr.max.y - 5.0;
                for i in 0..3 {
                    let offset = i as f32 * 4.0;
                    cp.circle_filled(
                        transform * Pos2::new(gx - offset, gy),
                        1.2 * zoom,
                        grip_color,
                    );
                    if i > 0 {
                        cp.circle_filled(
                            transform * Pos2::new(gx, gy - offset),
                            1.2 * zoom,
                            grip_color,
                        );
                    }
                    if i > 1 {
                        cp.circle_filled(
                            transform * Pos2::new(gx - 4.0, gy - 4.0),
                            1.2 * zoom,
                            grip_color,
                        );
                    }
                }
            }
        }

        // (Title bar elements are in the Tooltip chrome overlay below)

        // ========== INTERACTIONS ==========

        // Visual resize grip (bottom-right corner)
        {
            let grip_color = Color32::from_rgb(60, 60, 60);
            let gx = pr.max.x - 5.0;
            let gy = pr.max.y - 5.0;
            for i in 0..3 {
                let offset = i as f32 * 4.0;
                painter.circle_filled(Pos2::new(gx - offset, gy), 1.2, grip_color);
                if i > 0 {
                    painter.circle_filled(Pos2::new(gx, gy - offset), 1.2, grip_color);
                }
                if i > 1 {
                    painter.circle_filled(Pos2::new(gx - 4.0, gy - 4.0), 1.2, grip_color);
                }
            }
        }

        // Resize handles — extended outside panel for easier grabbing
        let edge_out = 6.0;
        let edge_in = 6.0;
        let corner_size = edge_out + edge_in;

        let rbr = Rect::from_min_max(
            Pos2::new(pr.max.x - edge_in, pr.max.y - edge_in),
            Pos2::new(pr.max.x + edge_out, pr.max.y + edge_out),
        );
        let rbl = Rect::from_min_max(
            Pos2::new(pr.min.x - edge_out, pr.max.y - edge_in),
            Pos2::new(pr.min.x + edge_in, pr.max.y + edge_out),
        );
        let rr = Rect::from_min_max(
            Pos2::new(pr.max.x - edge_in, pr.min.y + TITLE_BAR_HEIGHT),
            Pos2::new(pr.max.x + edge_out, pr.max.y - edge_in),
        );
        let rl = Rect::from_min_max(
            Pos2::new(pr.min.x - edge_out, pr.min.y + TITLE_BAR_HEIGHT),
            Pos2::new(pr.min.x + edge_in, pr.max.y - edge_in),
        );
        let rb = Rect::from_min_max(
            Pos2::new(pr.min.x + corner_size, pr.max.y - edge_in),
            Pos2::new(pr.max.x - corner_size, pr.max.y + edge_out),
        );

        // Register all resize interactions upfront
        let brr_resp = ui.interact(rbr, egui::Id::new(self.id).with("rbr"), egui::Sense::drag());
        let blr_resp = ui.interact(rbl, egui::Id::new(self.id).with("rbl"), egui::Sense::drag());
        let rr_resp = ui.interact(rr, egui::Id::new(self.id).with("rr"), egui::Sense::drag());
        let rl_resp = ui.interact(rl, egui::Id::new(self.id).with("rl"), egui::Sense::drag());
        let rb_resp = ui.interact(rb, egui::Id::new(self.id).with("rb"), egui::Sense::drag());

        // Process resize drags — PRIMARY BUTTON ONLY
        if brr_resp.dragged_by(egui::PointerButton::Primary) {
            ix.resizing = true;
            ix.resize_delta = brr_resp.drag_delta();
        } else if blr_resp.dragged_by(egui::PointerButton::Primary) {
            ix.resizing = true;
            ix.resize_left = true;
            ix.resize_delta = blr_resp.drag_delta();
        } else if rr_resp.dragged_by(egui::PointerButton::Primary) {
            ix.resizing = true;
            ix.resize_delta = Vec2::new(rr_resp.drag_delta().x, 0.0);
        } else if rl_resp.dragged_by(egui::PointerButton::Primary) {
            ix.resizing = true;
            ix.resize_left = true;
            ix.resize_delta = Vec2::new(rl_resp.drag_delta().x, 0.0);
        } else if rb_resp.dragged_by(egui::PointerButton::Primary) {
            ix.resizing = true;
            ix.resize_delta = Vec2::new(0.0, rb_resp.drag_delta().y);
        }

        // Body: click + drag (for selection or mouse forwarding) + focus
        let body_resp = ui.interact(
            content_rect,
            egui::Id::new(self.id).with("body"),
            egui::Sense::click_and_drag(),
        );

        if body_resp.clicked_by(egui::PointerButton::Primary) {
            ix.clicked = true;

            // Track click count for double/triple click
            let now = ui.input(|i| i.time);
            if now - self.last_click_time < 0.4 {
                self.click_count = (self.click_count + 1).min(3);
            } else {
                self.click_count = 1;
            }
            self.last_click_time = now;

            match self.click_count {
                2 => {
                    // Double-click: select word
                    if let Some(pos) = body_resp.interact_pointer_pos() {
                        let (col, row) = self.pos_to_cell(pos, content_rect, ui.ctx());
                        if let Some((start, end)) = self.word_boundaries_at(col, row) {
                            self.selection = Some((start, row, end, row));
                            self.selection_display_offset =
                                scrollbar_state.map(|s| s.display_offset).unwrap_or(0);
                        }
                    }
                }
                3 => {
                    // Triple-click: select entire line
                    if let Some(pos) = body_resp.interact_pointer_pos() {
                        let (_, row) = self.pos_to_cell(pos, content_rect, ui.ctx());
                        let last_col = (self.last_cols as usize).saturating_sub(1);
                        self.selection = Some((0, row, last_col, row));
                        self.selection_display_offset =
                            scrollbar_state.map(|s| s.display_offset).unwrap_or(0);
                    }
                }
                _ => {
                    self.selection = None;
                }
            }
        }

        // Text selection via drag
        if local_interactions_enabled && body_resp.drag_started_by(egui::PointerButton::Primary) {
            ix.clicked = true;
            if let Some(pos) = body_resp.interact_pointer_pos() {
                let (col, row) = self.pos_to_cell(pos, content_rect, ui.ctx());
                self.selection = Some((col, row, col, row));
                self.selection_display_offset =
                    scrollbar_state.map(|s| s.display_offset).unwrap_or(0);
                self.selecting = true;
            }
        }
        if local_interactions_enabled
            && self.selecting
            && body_resp.dragged_by(egui::PointerButton::Primary)
        {
            if let Some(pos) = body_resp.hover_pos() {
                let (col, row) = self.pos_to_cell(pos, content_rect, ui.ctx());
                if let Some(ref mut sel) = self.selection {
                    sel.2 = col;
                    sel.3 = row;
                }
            } else if let Some(pos) = body_resp.interact_pointer_pos() {
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

        // Mouse forwarding to PTY in mouse mode (for TUI apps: htop, lazygit, vim, etc.)
        if !local_interactions_enabled {
            if let Some(pty) = &self.pty {
                let mods = ui.input(|i| i.modifiers);
                let mod_bits: u8 = if mods.shift { 4 } else { 0 }
                    + if mods.alt { 8 } else { 0 }
                    + if mods.ctrl { 16 } else { 0 };

                // Helper: send SGR mouse event
                let send_mouse = |btn: u8, col: usize, row: usize, press: bool| {
                    let suffix = if press { 'M' } else { 'm' };
                    let seq = format!("\x1b[<{};{};{}{}", btn + mod_bits, col + 1, row + 1, suffix);
                    pty.write(seq.as_bytes());
                };

                // Left click press
                if body_resp.clicked_by(egui::PointerButton::Primary)
                    || body_resp.drag_started_by(egui::PointerButton::Primary)
                {
                    ix.clicked = true;
                    if let Some(pos) = body_resp.interact_pointer_pos() {
                        let (col, row) = self.pos_to_cell(pos, content_rect, ui.ctx());
                        send_mouse(0, col, row, true);
                    }
                }
                // Middle click
                if body_resp.clicked_by(egui::PointerButton::Middle) {
                    ix.clicked = true;
                    if let Some(pos) = body_resp.interact_pointer_pos() {
                        let (col, row) = self.pos_to_cell(pos, content_rect, ui.ctx());
                        send_mouse(1, col, row, true);
                        send_mouse(1, col, row, false);
                    }
                }
                // Right click
                if body_resp.clicked_by(egui::PointerButton::Secondary) {
                    if let Some(pos) = body_resp.interact_pointer_pos() {
                        let (col, row) = self.pos_to_cell(pos, content_rect, ui.ctx());
                        send_mouse(2, col, row, true);
                        send_mouse(2, col, row, false);
                    }
                }
                // Mouse drag (motion with button held)
                if body_resp.dragged_by(egui::PointerButton::Primary) {
                    if let Some(pos) = body_resp.hover_pos().or(body_resp.interact_pointer_pos()) {
                        let (col, row) = self.pos_to_cell(pos, content_rect, ui.ctx());
                        send_mouse(32, col, row, true);
                    }
                }
                // Click release
                if body_resp.drag_stopped() {
                    if let Some(pos) = body_resp.interact_pointer_pos() {
                        let (col, row) = self.pos_to_cell(pos, content_rect, ui.ctx());
                        send_mouse(0, col, row, false);
                    }
                }
            }
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
        } else {
            self.drag_virtual_pos = None;
        }

        // Cursor icons — resize overrides body I-beam
        if brr_resp.hovered() || brr_resp.dragged_by(egui::PointerButton::Primary) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
        } else if blr_resp.hovered() || blr_resp.dragged_by(egui::PointerButton::Primary) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNeSw);
        } else if rr_resp.hovered()
            || rr_resp.dragged_by(egui::PointerButton::Primary)
            || rl_resp.hovered()
            || rl_resp.dragged_by(egui::PointerButton::Primary)
        {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        } else if rb_resp.hovered() || rb_resp.dragged_by(egui::PointerButton::Primary) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        } else if body_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
        }

        // Context menu with Copy / Paste / Select All
        body_resp.context_menu(|ui| {
            let has_sel = self.selection.is_some();
            if ui.add_enabled(has_sel, egui::Button::new("Copy")).clicked() {
                if let Some(text) = self.selected_text() {
                    ui.ctx().copy_text(text);
                }
                ui.close_menu();
            }
            if ui.button("Paste").clicked() {
                if let Some(pty) = &self.pty {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if let Ok(text) = clipboard.get_text() {
                            let mode = self.input_mode();
                            if mode.bracketed_paste {
                                let mut bytes = Vec::new();
                                bytes.extend_from_slice(b"\x1b[200~");
                                bytes.extend_from_slice(text.as_bytes());
                                bytes.extend_from_slice(b"\x1b[201~");
                                pty.write(&bytes);
                            } else {
                                pty.write(text.as_bytes());
                            }
                        }
                    }
                }
                ui.close_menu();
            }
            if ui.button("Select All").clicked() {
                let last_col = (self.last_cols as usize).saturating_sub(1);
                let last_row = (self.last_rows as usize).saturating_sub(1);
                self.selection = Some((0, 0, last_col, last_row));
                self.selection_display_offset =
                    scrollbar_state.map(|s| s.display_offset).unwrap_or(0);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Clear Scrollback").clicked() {
                if let Some(pty) = &self.pty {
                    pty.write(b"\x1b[3J");
                }
                ui.close_menu();
            }
            if ui.button("Reset Terminal").clicked() {
                if let Some(pty) = &self.pty {
                    pty.write(b"\x1bc");
                }
                ui.close_menu();
            }
            ui.separator();
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

    /// Find word boundaries at a given cell position.
    fn word_boundaries_at(&self, col: usize, row: usize) -> Option<(usize, usize)> {
        let pty = self.pty.as_ref()?;
        let term = pty.term.lock().ok()?;
        use alacritty_terminal::grid::Dimensions;
        use alacritty_terminal::index::{Column, Point};
        use alacritty_terminal::term::viewport_to_point;

        let display_offset = term.grid().display_offset();
        let cols = term.columns();
        let point = viewport_to_point(display_offset, Point::new(row, Column(col)));
        let c = term.grid()[point].c;

        if c == ' ' || c == '\0' {
            return None;
        }

        let is_word_char =
            |ch: char| ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '.' || ch == '/';

        let mut start = col;
        while start > 0 {
            let p = viewport_to_point(display_offset, Point::new(row, Column(start - 1)));
            if !is_word_char(term.grid()[p].c) {
                break;
            }
            start -= 1;
        }

        let mut end = col;
        while end + 1 < cols {
            let p = viewport_to_point(display_offset, Point::new(row, Column(end + 1)));
            if !is_word_char(term.grid()[p].c) {
                break;
            }
            end += 1;
        }

        Some((start, end))
    }

    pub fn apply_resize(&mut self, delta: Vec2) {
        self.size = Vec2::new(
            (self.size.x + delta.x).max(MIN_WIDTH),
            (self.size.y + delta.y).max(MIN_HEIGHT),
        );
    }

    #[allow(dead_code)]
    pub fn apply_resize_left(&mut self, delta: Vec2) {
        // Left-edge resize: dragging left grows width, moves position
        let new_width = (self.size.x - delta.x).max(MIN_WIDTH);
        let actual_dx = self.size.x - new_width;
        self.position.x += actual_dx;
        self.size.x = new_width;
        // Vertical component is normal (bottom edge)
        self.size.y = (self.size.y + delta.y).max(MIN_HEIGHT);
    }
}

/// Extract text from terminal grid within a cell range.
fn extract_selection_text(
    term: &alacritty_terminal::Term<crate::terminal::pty::EventProxy>,
    sc: usize,
    sr: usize,
    ec: usize,
    er: usize,
    sel_display_offset: usize,
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
    // Use the display_offset from when the selection was made, not current
    let display_offset = sel_display_offset;

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
