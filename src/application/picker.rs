// App picker modal — select which application to embed.

use super::registry::{AppEntry, APPS};

#[derive(Default)]
pub struct AppPicker {
    pub open: bool,
}

impl AppPicker {
    /// Show the picker and return the selected app (if any).
    pub fn show(&mut self, ctx: &egui::Context) -> Option<&'static AppEntry> {
        if !self.open {
            return None;
        }

        let mut selected = None;

        egui::Window::new("Open Application")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    for app in APPS {
                        let btn = ui.add_sized(
                            [130.0, 100.0],
                            egui::Button::new(
                                egui::RichText::new(format!("{}\n{}", app.icon, app.name))
                                    .size(16.0)
                                    .color(egui::Color32::from_rgb(200, 200, 200)),
                            )
                            .fill(egui::Color32::from_rgb(30, 30, 30)),
                        );
                        if btn.clicked() {
                            selected = Some(app);
                            self.open = false;
                        }
                    }
                    ui.add_space(8.0);
                });
                ui.add_space(8.0);
            });

        // Close on Escape
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.open = false;
        }

        selected
    }
}
