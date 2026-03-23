// Workspace tree — section headers per workspace, terminal items underneath

use egui::{Color32, Pos2, Rect, Vec2};

use crate::sidebar::{
    SidebarResponse, DIVIDER, HOVER_BG, ITEM_BG, ITEM_HEIGHT, ITEM_ROUNDING, SECTION_HEADER_HEIGHT,
    TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::state::workspace::Workspace;

/// Draw the workspace tree for the Workspaces tab.
pub fn draw_workspace_tree(
    ui: &mut egui::Ui,
    workspaces: &[Workspace],
    active_ws: usize,
) -> Vec<SidebarResponse> {
    let mut responses = Vec::new();
    let available_width = ui.available_width();

    for (ws_idx, ws) in workspaces.iter().enumerate() {
        let is_active = ws_idx == active_ws;

        // ── Separator with padding ───────────────────────────────────
        ui.add_space(8.0);
        let sep_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(available_width, 1.0));
        ui.painter().rect_filled(sep_rect, 0.0, DIVIDER);
        ui.allocate_exact_size(Vec2::new(available_width, 1.0), egui::Sense::hover());

        // ── Section header ─────────────────────────────────────────
        let (header_rect, _) = ui.allocate_exact_size(
            Vec2::new(available_width, SECTION_HEADER_HEIGHT),
            egui::Sense::hover(),
        );

        let painter = ui.painter();

        // Workspace name
        let ws_label = if let Some(cwd) = &ws.cwd {
            cwd.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| ws.name.clone())
        } else {
            ws.name.clone()
        };

        let name_color = if is_active {
            TEXT_PRIMARY
        } else {
            TEXT_SECONDARY
        };
        painter.text(
            Pos2::new(header_rect.left() + 2.0, header_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &ws_label,
            egui::FontId::proportional(11.0),
            name_color,
        );

        // Click header to switch workspace (exclude "+" button area)
        let header_click_rect = Rect::from_min_max(
            header_rect.min,
            Pos2::new(header_rect.right() - 28.0, header_rect.max.y),
        );
        let header_resp = ui.interact(
            header_click_rect,
            egui::Id::new("ws_header").with(ws_idx),
            egui::Sense::click(),
        );
        if header_resp.clicked() && !is_active {
            responses.push(SidebarResponse::SwitchWorkspace(ws_idx));
        }
        // Context menu for delete
        header_resp.context_menu(|ui| {
            if workspaces.len() > 1 && ui.button("Delete workspace").clicked() {
                responses.push(SidebarResponse::DeleteWorkspace(ws_idx));
                ui.close_menu();
            }
        });

        // "+" button (aligned right)
        let btn_rect = Rect::from_min_size(
            Pos2::new(header_rect.right() - 24.0, header_rect.min.y + 8.0),
            Vec2::new(24.0, 24.0),
        );
        let btn_resp = ui.interact(
            btn_rect,
            egui::Id::new("ws_add").with(ws_idx),
            egui::Sense::click(),
        );
        if btn_resp.hovered() {
            painter.rect_filled(btn_rect, 6.0, HOVER_BG);
        }
        let btn_color = if btn_resp.hovered() {
            TEXT_PRIMARY
        } else {
            TEXT_SECONDARY
        };
        painter.text(
            btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            "+",
            egui::FontId::proportional(14.0),
            btn_color,
        );
        if btn_resp.clicked() {
            responses.push(SidebarResponse::SpawnTerminal);
        }

        // ── Terminal items (only for active workspace) ───────────────
        if is_active {
            for (panel_idx, panel) in ws.panels.iter().enumerate() {
                let is_focused = panel.focused;

                // Allocate row
                let (item_rect, _) = ui.allocate_exact_size(
                    Vec2::new(available_width, ITEM_HEIGHT),
                    egui::Sense::hover(),
                );

                let painter = ui.painter();
                let resp = ui.interact(
                    item_rect,
                    egui::Id::new("ws_panel").with(panel.id),
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
                    panel.color
                } else {
                    Color32::from_rgb(50, 50, 50)
                };
                painter.circle_filled(dot_center, 3.0, dot_color);

                // Title
                let title_color = if is_focused {
                    TEXT_PRIMARY
                } else {
                    TEXT_SECONDARY
                };
                let display_title = if panel.title.len() > 34 {
                    format!("{}...", &panel.title[..34])
                } else {
                    panel.title.clone()
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
                    responses.push(SidebarResponse::FocusPanel { panel_id: panel.id });
                }

                // Context menu
                resp.context_menu(|ui| {
                    if ui.button("Rename").clicked() {
                        responses.push(SidebarResponse::RenamePanel(panel.id));
                        ui.close_menu();
                    }
                    if ui.button("Close").clicked() {
                        responses.push(SidebarResponse::ClosePanel(panel_idx));
                        ui.close_menu();
                    }
                });
            }
        }
    }

    // "New workspace" row at the bottom
    ui.add_space(8.0);
    let (add_rect, _) = ui.allocate_exact_size(
        Vec2::new(available_width, ITEM_HEIGHT),
        egui::Sense::hover(),
    );
    let add_resp = ui.interact(add_rect, egui::Id::new("ws_create"), egui::Sense::click());
    if add_resp.hovered() {
        ui.painter().rect_filled(add_rect, ITEM_ROUNDING, HOVER_BG);
    }
    ui.painter().text(
        Pos2::new(add_rect.left() + 14.0, add_rect.center().y),
        egui::Align2::LEFT_CENTER,
        "+ New workspace",
        egui::FontId::proportional(11.0),
        if add_resp.hovered() {
            TEXT_PRIMARY
        } else {
            TEXT_MUTED
        },
    );
    if add_resp.clicked() {
        responses.push(SidebarResponse::CreateWorkspace);
    }

    responses
}
