// Viewport/camera: pan + zoom with manual coordinate transforms

use egui::{emath::TSTransform, Pos2, Rect, Vec2};

#[derive(Clone, Debug)]
pub struct Viewport {
    pub pan: Vec2,
    pub zoom: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self { pan: Vec2::ZERO, zoom: 1.0 }
    }
}

impl Viewport {
    pub const ZOOM_MIN: f32 = 0.25;
    pub const ZOOM_MAX: f32 = 4.0;

    /// Build TSTransform for the canvas layer (used with set_transform_layer).
    pub fn transform(&self, canvas_rect: Rect) -> TSTransform {
        TSTransform::new(canvas_rect.min.to_vec2() + self.pan, self.zoom)
    }

    pub fn screen_to_canvas(&self, screen_pos: Pos2, screen_rect: Rect) -> Pos2 {
        let rel = screen_pos - screen_rect.left_top();
        Pos2::new(
            (rel.x - self.pan.x) / self.zoom,
            (rel.y - self.pan.y) / self.zoom,
        )
    }

    pub fn canvas_to_screen(&self, canvas_pos: Pos2, screen_rect: Rect) -> Pos2 {
        Pos2::new(
            canvas_pos.x * self.zoom + self.pan.x + screen_rect.left(),
            canvas_pos.y * self.zoom + self.pan.y + screen_rect.top(),
        )
    }

    pub fn visible_canvas_rect(&self, screen_rect: Rect) -> Rect {
        let tl = self.screen_to_canvas(screen_rect.left_top(), screen_rect);
        let br = self.screen_to_canvas(screen_rect.right_bottom(), screen_rect);
        Rect::from_min_max(tl, br)
    }

    pub fn is_visible(&self, panel_rect: Rect, screen_rect: Rect) -> bool {
        self.visible_canvas_rect(screen_rect).intersects(panel_rect)
    }

    pub fn zoom_around(&mut self, screen_pos: Pos2, screen_rect: Rect, zoom_factor: f32) {
        let anchor = self.screen_to_canvas(screen_pos, screen_rect);
        self.zoom = (self.zoom * zoom_factor).clamp(Self::ZOOM_MIN, Self::ZOOM_MAX);
        let rel = screen_pos - screen_rect.left_top();
        self.pan.x = rel.x - anchor.x * self.zoom;
        self.pan.y = rel.y - anchor.y * self.zoom;
    }

    pub fn pan_to_center(&mut self, canvas_pos: Pos2, screen_rect: Rect) {
        self.pan.x = screen_rect.width() / 2.0 - canvas_pos.x * self.zoom;
        self.pan.y = screen_rect.height() / 2.0 - canvas_pos.y * self.zoom;
    }

    #[allow(dead_code)]
    pub fn canvas_rect_to_screen(&self, rect: Rect, screen_rect: Rect) -> Rect {
        Rect::from_min_max(
            self.canvas_to_screen(rect.min, screen_rect),
            self.canvas_to_screen(rect.max, screen_rect),
        )
    }
}
