// Terminal session list — minimal style

use crate::terminal::panel::TerminalPanel;
use egui::{self, Color32, Ui};
use uuid::Uuid;

#[allow(dead_code)]
pub fn draw_session_list(ui: &mut Ui, panels: &[TerminalPanel]) -> Option<Uuid> {
    let mut clicked = None;

    ui.label(
        egui::RichText::new(format!("Terminals  {}", panels.len()))
            .color(Color32::from_rgb(80, 80, 80))
            .size(10.0),
    );
    ui.add_space(4.0);

    for panel in panels {
        let alive = panel.is_alive();
        let tc = if panel.focused {
            Color32::WHITE
        } else if !alive {
            Color32::from_rgb(60, 60, 60)
        } else {
            Color32::from_rgb(160, 160, 160)
        };

        let resp = ui.horizontal(|ui| {
            ui.set_min_height(22.0);
            // Tiny dot
            let (dr, _) = ui.allocate_exact_size(egui::Vec2::splat(6.0), egui::Sense::hover());
            ui.painter().circle_filled(
                dr.center(),
                2.5,
                if alive {
                    panel
                        .color
                        .linear_multiply(if panel.focused { 1.0 } else { 0.5 })
                } else {
                    Color32::from_rgb(40, 40, 40)
                },
            );
            ui.add_space(2.0);
            ui.add(
                egui::Label::new(egui::RichText::new(&panel.title).color(tc).size(11.0))
                    .selectable(false)
                    .sense(egui::Sense::click())
                    .truncate(),
            )
        });
        if resp.inner.clicked() {
            clicked = Some(panel.id);
        }
    }

    clicked
}
