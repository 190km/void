// Quick actions — minimal

use egui::{self, Color32, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum QuickAction {
    NewTerminal,
    CommandPalette,
}

#[allow(dead_code)]
pub fn draw_quick_actions(ui: &mut Ui) -> Option<QuickAction> {
    let mut action = None;

    let r1 = ui.horizontal(|ui| {
        ui.set_min_height(22.0);
        ui.add(
            egui::Label::new(
                egui::RichText::new("+ New terminal")
                    .color(Color32::from_rgb(100, 100, 100))
                    .size(11.0),
            )
            .selectable(false)
            .sense(egui::Sense::click()),
        )
    });
    if r1.inner.clicked() {
        action = Some(QuickAction::NewTerminal);
    }

    action
}
