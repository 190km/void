// Sidebar module: Framer-style dark sidebar with tabs, search, and list items

pub mod terminal_list;
pub mod workspace_list;

use egui::{Color32, Pos2, Rect, Vec2};
use uuid::Uuid;

use crate::state::workspace::Workspace;

// ── Color palette (Tailwind neutral/zinc) ──────────────────────────────────

pub const SIDEBAR_BG: Color32 = Color32::from_rgb(23, 23, 23);
pub const SIDEBAR_BORDER: Color32 = Color32::from_rgb(38, 38, 38);
pub const INPUT_BG: Color32 = Color32::from_rgb(39, 39, 42);
pub const ACTIVE_TAB_BG: Color32 = Color32::from_rgb(63, 63, 70);
pub const TEXT_PRIMARY: Color32 = Color32::WHITE;
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(163, 163, 163);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(115, 115, 115);
pub const DIVIDER: Color32 = Color32::from_rgb(38, 38, 38);
pub const HOVER_BG: Color32 = Color32::from_rgba_premultiplied(39, 39, 42, 120);
pub const ITEM_BG: Color32 = Color32::from_rgb(39, 39, 42);

// ── Dimensions ─────────────────────────────────────────────────────────────

pub const SIDEBAR_PADDING_H: f32 = 12.0;
pub const TAB_BAR_HEIGHT: f32 = 32.0;
pub const TAB_BAR_ROUNDING: f32 = 8.0;
pub const TAB_INDICATOR_ROUNDING: f32 = 6.0;
pub const TAB_INDICATOR_INSET: f32 = 3.0;
pub const ITEM_HEIGHT: f32 = 32.0;
pub const ITEM_ROUNDING: f32 = 8.0;
pub const SECTION_HEADER_HEIGHT: f32 = 40.0;

// ── Types ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    Workspaces,
    Terminals,
}

#[derive(Debug, Clone)]
pub enum SidebarResponse {
    SwitchWorkspace(usize),
    CreateWorkspace,
    DeleteWorkspace(usize),
    FocusPanel { panel_id: Uuid },
    SpawnTerminal,
    RenamePanel(Uuid),
    ClosePanel(usize),
}

pub struct Sidebar {
    pub active_tab: SidebarTab,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self {
            active_tab: SidebarTab::Workspaces,
        }
    }
}

impl Sidebar {
    /// Top-level sidebar render. Returns actions for `VoidApp` to handle.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        workspaces: &[Workspace],
        active_ws: usize,
        brand_texture: &egui::TextureHandle,
    ) -> Vec<SidebarResponse> {
        let mut responses = Vec::new();

        ui.spacing_mut().item_spacing.y = 0.0;

        // ── Brand logo ─────────────────────────────────────────────────
        ui.add_space(14.0);
        let logo_resp = ui.add(
            egui::Image::new(egui::load::SizedTexture::new(
                brand_texture.id(),
                brand_texture.size_vec2(),
            ))
            .max_height(14.0)
            .tint(Color32::from_rgb(140, 140, 140))
            .sense(egui::Sense::hover()),
        );
        // Suppress default context menu on the logo area
        logo_resp.context_menu(|_ui| {});
        ui.add_space(14.0);

        // ── Tab bar ────────────────────────────────────────────────────
        self.draw_tab_bar(ui);
        ui.add_space(10.0);

        // ── Scrollable content ─────────────────────────────────────────
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                match self.active_tab {
                    SidebarTab::Workspaces => {
                        responses.extend(workspace_list::draw_workspace_tree(
                            ui, workspaces, active_ws,
                        ));
                    }
                    SidebarTab::Terminals => {
                        responses.extend(terminal_list::draw_terminal_list(
                            ui,
                            &workspaces[active_ws].panels,
                        ));
                    }
                }
            });

        // ── Bottom shortcuts hint ──────────────────────────────────────
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("Ctrl+Shift+T new · Ctrl+B sidebar · Ctrl+M minimap")
                    .color(TEXT_MUTED)
                    .size(9.5),
            );
            ui.add_space(6.0);
        });

        responses
    }

    // ── Tab bar ────────────────────────────────────────────────────────────

    fn draw_tab_bar(&mut self, ui: &mut egui::Ui) {
        let available_width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(available_width, TAB_BAR_HEIGHT),
            egui::Sense::hover(),
        );

        let painter = ui.painter();

        // Track background (pill)
        painter.rect_filled(rect, TAB_BAR_ROUNDING, INPUT_BG);

        // Compute tab rects
        let inset = TAB_INDICATOR_INSET;
        let tab_width = (rect.width() - inset * 2.0) / 2.0;
        let tab_height = rect.height() - inset * 2.0;

        let tab_rect_0 = Rect::from_min_size(
            Pos2::new(rect.min.x + inset, rect.min.y + inset),
            Vec2::new(tab_width, tab_height),
        );
        let tab_rect_1 = Rect::from_min_size(
            Pos2::new(tab_rect_0.max.x, rect.min.y + inset),
            Vec2::new(tab_width, tab_height),
        );

        // Active indicator
        let active_rect = match self.active_tab {
            SidebarTab::Workspaces => tab_rect_0,
            SidebarTab::Terminals => tab_rect_1,
        };
        painter.rect_filled(active_rect, TAB_INDICATOR_ROUNDING, ACTIVE_TAB_BG);
        painter.rect_stroke(
            active_rect,
            TAB_INDICATOR_ROUNDING,
            egui::Stroke::new(0.5, Color32::from_rgb(55, 55, 60)),
        );

        // Divider between tabs (subtle, hidden when tab indicator covers it)
        let divider_x = tab_rect_0.max.x;
        let divider_top = rect.min.y + 9.0;
        let divider_bot = rect.max.y - 9.0;
        painter.line_segment(
            [
                Pos2::new(divider_x, divider_top),
                Pos2::new(divider_x, divider_bot),
            ],
            egui::Stroke::new(1.0, Color32::from_rgb(55, 55, 60)),
        );

        // Tab labels + interaction
        let tabs = [
            (SidebarTab::Workspaces, "Workspaces", tab_rect_0),
            (SidebarTab::Terminals, "Terminals", tab_rect_1),
        ];
        for (tab, label, tab_rect) in tabs {
            let is_active = self.active_tab == tab;
            let resp = ui.interact(tab_rect, egui::Id::new(label), egui::Sense::click());
            let text_color = if is_active {
                TEXT_PRIMARY
            } else if resp.hovered() {
                Color32::from_rgb(200, 200, 200)
            } else {
                TEXT_SECONDARY
            };
            painter.text(
                tab_rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(11.0),
                text_color,
            );
            if resp.clicked() {
                self.active_tab = tab;
            }
        }
    }
}
