use eframe::egui;
use egui::{Color32, Pos2, Vec2};

use crate::canvas::viewport::Viewport;
use crate::command_palette::commands::Command;
use crate::command_palette::CommandPalette;
use crate::state::workspace::Workspace;
use crate::terminal::panel::PanelAction;

const PANEL_COLORS: &[Color32] = &[
    Color32::from_rgb(90, 130, 200),
    Color32::from_rgb(200, 90, 90),
    Color32::from_rgb(90, 180, 90),
    Color32::from_rgb(200, 160, 60),
    Color32::from_rgb(150, 90, 200),
    Color32::from_rgb(200, 120, 160),
    Color32::from_rgb(80, 170, 200),
    Color32::from_rgb(180, 180, 80),
];
const SIDEBAR_WIDTH: f32 = 260.0;

pub struct VoidApp {
    workspaces: Vec<Workspace>,
    active_ws: usize,
    viewport: Viewport,
    sidebar_visible: bool,
    show_grid: bool,
    show_minimap: bool,
    ctx: Option<egui::Context>,
    command_palette: CommandPalette,
    renaming_panel: Option<uuid::Uuid>,
    rename_buf: String,
    brand_texture: egui::TextureHandle,
}

impl VoidApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = cc.egui_ctx.clone();

        let brand_texture = {
            let png = include_bytes!("../assets/brand.png");
            let img = image::load_from_memory(png)
                .expect("Failed to load brand logo")
                .to_rgba8();
            let (w, h) = img.dimensions();
            ctx.load_texture(
                "brand_logo",
                egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], img.as_raw()),
                egui::TextureOptions::LINEAR,
            )
        };

        let mut ws = Workspace::new("Default", None);
        ws.spawn_terminal(&ctx, PANEL_COLORS);

        Self {
            workspaces: vec![ws],
            active_ws: 0,
            viewport: Viewport {
                pan: Vec2::new(100.0, 50.0),
                zoom: 0.65,
            },
            sidebar_visible: true,
            show_grid: true,
            show_minimap: true,
            ctx: Some(ctx),
            command_palette: CommandPalette::default(),
            renaming_panel: None,
            rename_buf: String::new(),
            brand_texture,
        }
    }

    fn ws(&self) -> &Workspace {
        &self.workspaces[self.active_ws]
    }
    fn ws_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active_ws]
    }

    fn current_canvas_rect(&self, screen_rect: egui::Rect) -> egui::Rect {
        let mut canvas_rect = screen_rect;
        if self.sidebar_visible {
            canvas_rect.min.x += SIDEBAR_WIDTH;
        }
        canvas_rect
    }

    fn switch_workspace(&mut self, idx: usize) {
        if idx >= self.workspaces.len() || idx == self.active_ws {
            return;
        }
        // Save viewport
        self.workspaces[self.active_ws].viewport_pan = self.viewport.pan;
        self.workspaces[self.active_ws].viewport_zoom = self.viewport.zoom;
        self.active_ws = idx;
        // Restore viewport
        self.viewport.pan = self.workspaces[idx].viewport_pan;
        self.viewport.zoom = self.workspaces[idx].viewport_zoom;
    }

    fn create_workspace_with_picker(&mut self) {
        let dir = rfd::FileDialog::new()
            .set_title("Choose workspace directory")
            .pick_folder();

        if let Some(path) = dir {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Workspace".to_string());

            let mut ws = Workspace::new(name, Some(path));
            if let Some(ctx) = &self.ctx {
                ws.spawn_terminal(ctx, PANEL_COLORS);
            }

            self.workspaces.push(ws);
            let new_idx = self.workspaces.len() - 1;
            self.switch_workspace(new_idx);
        }
    }

    fn spawn_terminal(&mut self) {
        if let Some(ctx) = self.ctx.clone() {
            self.ws_mut().spawn_terminal(&ctx, PANEL_COLORS);
        }
    }

    fn execute_command(&mut self, cmd: Command, screen_rect: egui::Rect) {
        match cmd {
            Command::NewTerminal => self.spawn_terminal(),
            Command::CloseTerminal => self.ws_mut().close_focused(),
            Command::RenameTerminal => {
                let found = self
                    .ws()
                    .panels
                    .iter()
                    .find(|p| p.focused)
                    .map(|p| (p.id, p.title.clone()));
                if let Some((id, title)) = found {
                    self.renaming_panel = Some(id);
                    self.rename_buf = title;
                }
            }
            Command::ToggleSidebar => self.sidebar_visible = !self.sidebar_visible,
            Command::ToggleMinimap => self.show_minimap = !self.show_minimap,
            Command::ToggleGrid => self.show_grid = !self.show_grid,
            Command::ZoomIn => self.viewport.zoom = (self.viewport.zoom * 1.2).min(4.0),
            Command::ZoomOut => self.viewport.zoom = (self.viewport.zoom / 1.2).max(0.1),
            Command::ZoomReset => {
                self.viewport.zoom = 1.0;
                self.viewport.pan = Vec2::ZERO;
            }
            Command::ZoomToFit => self.zoom_to_fit(screen_rect),
            Command::FocusNext => self.ws_mut().focus_next(),
            Command::FocusPrev => self.ws_mut().focus_prev(),
            Command::ToggleFullscreen => {}
        }
    }

    fn zoom_to_fit(&mut self, screen_rect: egui::Rect) {
        let panels = &self.ws().panels;
        if panels.is_empty() {
            return;
        }
        let mut min = Pos2::new(f32::MAX, f32::MAX);
        let mut max = Pos2::new(f32::MIN, f32::MIN);
        for p in panels {
            let r = p.rect();
            min.x = min.x.min(r.min.x);
            min.y = min.y.min(r.min.y);
            max.x = max.x.max(r.max.x);
            max.y = max.y.max(r.max.y);
        }
        let cw = max.x - min.x;
        let ch = max.y - min.y;
        if cw <= 0.0 || ch <= 0.0 {
            return;
        }
        let m = 80.0;
        self.viewport.zoom = ((screen_rect.width() - m * 2.0) / cw)
            .min((screen_rect.height() - m * 2.0) / ch)
            .clamp(0.1, 4.0);
        self.viewport.pan_to_center(
            Pos2::new((min.x + max.x) / 2.0, (min.y + max.y) / 2.0),
            screen_rect,
        );
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) -> Option<Command> {
        if self.command_palette.open {
            return None;
        }
        let mut cmd = None;
        ctx.input(|i| {
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::P) {
            } else if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::B) {
                cmd = Some(Command::ToggleSidebar);
            } else if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::M) {
                cmd = Some(Command::ToggleMinimap);
            } else if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::G) {
                cmd = Some(Command::ToggleGrid);
            } else if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::T) {
                cmd = Some(Command::NewTerminal);
            } else if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::W) {
                cmd = Some(Command::CloseTerminal);
            } else if i.modifiers.ctrl
                && i.modifiers.shift
                && i.key_pressed(egui::Key::CloseBracket)
            {
                cmd = Some(Command::FocusNext);
            } else if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::OpenBracket)
            {
                cmd = Some(Command::FocusPrev);
            } else if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Num0) {
                cmd = Some(Command::ZoomToFit);
            } else if i.key_pressed(egui::Key::F2) && !i.modifiers.ctrl {
                cmd = Some(Command::RenameTerminal);
            }
        });
        cmd
    }
}

impl eframe::App for VoidApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.ctx.is_none() {
            self.ctx = Some(ctx.clone());
        }
        let screen_rect = ctx.screen_rect();
        let canvas_rect_for_commands = self.current_canvas_rect(screen_rect);

        // Command palette toggle
        if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::P)) {
            self.command_palette.toggle();
        }

        if let Some(cmd) = self.handle_shortcuts(ctx) {
            self.execute_command(cmd, canvas_rect_for_commands);
        }

        // Sync titles
        for p in &mut self.ws_mut().panels {
            p.sync_title();
        }

        // Keyboard input to focused terminal
        if !self.command_palette.open && self.renaming_panel.is_none() {
            for p in &self.ws().panels {
                if p.focused {
                    p.handle_input(ctx);
                    break;
                }
            }
        }

        // Command palette
        if let Some(cmd) = self.command_palette.show(ctx) {
            self.execute_command(cmd, canvas_rect_for_commands);
        }

        // Rename dialog
        if let Some(rename_id) = self.renaming_panel {
            let mut close = false;
            egui::Area::new(egui::Id::new("rename_dialog"))
                .order(egui::Order::Foreground)
                .fixed_pos(Pos2::new(
                    screen_rect.center().x - 150.0,
                    screen_rect.min.y + 120.0,
                ))
                .show(ctx, |ui| {
                    egui::Frame::default()
                        .fill(Color32::from_rgb(20, 20, 20))
                        .stroke(egui::Stroke::new(0.5, Color32::from_rgb(40, 40, 40)))
                        .rounding(8.0)
                        .inner_margin(14.0)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("Rename")
                                    .color(Color32::from_rgb(160, 160, 160))
                                    .size(12.0),
                            );
                            ui.add_space(6.0);
                            let r = ui.add(
                                egui::TextEdit::singleline(&mut self.rename_buf)
                                    .desired_width(280.0)
                                    .font(egui::FontId::monospace(12.0)),
                            );
                            r.request_focus();
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if ui.button("OK").clicked()
                                    || ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    let buf = self.rename_buf.clone();
                                    if let Some(p) =
                                        self.ws_mut().panels.iter_mut().find(|p| p.id == rename_id)
                                    {
                                        p.title = buf;
                                    }
                                    close = true;
                                }
                                if ui.button("Cancel").clicked()
                                    || ui.input(|i| i.key_pressed(egui::Key::Escape))
                                {
                                    close = true;
                                }
                            });
                        });
                });
            if close {
                self.renaming_panel = None;
                self.rename_buf.clear();
            }
        }

        // --- Sidebar ---
        if self.sidebar_visible {
            egui::SidePanel::left("sidebar")
                .exact_width(SIDEBAR_WIDTH)
                .frame(
                    egui::Frame::default()
                        .fill(Color32::from_rgb(18, 18, 18))
                        .stroke(egui::Stroke::new(0.5, Color32::from_rgb(35, 35, 35)))
                        .inner_margin(egui::Margin::symmetric(14.0, 0.0)),
                )
                .show(ctx, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.add_space(14.0);
                    ui.add(
                        egui::Image::new(egui::load::SizedTexture::new(
                            self.brand_texture.id(),
                            self.brand_texture.size_vec2(),
                        ))
                        .max_height(14.0)
                        .tint(Color32::from_rgb(140, 140, 140)),
                    );
                    ui.add_space(14.0);

                    // Workspaces + terminals dropdown
                    let mut ws_action = None;
                    let mut spawn_terminal = false;
                    {
                        use crate::sidebar::workspace_list::WorkspaceAction;

                        // Build a minimal workspace manager view for the widget
                        let names: Vec<(String, bool)> = self
                            .workspaces
                            .iter()
                            .enumerate()
                            .map(|(i, ws)| (ws.name.clone(), i == self.active_ws))
                            .collect();

                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Workspaces")
                                    .color(Color32::from_rgb(80, 80, 80))
                                    .size(10.0),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let r = ui.add(
                                        egui::Label::new(
                                            egui::RichText::new("+")
                                                .color(Color32::from_rgb(80, 80, 80))
                                                .size(13.0),
                                        )
                                        .selectable(false)
                                        .sense(egui::Sense::click()),
                                    );
                                    if r.clicked() {
                                        ws_action = Some(WorkspaceAction::Create);
                                    }
                                },
                            );
                        });
                        ui.add_space(4.0);

                        let mut clicked_panel: Option<(usize, uuid::Uuid)> = None;

                        for (i, (name, active)) in names.iter().enumerate() {
                            let tc = if *active {
                                Color32::WHITE
                            } else {
                                Color32::from_rgb(120, 120, 120)
                            };

                            // Workspace row
                            let resp = ui.horizontal(|ui| {
                                ui.set_min_height(22.0);
                                // Arrow indicator (expanded for active)
                                let arrow = if *active { "▾" } else { "▸" };
                                ui.label(
                                    egui::RichText::new(arrow)
                                        .color(Color32::from_rgb(60, 60, 60))
                                        .size(9.0),
                                );
                                // Color dot
                                let (dr, _) = ui.allocate_exact_size(
                                    egui::Vec2::splat(6.0),
                                    egui::Sense::hover(),
                                );
                                let dc = if *active {
                                    Color32::from_rgb(90, 130, 200)
                                } else {
                                    Color32::from_rgb(50, 50, 50)
                                };
                                ui.painter().circle_filled(dr.center(), 2.5, dc);
                                ui.add_space(2.0);
                                let label = if let Some(cwd) = &self.workspaces[i].cwd {
                                    format!(
                                        "{}",
                                        cwd.file_name()
                                            .map(|n| n.to_string_lossy())
                                            .unwrap_or(std::borrow::Cow::Borrowed(name))
                                    )
                                } else {
                                    name.clone()
                                };
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(label).color(tc).size(11.0),
                                    )
                                    .selectable(false)
                                    .sense(egui::Sense::click()),
                                )
                            });
                            if resp.inner.clicked() && !*active {
                                ws_action = Some(WorkspaceAction::Switch(i));
                            }
                            resp.inner.context_menu(|ui| {
                                if self.workspaces.len() > 1 && ui.button("Delete").clicked() {
                                    ws_action = Some(WorkspaceAction::Delete(i));
                                    ui.close_menu();
                                }
                            });

                            // Dropdown: terminals for this workspace (only shown for active)
                            if *active {
                                for panel in &self.workspaces[i].panels {
                                    let ptc = if panel.focused {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(100, 100, 100)
                                    };
                                    let pr = ui.horizontal(|ui| {
                                        ui.set_min_height(20.0);
                                        ui.add_space(18.0); // indent
                                        let (dr, _) = ui.allocate_exact_size(
                                            egui::Vec2::splat(5.0),
                                            egui::Sense::hover(),
                                        );
                                        let dot_c = if panel.is_alive() {
                                            panel.color.linear_multiply(0.6)
                                        } else {
                                            Color32::from_rgb(40, 40, 40)
                                        };
                                        ui.painter().circle_filled(dr.center(), 2.0, dot_c);
                                        ui.add_space(2.0);
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(&panel.title)
                                                    .color(ptc)
                                                    .size(10.0),
                                            )
                                            .selectable(false)
                                            .sense(egui::Sense::click())
                                            .truncate(),
                                        )
                                    });
                                    if pr.inner.clicked() {
                                        clicked_panel = Some((i, panel.id));
                                    }
                                }
                                // + New terminal under this workspace
                                let nr = ui.horizontal(|ui| {
                                    ui.set_min_height(20.0);
                                    ui.add_space(18.0);
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new("+ terminal")
                                                .color(Color32::from_rgb(60, 60, 60))
                                                .size(10.0),
                                        )
                                        .selectable(false)
                                        .sense(egui::Sense::click()),
                                    )
                                });
                                if nr.inner.clicked() {
                                    spawn_terminal = true;
                                }
                            }

                            ui.add_space(2.0);
                        }

                        // Handle panel click (pan to it)
                        if let Some((_ws_idx, panel_id)) = clicked_panel {
                            if let Some(p) = self.ws().panels.iter().find(|p| p.id == panel_id) {
                                let center = p.rect().center();
                                self.viewport.pan_to_center(
                                    center,
                                    self.current_canvas_rect(ctx.screen_rect()),
                                );
                            }
                            if let Some(idx) =
                                self.ws().panels.iter().position(|p| p.id == panel_id)
                            {
                                self.ws_mut().bring_to_front(idx);
                            }
                        }
                    }

                    // Process workspace actions
                    if let Some(action) = ws_action {
                        use crate::sidebar::workspace_list::WorkspaceAction;
                        match action {
                            WorkspaceAction::Switch(idx) => self.switch_workspace(idx),
                            WorkspaceAction::Create => self.create_workspace_with_picker(),
                            WorkspaceAction::Delete(idx) => {
                                if self.workspaces.len() > 1 {
                                    self.workspaces.remove(idx);
                                    if self.active_ws >= self.workspaces.len() {
                                        self.active_ws = self.workspaces.len() - 1;
                                    } else if self.active_ws > idx {
                                        self.active_ws -= 1;
                                    }
                                    self.viewport.pan =
                                        self.workspaces[self.active_ws].viewport_pan;
                                    self.viewport.zoom =
                                        self.workspaces[self.active_ws].viewport_zoom;
                                }
                            }
                        }
                    }
                    if spawn_terminal {
                        self.spawn_terminal();
                    }

                    // Bottom
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new("Ctrl+Shift+T new · Ctrl+B sidebar · Ctrl+M minimap")
                                .color(Color32::from_rgb(50, 50, 50))
                                .size(9.5),
                        );
                        ui.add_space(6.0);
                    });
                });
        }

        // --- Canvas ---
        // Wheel ownership is hover-based: topmost terminal under the pointer gets scroll, otherwise canvas pans.
        let mut canvas_rect = self.current_canvas_rect(screen_rect);

        // We need canvas_rect from CentralPanel first, then compute transform
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(Color32::from_rgb(10, 10, 10)))
            .show(ctx, |ui| {
                canvas_rect = ui.available_rect_before_wrap();
            });

        let hovered_terminal = ctx.input(|input| {
            let pointer_pos = input.pointer.hover_pos()?;
            if !canvas_rect.contains(pointer_pos) {
                return None;
            }

            let pointer_canvas = self.viewport.screen_to_canvas(pointer_pos, canvas_rect);
            self.ws()
                .panels
                .iter()
                .enumerate()
                .filter(|(_, panel)| panel.scroll_hit_test(pointer_canvas))
                .max_by_key(|(_, panel)| panel.z_index)
                .map(|(idx, _)| idx)
        });

        if let Some(idx) = hovered_terminal {
            let scroll_y = ctx.input(|input| input.smooth_scroll_delta.y);
            if scroll_y != 0.0 {
                self.ws_mut().panels[idx].handle_scroll(ctx, scroll_y);
                ctx.input_mut(|input| {
                    input.smooth_scroll_delta = egui::Vec2::ZERO;
                    input.raw_scroll_delta = egui::Vec2::ZERO;
                });
            }
        }

        // Canvas input (grid, pan/zoom) — drawn on a background area
        egui::Area::new(egui::Id::new("canvas_bg_area"))
            .order(egui::Order::Background)
            .fixed_pos(canvas_rect.min)
            .interactable(true)
            .show(ctx, |ui| {
                ui.set_clip_rect(canvas_rect);
                let (_, bg_resp) = ui.allocate_exact_size(canvas_rect.size(), egui::Sense::click());

                crate::canvas::scene::handle_canvas_input(
                    ui,
                    &mut self.viewport,
                    canvas_rect,
                    hovered_terminal.is_some(),
                );

                if self.show_grid {
                    crate::canvas::grid::draw_dot_grid(ui, &self.viewport, canvas_rect);
                }

                if self.show_minimap {
                    let minimap = crate::canvas::minimap::draw_minimap(
                        ui,
                        &self.viewport,
                        canvas_rect,
                        &self.ws().panels,
                    );
                    if let Some(nav) = minimap.navigate_to {
                        self.viewport.pan_to_center(nav, canvas_rect);
                    }
                    if minimap.hide_clicked {
                        self.show_minimap = false;
                    }
                }

                // Unfocus when clicking empty canvas
                if bg_resp.clicked_by(egui::PointerButton::Primary) {
                    for p in &mut self.ws_mut().panels {
                        p.focused = false;
                    }
                }

                // Status bar: zoom level + pointer canvas coordinates (bottom-left)
                let zoom_pct = format!("{:.0}%", self.viewport.zoom * 100.0);
                let pointer_info = if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                    if canvas_rect.contains(pos) {
                        let canvas_pos = self.viewport.screen_to_canvas(pos, canvas_rect);
                        format!("  x:{:.0}  y:{:.0}", canvas_pos.x, canvas_pos.y)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                let status_text = format!("{}{}", zoom_pct, pointer_info);
                let status_pos = Pos2::new(canvas_rect.min.x + 8.0, canvas_rect.max.y - 18.0);
                ui.painter().text(
                    status_pos,
                    egui::Align2::LEFT_TOP,
                    status_text,
                    egui::FontId::monospace(10.0),
                    Color32::from_rgb(60, 60, 60),
                );
            });

        // --- Canvas content (transformed layer with panels) ---
        // Recompute transform (viewport may have changed from pan/zoom input above)
        let transform = self.viewport.transform(canvas_rect);
        let clip = transform.inverse() * canvas_rect;

        egui::Area::new(egui::Id::new("canvas_content"))
            .order(egui::Order::Middle)
            .fixed_pos(Pos2::ZERO)
            .interactable(true)
            .show(ctx, |ui| {
                ctx.set_transform_layer(ui.layer_id(), transform);
                ui.set_clip_rect(clip);
                ui.allocate_rect(clip, egui::Sense::hover());

                let mut order: Vec<usize> = (0..self.ws().panels.len()).collect();
                order.sort_by_key(|&i| self.ws().panels[i].z_index);

                let mut interactions = Vec::new();
                for &idx in &order {
                    if !self
                        .viewport
                        .is_visible(self.ws().panels[idx].rect(), canvas_rect)
                    {
                        continue;
                    }
                    let ix = self.ws_mut().panels[idx].show(ui);
                    if ix.clicked || ix.dragging_title || ix.resizing || ix.action.is_some() {
                        interactions.push((idx, ix));
                    }
                }

                // Process interactions with snap guides
                let mut to_close = Vec::new();
                let mut snap_guides: Vec<crate::canvas::snap::SnapGuide> = Vec::new();

                for (idx, ix) in &interactions {
                    if ix.clicked {
                        self.ws_mut().bring_to_front(*idx);
                    }
                    if ix.dragging_title {
                        // Collect other panel rects for snapping
                        let moving = self.ws().panels[*idx].rect();
                        let others: Vec<egui::Rect> = self
                            .ws()
                            .panels
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| i != idx)
                            .map(|(_, p)| p.rect())
                            .collect();
                        let result = crate::canvas::snap::snap_drag(moving, &others, ix.drag_delta);
                        self.ws_mut().panels[*idx].apply_drag(result.delta);
                        snap_guides = result.guides;
                    }
                    if ix.resizing {
                        self.ws_mut().panels[*idx].apply_resize(ix.resize_delta);
                    }
                    if let Some(action) = &ix.action {
                        match action {
                            PanelAction::Close => to_close.push(*idx),
                            PanelAction::Rename => {
                                self.renaming_panel = Some(self.ws().panels[*idx].id);
                                self.rename_buf = self.ws().panels[*idx].title.clone();
                            }
                        }
                    }
                }
                to_close.sort_unstable();
                for idx in to_close.into_iter().rev() {
                    self.ws_mut().close_panel(idx);
                }

                // Draw snap guides
                let painter = ui.painter();
                let guide_stroke =
                    egui::Stroke::new(1.0, Color32::from_rgba_premultiplied(100, 160, 255, 150));
                for guide in &snap_guides {
                    if guide.vertical {
                        painter.line_segment(
                            [
                                Pos2::new(guide.position, guide.start),
                                Pos2::new(guide.position, guide.end),
                            ],
                            guide_stroke,
                        );
                    } else {
                        painter.line_segment(
                            [
                                Pos2::new(guide.start, guide.position),
                                Pos2::new(guide.end, guide.position),
                            ],
                            guide_stroke,
                        );
                    }
                }
            });
    }
}
