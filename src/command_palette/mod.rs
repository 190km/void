// Command palette overlay — Ctrl+Shift+P

pub mod commands;
pub mod fuzzy;

use egui::{Color32, Key, Pos2, Rect, Vec2};

use self::commands::{Command, COMMANDS};

/// The command palette state.
pub struct CommandPalette {
    pub open: bool,
    pub query: String,
    pub selected_index: usize,
    filtered: Vec<usize>, // indices into COMMANDS
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self {
            open: false,
            query: String::new(),
            selected_index: 0,
            filtered: (0..COMMANDS.len()).collect(),
        }
    }
}

impl CommandPalette {
    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.query.clear();
            self.selected_index = 0;
            self.filtered = (0..COMMANDS.len()).collect();
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
    }

    fn update_filter(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..COMMANDS.len()).collect();
        } else {
            let mut scored: Vec<(usize, i32)> = COMMANDS
                .iter()
                .enumerate()
                .filter_map(|(i, entry)| {
                    fuzzy::fuzzy_score(&self.query, entry.label).map(|score| (i, score))
                })
                .collect();
            scored.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered = scored.into_iter().map(|(i, _)| i).collect();
        }
        // Clamp selection
        if self.selected_index >= self.filtered.len() {
            self.selected_index = self.filtered.len().saturating_sub(1);
        }
    }

    /// Show the command palette overlay. Returns a command if one was executed.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<Command> {
        if !self.open {
            return None;
        }

        let mut executed_command = None;

        // Handle keyboard navigation before rendering
        let mut close = false;
        ctx.input(|input| {
            if input.key_pressed(Key::Escape) {
                close = true;
            }
            if input.key_pressed(Key::ArrowDown) {
                if !self.filtered.is_empty() {
                    self.selected_index = (self.selected_index + 1) % self.filtered.len();
                }
            }
            if input.key_pressed(Key::ArrowUp) {
                if !self.filtered.is_empty() {
                    self.selected_index = if self.selected_index == 0 {
                        self.filtered.len() - 1
                    } else {
                        self.selected_index - 1
                    };
                }
            }
            if input.key_pressed(Key::Enter) {
                if let Some(&cmd_idx) = self.filtered.get(self.selected_index) {
                    executed_command = Some(COMMANDS[cmd_idx].command);
                }
            }
        });

        if close {
            self.close();
            return None;
        }

        if executed_command.is_some() {
            self.close();
            return executed_command;
        }

        // Draw overlay
        let screen = ctx.screen_rect();
        let palette_width = 500.0_f32.min(screen.width() - 40.0);
        let palette_x = screen.center().x - palette_width / 2.0;
        let palette_y = screen.min.y + 80.0;

        // Semi-transparent backdrop
        egui::Area::new(egui::Id::new("cmd_palette_backdrop"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let response = ui.allocate_response(screen.size(), egui::Sense::click());
                ui.painter()
                    .rect_filled(screen, 0.0, Color32::from_rgba_premultiplied(0, 0, 0, 120));
                if response.clicked() {
                    self.open = false;
                }
            });

        // Palette window
        egui::Area::new(egui::Id::new("cmd_palette"))
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::new(palette_x, palette_y))
            .show(ctx, |ui| {
                let frame_rect = Rect::from_min_size(
                    Pos2::new(palette_x, palette_y),
                    Vec2::new(palette_width, 400.0),
                );

                ui.painter().rect_filled(frame_rect, 8.0, Color32::from_rgb(20, 20, 20));
                ui.painter().rect_stroke(frame_rect, 8.0,
                    egui::Stroke::new(0.5, Color32::from_rgb(40, 40, 40)));

                ui.set_min_width(palette_width);
                ui.set_max_width(palette_width);

                ui.add_space(8.0);

                // Search input
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(">")
                            .color(Color32::from_rgb(160, 160, 160))
                            .size(16.0)
                            .strong(),
                    );
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .desired_width(palette_width - 50.0)
                            .font(egui::FontId::monospace(14.0))
                            .text_color(Color32::from_rgb(220, 220, 220))
                            .frame(false)
                            .hint_text("Type a command..."),
                    );
                    response.request_focus();

                    if response.changed() {
                        self.update_filter();
                    }
                });

                ui.add_space(4.0);

                // Separator
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().line_segment(
                        [
                            Pos2::new(rect.min.x, rect.min.y),
                            Pos2::new(rect.min.x + palette_width - 24.0, rect.min.y),
                        ],
                        egui::Stroke::new(1.0, Color32::from_rgb(50, 50, 50)),
                    );
                });

                ui.add_space(4.0);

                // Command list (scrollable)
                let max_visible = 10;
                let items_to_show = self.filtered.len().min(max_visible);

                for (display_idx, &cmd_idx) in
                    self.filtered.iter().take(items_to_show).enumerate()
                {
                    let entry = &COMMANDS[cmd_idx];
                    let is_selected = display_idx == self.selected_index;

                    let bg_color = if is_selected {
                        Color32::from_rgb(40, 40, 50)
                    } else {
                        Color32::TRANSPARENT
                    };

                    let item_rect = Rect::from_min_size(
                        Pos2::new(palette_x + 4.0, ui.cursor().min.y),
                        Vec2::new(palette_width - 8.0, 28.0),
                    );

                    if is_selected {
                        ui.painter().rect_filled(item_rect, 4.0, bg_color);
                    }

                    let _response = ui.horizontal(|ui| {
                        ui.add_space(16.0);

                        // Command label
                        ui.label(
                            egui::RichText::new(entry.label)
                                .color(if is_selected {
                                    Color32::WHITE
                                } else {
                                    Color32::from_rgb(200, 200, 200)
                                })
                                .size(13.0),
                        );

                        // Right-align shortcut
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(16.0);
                            ui.label(
                                egui::RichText::new(entry.shortcut)
                                    .color(Color32::from_rgb(100, 100, 100))
                                    .size(11.0),
                            );
                        });
                    });

                    // Click to execute
                    let click_response = ui.interact(
                        item_rect,
                        egui::Id::new("cmd_item").with(cmd_idx),
                        egui::Sense::click(),
                    );
                    if click_response.clicked() {
                        executed_command = Some(entry.command);
                    }
                    if click_response.hovered() {
                        self.selected_index = display_idx;
                    }
                }

                if self.filtered.is_empty() {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.label(
                            egui::RichText::new("No matching commands")
                                .color(Color32::from_rgb(100, 100, 100))
                                .size(13.0)
                                .italics(),
                        );
                    });
                }

                ui.add_space(8.0);
            });

        if executed_command.is_some() {
            self.close();
        }

        executed_command
    }
}
