// ApplicationPanel — embeds a native app window into the Void canvas.

use egui::{Color32, Pos2, Rect, Vec2};
use uuid::Uuid;

use super::registry::AppEntry;

const TITLE_BAR_HEIGHT: f32 = 36.0;
const BORDER_RADIUS: f32 = 10.0;
const MIN_WIDTH: f32 = 400.0;
const MIN_HEIGHT: f32 = 280.0;

const PANEL_BG: Color32 = Color32::from_rgb(22, 22, 22);
const BORDER_DEFAULT: Color32 = Color32::from_rgb(40, 40, 40);
const BORDER_FOCUS: Color32 = Color32::from_rgb(70, 70, 70);
const FG: Color32 = Color32::from_rgb(200, 200, 200);
const FG_DIM: Color32 = Color32::from_rgb(90, 90, 90);

pub use crate::terminal::panel::{PanelAction, PanelInteraction};

pub struct ApplicationPanel {
    pub id: Uuid,
    pub title: String,
    pub position: Pos2,
    pub size: Vec2,
    pub color: Color32,
    pub z_index: u32,
    pub focused: bool,

    pub app_id: String,
    process: Option<std::process::Child>,
    embedded: bool,
    launch_error: Option<String>,

    #[cfg(windows)]
    child_hwnd: Option<windows::Win32::Foundation::HWND>,

    pub drag_virtual_pos: Option<Pos2>,
    pub resize_virtual_rect: Option<Rect>,
}

impl ApplicationPanel {
    pub fn new(
        app_entry: &AppEntry,
        position: Pos2,
        size: Vec2,
        color: Color32,
        z_index: u32,
    ) -> Self {
        let mut panel = Self {
            id: Uuid::new_v4(),
            title: format!("{}...", app_entry.name),
            position,
            size,
            color,
            z_index,
            focused: false,
            app_id: app_entry.id.to_string(),
            process: None,
            embedded: false,
            launch_error: None,
            #[cfg(windows)]
            child_hwnd: None,
            drag_virtual_pos: None,
            resize_virtual_rect: None,
        };

        // Try to launch the app
        if let Some(exe) = super::registry::resolve_exe(app_entry.exe_candidates) {
            match std::process::Command::new(&exe).spawn() {
                Ok(child) => {
                    panel.process = Some(child);
                }
                Err(e) => {
                    panel.launch_error = Some(format!("Failed to launch: {e}"));
                    panel.title = format!("{} (failed)", app_entry.name);
                }
            }
        } else {
            panel.launch_error = Some("Executable not found".to_string());
            panel.title = format!("{} (not found)", app_entry.name);
        }

        panel
    }

    pub fn rect(&self) -> Rect {
        Rect::from_min_size(self.position, self.size)
    }

    fn content_rect(&self) -> Rect {
        let pr = self.rect();
        Rect::from_min_max(
            Pos2::new(pr.min.x + 1.0, pr.min.y + TITLE_BAR_HEIGHT),
            Pos2::new(pr.max.x - 1.0, pr.max.y - 1.0),
        )
    }

    pub fn is_alive(&self) -> bool {
        self.process
            .as_ref()
            .map(|_| {
                // Can't call try_wait on a shared ref, assume alive if process exists
                true
            })
            .unwrap_or(false)
    }

    pub fn apply_resize(&mut self, delta: Vec2) {
        self.size.x = (self.size.x + delta.x).max(MIN_WIDTH);
        self.size.y = (self.size.y + delta.y).max(MIN_HEIGHT);
    }

    pub fn apply_resize_left(&mut self, delta: Vec2) {
        let new_w = (self.size.x - delta.x).max(MIN_WIDTH);
        let actual_dx = self.size.x - new_w;
        self.position.x += actual_dx;
        self.size.x = new_w;
        self.size.y = (self.size.y + delta.y).max(MIN_HEIGHT);
    }

    #[cfg(windows)]
    fn try_embed(&mut self, void_hwnd: windows::Win32::Foundation::HWND) {
        if self.embedded || self.child_hwnd.is_some() {
            return;
        }
        let Some(ref process) = self.process else {
            return;
        };
        let pid = process.id();

        // Try to find the app's window
        let app_entry = super::registry::APPS.iter().find(|a| a.id == self.app_id);
        let title_contains = app_entry
            .map(|a| a.window_title_contains)
            .unwrap_or(&self.app_id);

        if let Some(hwnd) = super::embed::platform::find_window_by_pid(pid, title_contains) {
            if super::embed::platform::embed_window(hwnd, void_hwnd).is_ok() {
                self.child_hwnd = Some(hwnd);
                self.embedded = true;
                self.title = app_entry
                    .map(|a| a.name.to_string())
                    .unwrap_or_else(|| self.app_id.clone());
            }
        }
    }

    pub fn close(&mut self) {
        #[cfg(windows)]
        if let Some(hwnd) = self.child_hwnd.take() {
            super::embed::platform::detach_window(hwnd);
        }
        if let Some(mut p) = self.process.take() {
            let _ = p.kill();
        }
        self.embedded = false;
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        transform: egui::emath::TSTransform,
        screen_clip: Rect,
        #[cfg(windows)] void_hwnd: Option<windows::Win32::Foundation::HWND>,
    ) -> PanelInteraction {
        let mut ix = PanelInteraction::default();
        let pr = self.rect();
        let zoom = transform.scaling;
        let screen_pr = Rect::from_min_max(transform * pr.min, transform * pr.max);
        let shared_layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("term_text"));

        // Try to embed the window if not yet done
        #[cfg(windows)]
        if let Some(parent) = void_hwnd {
            self.try_embed(parent);
        }

        // Panel fill (same layer as terminals for proper z-ordering)
        {
            let cp = ui
                .ctx()
                .layer_painter(shared_layer)
                .with_clip_rect(screen_pr.expand(6.0 * zoom).intersect(screen_clip));
            // Shadow
            cp.rect_filled(
                screen_pr
                    .translate(Vec2::new(0.0, 3.0 * zoom))
                    .expand(2.0 * zoom),
                (BORDER_RADIUS + 2.0) * zoom,
                Color32::from_rgba_premultiplied(0, 0, 0, 35),
            );
            // Background
            cp.rect_filled(screen_pr, BORDER_RADIUS * zoom, PANEL_BG);
        }

        // Reposition the embedded native window to match the panel's screen rect
        #[cfg(windows)]
        if let Some(hwnd) = self.child_hwnd {
            let body = self.content_rect();
            let screen_body = Rect::from_min_max(transform * body.min, transform * body.max)
                .intersect(screen_clip);

            // Convert from egui screen coords to window-relative coords
            let window_pos = ui.input(|i| {
                i.viewport()
                    .outer_rect
                    .map(|wr| Pos2::new(wr.min.x, wr.min.y))
                    .unwrap_or(Pos2::ZERO)
            });
            let inner_pos = ui.input(|i| {
                i.viewport()
                    .inner_rect
                    .map(|ir| Pos2::new(ir.min.x, ir.min.y))
                    .unwrap_or(Pos2::ZERO)
            });
            let title_bar_offset = inner_pos.y - window_pos.y;

            let x = screen_body.min.x as i32;
            let y = (screen_body.min.y + title_bar_offset) as i32;
            let w = screen_body.width() as i32;
            let h = screen_body.height() as i32;

            let is_visible = screen_body.width() > 10.0 && screen_body.height() > 10.0;
            super::embed::platform::set_visible(hwnd, is_visible);
            if is_visible {
                super::embed::platform::reposition(hwnd, x, y, w, h);
            }
        }

        // Show placeholder text if not yet embedded
        if !self.embedded {
            let msg = if self.launch_error.is_some() {
                self.launch_error.as_deref().unwrap_or("Error")
            } else {
                "Launching..."
            };
            let body = self.content_rect();
            let cp = ui
                .ctx()
                .layer_painter(shared_layer)
                .with_clip_rect(screen_pr.intersect(screen_clip));
            let center = transform * body.center();
            cp.text(
                center,
                egui::Align2::CENTER_CENTER,
                msg,
                egui::FontId::monospace(14.0 * zoom),
                FG_DIM,
            );
        }

        // Chrome overlay (title bar, border, close button)
        {
            let cp = ui
                .ctx()
                .layer_painter(shared_layer)
                .with_clip_rect(screen_pr.expand(2.0 * zoom).intersect(screen_clip));

            let snap = |p: Pos2| Pos2::new(p.x.round(), p.y.round());
            let border_color = if self.focused {
                BORDER_FOCUS
            } else {
                BORDER_DEFAULT
            };

            // Border
            cp.rect_stroke(
                screen_pr,
                BORDER_RADIUS * zoom,
                egui::Stroke::new(zoom.max(0.5), border_color),
            );

            // Separator
            let sep_y = pr.min.y + TITLE_BAR_HEIGHT;
            let screen_sep_y = (transform * Pos2::new(0.0, sep_y)).y;
            cp.line_segment(
                [
                    Pos2::new(screen_pr.min.x + 8.0 * zoom, screen_sep_y),
                    Pos2::new(screen_pr.max.x - 8.0 * zoom, screen_sep_y),
                ],
                egui::Stroke::new((0.5 * zoom).max(0.5), Color32::from_rgb(40, 40, 40)),
            );

            // Colored dot
            let dot_x = pr.min.x + 12.0;
            let dot_y = pr.min.y + TITLE_BAR_HEIGHT * 0.5;
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

            // App icon badge
            let icon = super::registry::APPS
                .iter()
                .find(|a| a.id == self.app_id)
                .map(|a| a.icon)
                .unwrap_or("?");
            let icon_pos = snap(transform * Pos2::new(dot_x + 10.0, dot_y - 7.0));
            cp.text(
                icon_pos,
                egui::Align2::LEFT_TOP,
                format!("[{}] {}", icon, self.title),
                egui::FontId::monospace(13.0 * zoom),
                if self.is_alive() || self.process.is_none() {
                    FG
                } else {
                    FG_DIM
                },
            );

            // Close button
            let close_center = Pos2::new(pr.max.x - 14.0, dot_y);
            let sc = snap(transform * close_center);
            let screen_close = Rect::from_center_size(sc, Vec2::splat(24.0 * zoom));
            let close_hovered = ui
                .input(|i| i.pointer.hover_pos())
                .map(|p| screen_close.contains(p))
                .unwrap_or(false);

            let (cc, cs) = if close_hovered {
                cp.circle_filled(
                    sc,
                    (10.0 * zoom).round().max(2.0),
                    Color32::from_rgb(220, 50, 50),
                );
                (Color32::WHITE, (4.0 * zoom).round().max(1.0))
            } else {
                (
                    Color32::from_rgb(120, 120, 120),
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

            // Close click detection
            if close_hovered && ui.input(|i| i.pointer.primary_released()) {
                ix.action = Some(PanelAction::Close);
            }

            // Cursor
            if close_hovered {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }

        // Title bar interaction (drag to move, click to focus)
        let tb = Rect::from_min_size(pr.min, Vec2::new(pr.width(), TITLE_BAR_HEIGHT));
        let title_resp = ui.interact(
            tb,
            egui::Id::new(self.id).with("app_title"),
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

        // Resize handle (bottom-right)
        let edge = 6.0;
        let rbr = Rect::from_min_max(
            Pos2::new(pr.max.x - edge, pr.max.y - edge),
            Pos2::new(pr.max.x + edge, pr.max.y + edge),
        );
        let brr_resp = ui.interact(
            rbr,
            egui::Id::new(self.id).with("app_rbr"),
            egui::Sense::drag(),
        );
        if brr_resp.dragged_by(egui::PointerButton::Primary) {
            ix.resizing = true;
            ix.resize_delta = brr_resp.drag_delta();
        }
        if brr_resp.hovered() || brr_resp.dragged_by(egui::PointerButton::Primary) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
        }

        ix
    }

    pub fn to_saved(&self) -> crate::state::persistence::PanelState {
        crate::state::persistence::PanelState {
            title: self.title.clone(),
            position: [self.position.x, self.position.y],
            size: [self.size.x, self.size.y],
            color: [self.color.r(), self.color.g(), self.color.b()],
            z_index: self.z_index,
            focused: self.focused,
        }
    }

    pub fn sync_title(&mut self) {
        // App title is set during embedding, no dynamic sync needed
    }

    pub fn scroll_hit_test(&self, canvas_pos: Pos2) -> bool {
        self.content_rect().contains(canvas_pos)
    }
}

impl Drop for ApplicationPanel {
    fn drop(&mut self) {
        self.close();
    }
}
