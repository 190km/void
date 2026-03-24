use eframe::egui;
use egui::{Color32, Pos2, Vec2};

use crate::canvas::viewport::Viewport;
use crate::command_palette::commands::Command;
use crate::command_palette::CommandPalette;
use crate::sidebar::{Sidebar, SidebarResponse, SIDEBAR_BG, SIDEBAR_BORDER, SIDEBAR_PADDING_H};
use crate::state::workspace::Workspace;
use crate::terminal::panel::PanelAction;
use crate::update::UpdateChecker;

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
    font_size: f32,
    ctx: Option<egui::Context>,
    command_palette: CommandPalette,
    renaming_panel: Option<uuid::Uuid>,
    rename_buf: String,
    brand_texture: egui::TextureHandle,
    sidebar: Sidebar,
    update_checker: UpdateChecker,
}

impl VoidApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = cc.egui_ctx.clone();
        Self::setup_fonts(&ctx);

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

        // Try to restore saved layout, otherwise create a default workspace
        let (workspaces, active_ws, sidebar_visible, show_grid, show_minimap, viewport, font_size) =
            if let Some(saved) = crate::state::persistence::load_state() {
                let fs = saved.font_size;
                let wss: Vec<Workspace> = saved
                    .workspaces
                    .iter()
                    .map(|ws_state| Workspace::from_saved(&ctx, ws_state, PANEL_COLORS, fs))
                    .collect();
                let active = saved.active_ws.min(wss.len().saturating_sub(1));
                let vp = Viewport {
                    pan: Vec2::new(wss[active].viewport_pan.x, wss[active].viewport_pan.y),
                    zoom: wss[active].viewport_zoom,
                };
                (
                    wss,
                    active,
                    saved.sidebar_visible,
                    saved.show_grid,
                    saved.show_minimap,
                    vp,
                    fs,
                )
            } else {
                let fs = crate::terminal::renderer::DEFAULT_FONT_SIZE;
                let mut ws = Workspace::new("Default", None);
                ws.spawn_terminal(&ctx, PANEL_COLORS, fs);
                (
                    vec![ws],
                    0,
                    true,
                    true,
                    true,
                    Viewport {
                        pan: Vec2::new(100.0, 50.0),
                        zoom: 0.75,
                    },
                    fs,
                )
            };

        Self {
            workspaces,
            active_ws,
            viewport,
            sidebar_visible,
            show_grid,
            show_minimap,
            font_size,
            ctx: Some(ctx),
            command_palette: CommandPalette::default(),
            renaming_panel: None,
            rename_buf: String::new(),
            brand_texture,
            sidebar: Sidebar::default(),
            update_checker: UpdateChecker::new(cc.egui_ctx.clone()),
        }
    }

    /// Load system fonts as fallback for better Unicode coverage (box-drawing, symbols, etc.)
    fn setup_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();

        // System font paths with good Unicode coverage, in preference order
        #[cfg(target_os = "windows")]
        let fallback_paths: &[&str] = &[
            "C:\\Windows\\Fonts\\seguisym.ttf", // Segoe UI Symbol
            "C:\\Windows\\Fonts\\segoeui.ttf",  // Segoe UI
        ];
        #[cfg(target_os = "macos")]
        let fallback_paths: &[&str] = &[
            "/System/Library/Fonts/Apple Symbols.ttf",
            "/System/Library/Fonts/Menlo.ttc",
        ];
        #[cfg(target_os = "linux")]
        let fallback_paths: &[&str] = &[
            "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
        ];

        for (i, path) in fallback_paths.iter().enumerate() {
            if let Ok(data) = std::fs::read(path) {
                let name = format!("fallback_{}", i);
                fonts
                    .font_data
                    .insert(name.clone(), egui::FontData::from_owned(data).into());
                if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                    family.push(name.clone());
                }
                if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                    family.push(name);
                }
            }
        }

        ctx.set_fonts(fonts);
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
                ws.spawn_terminal(ctx, PANEL_COLORS, self.font_size);
            }

            self.workspaces.push(ws);
            let new_idx = self.workspaces.len() - 1;
            self.switch_workspace(new_idx);
        }
    }

    fn spawn_terminal(&mut self) {
        if let Some(ctx) = self.ctx.clone() {
            let font_size = self.font_size;
            self.ws_mut().spawn_terminal(&ctx, PANEL_COLORS, font_size);
        }
    }

    fn execute_command(&mut self, cmd: Command, ctx: &egui::Context, screen_rect: egui::Rect) {
        match cmd {
            Command::NewTerminal => self.spawn_terminal(),
            Command::CloseTerminal => self.ws_mut().close_focused(),
            Command::RenameTerminal => {
                let found = self
                    .ws()
                    .panels
                    .iter()
                    .find(|p| p.focused())
                    .map(|p| (p.id(), p.title().to_string()));
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
            Command::FontZoomIn => {
                use crate::terminal::renderer::{FONT_SIZE_STEP, MAX_FONT_SIZE};
                self.font_size = (self.font_size + FONT_SIZE_STEP).min(MAX_FONT_SIZE);
            }
            Command::FontZoomOut => {
                use crate::terminal::renderer::{FONT_SIZE_STEP, MIN_FONT_SIZE};
                self.font_size = (self.font_size - FONT_SIZE_STEP).max(MIN_FONT_SIZE);
            }
            Command::FontZoomReset => {
                self.font_size = crate::terminal::renderer::DEFAULT_FONT_SIZE;
            }
            Command::FocusNext => self.ws_mut().focus_next(),
            Command::FocusPrev => self.ws_mut().focus_prev(),
            Command::ToggleFullscreen => {
                let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
            }
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
            } else if i.modifiers.ctrl
                && i.modifiers.shift
                && (i.key_pressed(egui::Key::Equals) || i.key_pressed(egui::Key::Plus))
            {
                cmd = Some(Command::FontZoomIn);
            } else if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Minus) {
                cmd = Some(Command::FontZoomOut);
            } else if i.key_pressed(egui::Key::F2) && !i.modifiers.ctrl {
                cmd = Some(Command::RenameTerminal);
            }
        });
        cmd
    }
}

impl VoidApp {
    fn snapshot_state(&self) -> crate::state::persistence::AppState {
        // Save current viewport into the active workspace snapshot
        let workspaces: Vec<_> = self
            .workspaces
            .iter()
            .enumerate()
            .map(|(i, ws)| {
                let mut saved = ws.to_saved();
                if i == self.active_ws {
                    saved.viewport_pan = [self.viewport.pan.x, self.viewport.pan.y];
                    saved.viewport_zoom = self.viewport.zoom;
                }
                saved
            })
            .collect();
        crate::state::persistence::AppState {
            workspaces,
            active_ws: self.active_ws,
            sidebar_visible: self.sidebar_visible,
            show_grid: self.show_grid,
            show_minimap: self.show_minimap,
            font_size: self.font_size,
        }
    }
}

impl eframe::App for VoidApp {
    fn on_exit(&mut self) {
        crate::state::persistence::save_state(&self.snapshot_state());
    }

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
            self.execute_command(cmd, ctx, canvas_rect_for_commands);
        }

        // Sync titles
        for p in &mut self.ws_mut().panels {
            p.sync_title();
        }

        // Keyboard input to focused terminal
        if !self.command_palette.open && self.renaming_panel.is_none() {
            for p in &mut self.ws_mut().panels {
                if p.focused() {
                    p.handle_input(ctx);
                    break;
                }
            }
        }

        // Command palette
        if let Some(cmd) = self.command_palette.show(ctx) {
            self.execute_command(cmd, ctx, canvas_rect_for_commands);
        }

        // Rename dialog
        if let Some(rename_id) = self.renaming_panel {
            let mut close = false;
            egui::Area::new(egui::Id::new("rename_dialog"))
                .order(egui::Order::Debug)
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
                                    if let Some(p) = self
                                        .ws_mut()
                                        .panels
                                        .iter_mut()
                                        .find(|p| p.id() == rename_id)
                                    {
                                        p.set_title(buf);
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
                        .fill(SIDEBAR_BG)
                        .stroke(egui::Stroke::new(0.5, SIDEBAR_BORDER))
                        .inner_margin(egui::Margin::symmetric(SIDEBAR_PADDING_H, 0.0)),
                )
                .show(ctx, |ui| {
                    let update_state = self.update_checker.state();
                    let responses = self.sidebar.show(
                        ui,
                        &self.workspaces,
                        self.active_ws,
                        &self.brand_texture,
                        &update_state,
                        &self.update_checker,
                    );
                    for resp in responses {
                        match resp {
                            SidebarResponse::SwitchWorkspace(idx) => {
                                self.switch_workspace(idx);
                            }
                            SidebarResponse::CreateWorkspace => {
                                self.create_workspace_with_picker();
                            }
                            SidebarResponse::DeleteWorkspace(idx) => {
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
                            SidebarResponse::FocusPanel { panel_id } => {
                                if let Some(p) =
                                    self.ws().panels.iter().find(|p| p.id() == panel_id)
                                {
                                    let center = p.rect().center();
                                    self.viewport.pan_to_center(
                                        center,
                                        self.current_canvas_rect(ctx.screen_rect()),
                                    );
                                }
                                if let Some(idx) =
                                    self.ws().panels.iter().position(|p| p.id() == panel_id)
                                {
                                    self.ws_mut().bring_to_front(idx);
                                }
                            }
                            SidebarResponse::SpawnTerminal => {
                                self.spawn_terminal();
                            }
                            SidebarResponse::RenamePanel(id) => {
                                let title = self
                                    .ws()
                                    .panels
                                    .iter()
                                    .find(|p| p.id() == id)
                                    .map(|p| p.title().to_string());
                                if let Some(t) = title {
                                    self.renaming_panel = Some(id);
                                    self.rename_buf = t;
                                }
                            }
                            SidebarResponse::ClosePanel(idx) => {
                                self.ws_mut().close_panel(idx);
                            }
                        }
                    }
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
                .max_by_key(|(_, panel)| panel.z_index())
                .map(|(idx, _)| idx)
        });

        if !self.command_palette.open {
            if let Some(idx) = hovered_terminal {
                let scroll_y = ctx.input(|input| input.smooth_scroll_delta.y);
                if scroll_y != 0.0 {
                    let font_size = self.font_size;
                    self.ws_mut().panels[idx].handle_scroll(ctx, scroll_y, font_size);
                    ctx.input_mut(|input| {
                        input.smooth_scroll_delta = egui::Vec2::ZERO;
                        input.raw_scroll_delta = egui::Vec2::ZERO;
                    });
                }
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

                if !self.command_palette.open {
                    crate::canvas::scene::handle_canvas_input(
                        ui,
                        &mut self.viewport,
                        canvas_rect,
                        hovered_terminal.is_some(),
                    );
                }

                if self.show_grid {
                    crate::canvas::grid::draw_grid(ui, &self.viewport, canvas_rect);
                }

                // Unfocus when clicking empty canvas
                if bg_resp.clicked_by(egui::PointerButton::Primary) {
                    for p in &mut self.ws_mut().panels {
                        p.set_focused(false);
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
                order.sort_by_key(|&i| self.ws().panels[i].z_index());

                let mut interactions = Vec::new();
                for &idx in &order {
                    if !self
                        .viewport
                        .is_visible(self.ws().panels[idx].rect(), canvas_rect)
                    {
                        continue;
                    }
                    let font_size = self.font_size;
                    let ix = self.ws_mut().panels[idx].show(ui, transform, canvas_rect, font_size);
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
                        // Track virtual (unsnapped) position so accumulated
                        // movement can escape snap zones naturally.
                        {
                            let panel = &mut self.ws_mut().panels[*idx];
                            if panel.drag_virtual_pos().is_none() {
                                let pos = panel.position();
                                panel.set_drag_virtual_pos(Some(pos));
                            }
                            let vp = panel.drag_virtual_pos().unwrap();
                            panel.set_drag_virtual_pos(Some(vp + ix.drag_delta));
                        }

                        let virtual_pos = self.ws().panels[*idx].drag_virtual_pos().unwrap();
                        let size = self.ws().panels[*idx].size();
                        let virtual_rect = egui::Rect::from_min_size(virtual_pos, size);
                        let others: Vec<egui::Rect> = self
                            .ws()
                            .panels
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| i != idx)
                            .map(|(_, p)| p.rect())
                            .collect();
                        let result =
                            crate::canvas::snap::snap_drag(virtual_rect, &others, egui::Vec2::ZERO);
                        self.ws_mut().panels[*idx].set_position(virtual_pos + result.delta);
                        snap_guides = result.guides;
                    }
                    if ix.resizing {
                        // Track virtual (unsnapped) rect so accumulated movement
                        // can escape snap zones — same pattern as drag_virtual_pos.
                        {
                            let panel = &mut self.ws_mut().panels[*idx];
                            if panel.resize_virtual_rect().is_none() {
                                panel.set_resize_virtual_rect(Some(panel.rect()));
                            }
                            let mut vr = panel.resize_virtual_rect().unwrap();
                            if ix.resize_left {
                                vr.min.x += ix.resize_delta.x;
                                vr.max.y += ix.resize_delta.y;
                            } else {
                                vr.max.x += ix.resize_delta.x;
                                vr.max.y += ix.resize_delta.y;
                            }
                            panel.set_resize_virtual_rect(Some(vr));
                        }

                        let vr = self.ws().panels[*idx].resize_virtual_rect().unwrap();
                        let others: Vec<egui::Rect> = self
                            .ws()
                            .panels
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| i != idx)
                            .map(|(_, p)| p.rect())
                            .collect();
                        // Compute snap from virtual rect (delta=ZERO since vr already
                        // contains the accumulated movement, same as drag).
                        let result = crate::canvas::snap::snap_resize(
                            vr,
                            &others,
                            egui::Vec2::ZERO,
                            ix.resize_left,
                        );
                        snap_guides = result.guides;

                        // Apply: virtual rect + snap adjustment → actual panel rect
                        let snapped = egui::Rect::from_min_max(
                            vr.min
                                + egui::Vec2::new(
                                    if ix.resize_left { result.delta.x } else { 0.0 },
                                    0.0,
                                ),
                            vr.max
                                + egui::Vec2::new(
                                    if ix.resize_left { 0.0 } else { result.delta.x },
                                    result.delta.y,
                                ),
                        );
                        let panel = &mut self.ws_mut().panels[*idx];
                        panel.set_position(snapped.min);
                        let new_size = snapped.size();
                        let clamped = egui::Vec2::new(new_size.x.max(400.0), new_size.y.max(280.0));
                        // Use set_position + direct size set via apply_resize trick
                        let current_size = panel.size();
                        panel.apply_resize(clamped - current_size);
                    }
                    if let Some(action) = &ix.action {
                        match action {
                            PanelAction::Close => to_close.push(*idx),
                            PanelAction::Rename => {
                                self.renaming_panel = Some(self.ws().panels[*idx].id());
                                self.rename_buf = self.ws().panels[*idx].title().to_string();
                            }
                        }
                    }
                }
                // Clear resize_virtual_rect for panels that aren't being resized
                let resizing_indices: Vec<usize> = interactions
                    .iter()
                    .filter(|(_, ix)| ix.resizing)
                    .map(|(idx, _)| *idx)
                    .collect();
                for (i, panel) in self.ws_mut().panels.iter_mut().enumerate() {
                    if !resizing_indices.contains(&i) && panel.resize_virtual_rect().is_some() {
                        panel.set_resize_virtual_rect(None);
                    }
                }

                to_close.sort_unstable();
                for idx in to_close.into_iter().rev() {
                    self.ws_mut().close_panel(idx);
                }

                // Unfocus all panels when clicking empty canvas
                if !interactions.iter().any(|(_, ix)| ix.clicked) {
                    let canvas_clicked = ctx.input(|i| {
                        i.pointer.button_clicked(egui::PointerButton::Primary)
                            && i.pointer
                                .latest_pos()
                                .is_some_and(|pos| canvas_rect.contains(pos))
                    });
                    if canvas_clicked {
                        for p in &mut self.ws_mut().panels {
                            p.set_focused(false);
                        }
                    }
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

        // --- Minimap overlay ---
        // Drawn in a small foreground area covering only the minimap rect,
        // so it doesn't block terminal interactions.
        if self.show_minimap {
            let mm_w = 220.0;
            let mm_h = 170.0;
            let mm_pos = Pos2::new(canvas_rect.max.x - mm_w, canvas_rect.max.y - mm_h);
            egui::Area::new(egui::Id::new("minimap_overlay"))
                .order(egui::Order::Debug)
                .fixed_pos(mm_pos)
                .interactable(true)
                .show(ctx, |ui| {
                    ui.set_clip_rect(canvas_rect);
                    ui.allocate_exact_size(Vec2::new(mm_w, mm_h), egui::Sense::hover());
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
                });
        }
    }
}
