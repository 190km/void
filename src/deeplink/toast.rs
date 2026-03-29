// Lightweight toast notifications for deep-link navigation feedback.

use egui::{Align2, Color32, FontId, Rect, Ui};

pub struct Toast {
    pub message: String,
    pub expires_at: f64,
}

impl Toast {
    pub fn new(message: impl Into<String>, duration_secs: f64, current_time: f64) -> Self {
        Self {
            message: message.into(),
            expires_at: current_time + duration_secs,
        }
    }

    pub fn is_expired(&self, current_time: f64) -> bool {
        current_time >= self.expires_at
    }

    /// Render the toast as a bottom-center overlay. Returns `true` if still visible.
    pub fn show(&self, ui: &mut Ui, canvas_rect: Rect, current_time: f64) -> bool {
        if self.is_expired(current_time) {
            return false;
        }

        let remaining = self.expires_at - current_time;
        let alpha = if remaining < 0.5 {
            (remaining / 0.5) as f32
        } else {
            1.0
        };

        let painter = ui.painter();
        let font = FontId::proportional(14.0);
        let text_color = Color32::from_white_alpha((230.0 * alpha) as u8);
        let bg_color = Color32::from_rgba_unmultiplied(30, 30, 30, (200.0 * alpha) as u8);

        let galley = painter.layout_no_wrap(self.message.clone(), font, Color32::WHITE);
        let text_size = galley.size();
        let padding = egui::vec2(16.0, 10.0);
        let toast_size = text_size + padding * 2.0;

        let center_x = canvas_rect.center().x;
        let bottom_y = canvas_rect.max.y - 40.0;
        let toast_rect = Rect::from_center_size(
            egui::pos2(center_x, bottom_y - toast_size.y / 2.0),
            toast_size,
        );

        painter.rect_filled(toast_rect, 8.0, bg_color);
        painter.text(
            toast_rect.center(),
            Align2::CENTER_CENTER,
            &self.message,
            FontId::proportional(14.0),
            text_color,
        );

        true
    }
}
