// Flat panel list — all panels in current workspace

use egui::{Color32, Pos2, Vec2};

use crate::panel::CanvasPanel;
use crate::sidebar::{
    SidebarResponse, HOVER_BG, ITEM_BG, ITEM_HEIGHT, ITEM_ROUNDING, TEXT_MUTED, TEXT_PRIMARY,
    TEXT_SECONDARY,
};

/// Draw a flat list of panels for the Terminals tab.
pub fn draw_terminal_list(ui: &mut egui::Ui, panels: &[CanvasPanel]) -> Vec<SidebarResponse> {
    let mut responses = Vec::new();
    let available_width = ui.available_width();

    if panels.is_empty() {
        ui.add_space(16.0);
        ui.painter().text(
            Pos2::new(ui.cursor().min.x + available_width / 2.0, ui.cursor().min.y),
            egui::Align2::CENTER_TOP,
            "No terminals",
            egui::FontId::proportional(11.0),
            TEXT_MUTED,
        );
        ui.allocate_exact_size(Vec2::new(available_width, 20.0), egui::Sense::hover());
        return responses;
    }

    ui.add_space(4.0);

    for (panel_idx, panel) in panels.iter().enumerate() {
        let is_focused = panel.focused();

        // Allocate row
        let (item_rect, _) = ui.allocate_exact_size(
            Vec2::new(available_width, ITEM_HEIGHT),
            egui::Sense::hover(),
        );

        let painter = ui.painter();
        let resp = ui.interact(
            item_rect,
            egui::Id::new("term_item").with(panel.id()),
            egui::Sense::click(),
        );

        // Background: selected or hover
        if is_focused {
            painter.rect_filled(item_rect, ITEM_ROUNDING, ITEM_BG);
        } else if resp.hovered() {
            painter.rect_filled(item_rect, ITEM_ROUNDING, HOVER_BG);
        }

        // Color dot
        let dot_center = Pos2::new(item_rect.left() + 14.0, item_rect.center().y);
        let dot_color = if panel.is_alive() {
            panel.color()
        } else {
            Color32::from_rgb(50, 50, 50)
        };
        painter.circle_filled(dot_center, 3.0, dot_color);

        // Alive indicator ring
        if panel.is_alive() {
            painter.circle_stroke(
                dot_center,
                4.5,
                egui::Stroke::new(0.5, panel.color().linear_multiply(0.3)),
            );
        }

        // Title
        let title_color = if is_focused {
            TEXT_PRIMARY
        } else {
            TEXT_SECONDARY
        };
        let title = panel.title();
        let display_title = if title.len() > 31 {
            format!("{}...", &title[..31])
        } else {
            title.to_string()
        };
        painter.text(
            Pos2::new(item_rect.left() + 26.0, item_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &display_title,
            egui::FontId::proportional(12.0),
            title_color,
        );

        // "···" on hover
        if resp.hovered() {
            painter.text(
                Pos2::new(item_rect.right() - 14.0, item_rect.center().y),
                egui::Align2::CENTER_CENTER,
                "···",
                egui::FontId::proportional(12.0),
                TEXT_MUTED,
            );
        }

        // Click → focus panel
        if resp.clicked() {
            responses.push(SidebarResponse::FocusPanel { panel_id: panel.id() });
        }

        // Context menu
        resp.context_menu(|ui| {
            if ui.button("Rename").clicked() {
                responses.push(SidebarResponse::RenamePanel(panel.id()));
                ui.close_menu();
            }
            if ui.button("Close").clicked() {
                responses.push(SidebarResponse::ClosePanel(panel_idx));
                ui.close_menu();
            }
        });
    }

    // "+ terminal" at the bottom
    ui.add_space(8.0);
    let (add_rect, _) = ui.allocate_exact_size(
        Vec2::new(available_width, ITEM_HEIGHT),
        egui::Sense::hover(),
    );
    let add_resp = ui.interact(add_rect, egui::Id::new("term_add"), egui::Sense::click());
    if add_resp.hovered() {
        ui.painter().rect_filled(add_rect, ITEM_ROUNDING, HOVER_BG);
    }
    ui.painter().text(
        Pos2::new(add_rect.left() + 14.0, add_rect.center().y),
        egui::Align2::LEFT_CENTER,
        "+ New terminal",
        egui::FontId::proportional(11.0),
        if add_resp.hovered() {
            TEXT_PRIMARY
        } else {
            TEXT_MUTED
        },
    );
    if add_resp.clicked() {
        responses.push(SidebarResponse::SpawnTerminal);
    }

    responses
}
