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
            if input.key_pressed(Key::ArrowDown) && !self.filtered.is_empty() {
                self.selected_index = (self.selected_index + 1) % self.filtered.len();
            }
            if input.key_pressed(Key::ArrowUp) && !self.filtered.is_empty() {
                self.selected_index = if self.selected_index == 0 {
                    self.filtered.len() - 1
                } else {
                    self.selected_index - 1
                };
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
                ui.painter().rect_filled(
                    screen,
                    0.0,
                    Color32::from_rgba_premultiplied(0, 0, 0, 150),
                );
                if response.clicked() {
                    self.open = false;
                }
            });

        // Palette window
        egui::Area::new(egui::Id::new("cmd_palette"))
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::new(palette_x, palette_y))
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(Color32::from_rgb(24, 24, 28))
                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(55, 55, 65)))
                    .rounding(10.0)
                    .inner_margin(egui::Margin::same(0.0))
                    .show(ui, |ui| {
                        ui.set_min_width(palette_width);
                        ui.set_max_width(palette_width);

                        ui.add_space(12.0);

                        // Search input row
                        ui.horizontal(|ui| {
                            ui.add_space(16.0);
                            ui.label(
                                egui::RichText::new(">")
                                    .color(Color32::from_rgb(130, 130, 200))
                                    .size(16.0)
                                    .strong(),
                            );
                            ui.add_space(4.0);
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut self.query)
                                    .desired_width(palette_width - 60.0)
                                    .font(egui::FontId::monospace(14.0))
                                    .text_color(Color32::from_rgb(220, 220, 225))
                                    .frame(false)
                                    .hint_text("Type a command..."),
                            );
                            response.request_focus();

                            if response.changed() {
                                self.update_filter();
                            }
                        });

                        ui.add_space(8.0);

                        // Separator
                        let sep_rect = ui.available_rect_before_wrap();
                        ui.painter().line_segment(
                            [
                                Pos2::new(sep_rect.min.x, sep_rect.min.y),
                                Pos2::new(sep_rect.min.x + palette_width, sep_rect.min.y),
                            ],
                            egui::Stroke::new(1.0, Color32::from_rgb(45, 45, 52)),
                        );

                        ui.add_space(6.0);

                        // Command list
                        let max_visible = 10;
                        let items_to_show = self.filtered.len().min(max_visible);
                        let row_height = 32.0;

                        for (display_idx, &cmd_idx) in
                            self.filtered.iter().take(items_to_show).enumerate()
                        {
                            let entry = &COMMANDS[cmd_idx];
                            let is_selected = display_idx == self.selected_index;

                            let (row_rect, row_response) = ui.allocate_exact_size(
                                Vec2::new(palette_width, row_height),
                                egui::Sense::click(),
                            );

                            // Selection highlight
                            if is_selected || row_response.hovered() {
                                let hl = Rect::from_min_max(
                                    Pos2::new(row_rect.min.x + 6.0, row_rect.min.y + 1.0),
                                    Pos2::new(row_rect.max.x - 6.0, row_rect.max.y - 1.0),
                                );
                                ui.painter().rect_filled(
                                    hl,
                                    6.0,
                                    if is_selected {
                                        Color32::from_rgb(45, 45, 62)
                                    } else {
                                        Color32::from_rgb(35, 35, 45)
                                    },
                                );
                            }

                            // Label — left aligned, vertically centered
                            ui.painter().text(
                                Pos2::new(row_rect.min.x + 20.0, row_rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                entry.label,
                                egui::FontId::proportional(13.0),
                                if is_selected {
                                    Color32::WHITE
                                } else {
                                    Color32::from_rgb(195, 195, 200)
                                },
                            );

                            // Shortcut badge — right aligned, vertically centered
                            if !entry.shortcut.is_empty() {
                                let shortcut_font = egui::FontId::monospace(10.5);
                                let shortcut_galley = ui.painter().layout_no_wrap(
                                    entry.shortcut.to_string(),
                                    shortcut_font.clone(),
                                    Color32::from_rgb(120, 120, 135),
                                );
                                let text_w = shortcut_galley.rect.width();
                                let text_h = shortcut_galley.rect.height();
                                let badge_pad_x = 7.0;
                                let badge_pad_y = 3.0;
                                let badge_rect = Rect::from_min_size(
                                    Pos2::new(
                                        row_rect.max.x - 18.0 - text_w - badge_pad_x * 2.0,
                                        row_rect.center().y - (text_h + badge_pad_y * 2.0) / 2.0,
                                    ),
                                    Vec2::new(
                                        text_w + badge_pad_x * 2.0,
                                        text_h + badge_pad_y * 2.0,
                                    ),
                                );
                                ui.painter().rect_filled(
                                    badge_rect,
                                    4.0,
                                    Color32::from_rgb(35, 35, 42),
                                );
                                ui.painter().rect_stroke(
                                    badge_rect,
                                    4.0,
                                    egui::Stroke::new(0.5, Color32::from_rgb(60, 60, 70)),
                                );
                                ui.painter().galley(
                                    Pos2::new(
                                        badge_rect.min.x + badge_pad_x,
                                        badge_rect.min.y + badge_pad_y,
                                    ),
                                    shortcut_galley,
                                    Color32::TRANSPARENT,
                                );
                            }

                            if row_response.clicked() {
                                executed_command = Some(entry.command);
                            }
                            if row_response.hovered() {
                                self.selected_index = display_idx;
                            }
                        }

                        if self.filtered.is_empty() {
                            let (empty_rect, _) = ui.allocate_exact_size(
                                Vec2::new(palette_width, 36.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().text(
                                Pos2::new(empty_rect.min.x + 20.0, empty_rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                "No matching commands",
                                egui::FontId::proportional(13.0),
                                Color32::from_rgb(100, 100, 110),
                            );
                        }

                        ui.add_space(6.0);
                    });
            });

        if executed_command.is_some() {
            self.close();
        }

        executed_command
    }
}
