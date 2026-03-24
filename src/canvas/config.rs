// Centralized canvas configuration constants

use egui::Color32;

// ===== Viewport / Zoom =====
pub const ZOOM_MIN: f32 = 0.125;
pub const ZOOM_MAX: f32 = 4.0;
pub const ZOOM_KEYBOARD_FACTOR: f32 = 1.15;

// ===== Grid =====
pub const GRID_SPACING: f32 = 40.0;
pub const GRID_COLOR: Color32 = Color32::from_rgb(30, 30, 30);

// ===== Snap =====
pub const SNAP_THRESHOLD: f32 = 8.0;

// ===== Minimap =====
pub const MINIMAP_WIDTH: f32 = 200.0;
pub const MINIMAP_HEIGHT: f32 = 150.0;
pub const MINIMAP_PADDING: f32 = 10.0;
pub const MINIMAP_BG: Color32 = Color32::from_rgba_premultiplied(15, 15, 15, 200);
pub const MINIMAP_VIEWPORT_BORDER: Color32 = Color32::from_rgb(100, 100, 100);

// ===== Default panel size =====
pub const DEFAULT_PANEL_WIDTH: f32 = 1904.0; // 1120 * 1.7
pub const DEFAULT_PANEL_HEIGHT: f32 = 720.0;
pub const PANEL_GAP: f32 = 30.0;
