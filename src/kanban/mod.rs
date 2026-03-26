// src/kanban/mod.rs — Kanban board canvas panel
//
// Reads task data from the bus every frame and renders a multi-column kanban view.
// Draggable, resizable, and zoomable — just like terminal panels.

#![allow(dead_code)]

use egui::{Color32, Pos2, Rect, Vec2};
use uuid::Uuid;

use crate::bus::task::{TaskInfo, TaskStatus};
use crate::bus::types::GroupInfo;

// ─── Colors ─────────────────────────────────────────────────────

const KANBAN_BG: Color32 = Color32::from_rgb(24, 24, 27);
const KANBAN_BORDER: Color32 = Color32::from_rgb(39, 39, 42);
const COLUMN_HEADER_BG: Color32 = Color32::from_rgb(39, 39, 42);
const CARD_BG: Color32 = Color32::from_rgb(39, 39, 42);
const CARD_HOVER: Color32 = Color32::from_rgb(52, 52, 59);
const CARD_TEXT: Color32 = Color32::from_rgb(228, 228, 231);
const CARD_TEXT_DIM: Color32 = Color32::from_rgb(113, 113, 122);

const TITLE_BAR_HEIGHT: f32 = 32.0;
const COLUMN_HEADER_HEIGHT: f32 = 28.0;
const COLUMN_MIN_WIDTH: f32 = 160.0;
const COLUMN_PADDING: f32 = 8.0;
const CARD_HEIGHT_MIN: f32 = 56.0;
const CARD_GAP: f32 = 6.0;
const CARD_ROUNDING: f32 = 6.0;
const CARD_BORDER_WIDTH: f32 = 3.0;
const CARD_PADDING: f32 = 8.0;
const BORDER_RADIUS: f32 = 8.0;

// ─── Column Definitions ─────────────────────────────────────────

const COLUMN_NAMES: &[&str] = &["BLOCKED", "PENDING", "IN PROGRESS", "DONE", "FAILED"];

fn column_color(col: usize) -> Color32 {
    match col {
        0 => Color32::from_rgb(234, 179, 8),   // yellow
        1 => Color32::from_rgb(163, 163, 163), // gray
        2 => Color32::from_rgb(59, 130, 246),  // blue
        3 => Color32::from_rgb(34, 197, 94),   // green
        4 => Color32::from_rgb(239, 68, 68),   // red
        _ => Color32::GRAY,
    }
}

// ─── Struct ─────────────────────────────────────────────────────

pub struct KanbanPanel {
    pub id: Uuid,
    pub position: Pos2,
    pub size: Vec2,
    pub z_index: u32,
    pub focused: bool,

    /// Group this kanban is bound to.
    pub group_id: Option<Uuid>,

    /// Cached task data (refreshed every frame from bus).
    cached_tasks: Vec<TaskInfo>,
    cached_group: Option<GroupInfo>,

    /// Scroll offset per column.
    column_scroll: [f32; 5],

    /// Currently expanded task card.
    expanded_task: Option<Uuid>,

    /// Swimlane mode toggle.
    swimlane_mode: bool,

    /// Drag state.
    pub drag_virtual_pos: Option<Pos2>,
    pub resize_virtual_rect: Option<Rect>,
}

impl KanbanPanel {
    pub fn new(position: Pos2, group_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            position,
            size: Vec2::new(800.0, 500.0),
            z_index: 0,
            focused: false,
            group_id: Some(group_id),
            cached_tasks: Vec::new(),
            cached_group: None,
            column_scroll: [0.0; 5],
            expanded_task: None,
            swimlane_mode: false,
            drag_virtual_pos: None,
            resize_virtual_rect: None,
        }
    }

    pub fn rect(&self) -> Rect {
        Rect::from_min_size(self.position, self.size)
    }

    /// Refresh cached data from the bus.
    pub fn sync_from_bus(&mut self, bus: &crate::bus::TerminalBus) {
        if let Some(gid) = self.group_id {
            self.cached_tasks = bus.task_list(gid, None, None);
            self.cached_group = bus.get_group(gid);
        }
    }

    /// Group tasks by column.
    fn tasks_by_column(&self) -> [Vec<&TaskInfo>; 5] {
        let mut columns: [Vec<&TaskInfo>; 5] = Default::default();
        for task in &self.cached_tasks {
            let col = TaskStatus::from_str(&task.status)
                .map(|s| s.column())
                .unwrap_or(1);
            if col < 5 {
                columns[col].push(task);
            }
        }
        // Sort each column by priority (desc)
        for col in &mut columns {
            col.sort_by(|a, b| b.priority.cmp(&a.priority));
        }
        columns
    }

    /// Render the kanban board. Returns any interaction.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        _transform: egui::emath::TSTransform,
        screen_clip: Rect,
    ) -> KanbanInteraction {
        let panel_rect = self.rect();

        // Frustum cull
        if !screen_clip.intersects(panel_rect) {
            return KanbanInteraction::None;
        }

        let painter = ui.painter();

        // Panel background + border + shadow
        painter.rect_filled(
            panel_rect.expand(2.0),
            BORDER_RADIUS + 1.0,
            Color32::from_rgba_premultiplied(0, 0, 0, 40),
        );
        painter.rect_filled(panel_rect, BORDER_RADIUS, KANBAN_BG);
        let border_color = if self.focused {
            Color32::from_rgb(59, 130, 246)
        } else {
            KANBAN_BORDER
        };
        painter.rect_stroke(
            panel_rect,
            BORDER_RADIUS,
            egui::Stroke::new(1.0, border_color),
        );

        // Title bar
        let title_rect = Rect::from_min_size(
            panel_rect.min,
            Vec2::new(panel_rect.width(), TITLE_BAR_HEIGHT),
        );
        painter.rect_filled(
            Rect::from_min_max(
                title_rect.min,
                Pos2::new(title_rect.max.x, title_rect.max.y),
            ),
            egui::Rounding {
                nw: BORDER_RADIUS,
                ne: BORDER_RADIUS,
                sw: 0.0,
                se: 0.0,
            },
            Color32::from_rgb(30, 30, 33),
        );

        let group_name = self
            .cached_group
            .as_ref()
            .map(|g| g.name.as_str())
            .unwrap_or("?");
        let title_text = format!("Kanban — {}", group_name);
        painter.text(
            Pos2::new(title_rect.min.x + 12.0, title_rect.center().y),
            egui::Align2::LEFT_CENTER,
            title_text,
            egui::FontId::proportional(12.0),
            CARD_TEXT,
        );

        // Title bar drag interaction
        let title_resp = ui.interact(
            title_rect,
            egui::Id::new(self.id).with("kanban_title"),
            egui::Sense::drag(),
        );
        let mut interaction = KanbanInteraction::None;
        if title_resp.dragged() {
            interaction = KanbanInteraction::DragStart;
        }
        if title_resp.clicked() {
            interaction = KanbanInteraction::Clicked;
        }

        // Content area
        let content_top = panel_rect.min.y + TITLE_BAR_HEIGHT;
        let content_rect = Rect::from_min_max(
            Pos2::new(panel_rect.min.x + COLUMN_PADDING, content_top + 4.0),
            Pos2::new(
                panel_rect.max.x - COLUMN_PADDING,
                panel_rect.max.y - COLUMN_PADDING,
            ),
        );

        let columns = self.tasks_by_column();

        // Determine visible columns (hide empty blocked/failed)
        let visible_cols: Vec<usize> = (0..5)
            .filter(|&c| !columns[c].is_empty() || c == 1 || c == 2 || c == 3)
            .collect();

        if visible_cols.is_empty() {
            painter.text(
                content_rect.center(),
                egui::Align2::CENTER_CENTER,
                "No tasks yet",
                egui::FontId::proportional(12.0),
                CARD_TEXT_DIM,
            );
            return interaction;
        }

        let col_width = (content_rect.width() / visible_cols.len() as f32).max(COLUMN_MIN_WIDTH);

        for (vi, &col_idx) in visible_cols.iter().enumerate() {
            let col_x = content_rect.min.x + vi as f32 * col_width;
            let col_rect = Rect::from_min_size(
                Pos2::new(col_x, content_rect.min.y),
                Vec2::new(col_width, content_rect.height()),
            );

            // Column header
            let header_rect = Rect::from_min_size(
                col_rect.min,
                Vec2::new(col_width - 4.0, COLUMN_HEADER_HEIGHT),
            );

            let col_color = column_color(col_idx);
            let count = columns[col_idx].len();
            let header_text = format!("{} ({})", COLUMN_NAMES[col_idx], count);
            painter.text(
                Pos2::new(header_rect.min.x + 4.0, header_rect.center().y),
                egui::Align2::LEFT_CENTER,
                header_text,
                egui::FontId::proportional(10.0),
                col_color,
            );

            // Separator line under header
            painter.line_segment(
                [
                    Pos2::new(header_rect.min.x, header_rect.max.y),
                    Pos2::new(header_rect.max.x, header_rect.max.y),
                ],
                egui::Stroke::new(0.5, Color32::from_rgb(50, 50, 55)),
            );

            // Cards
            let mut card_y = header_rect.max.y + CARD_GAP;
            for task in &columns[col_idx] {
                let card_height = CARD_HEIGHT_MIN;
                let card_rect = Rect::from_min_size(
                    Pos2::new(col_x + 2.0, card_y),
                    Vec2::new(col_width - 8.0, card_height),
                );

                if card_rect.min.y > content_rect.max.y {
                    break; // off-screen
                }

                // Card background
                let card_resp = ui.interact(
                    card_rect,
                    egui::Id::new(self.id).with(task.id),
                    egui::Sense::click(),
                );
                let bg = if card_resp.hovered() {
                    CARD_HOVER
                } else {
                    CARD_BG
                };
                painter.rect_filled(card_rect, CARD_ROUNDING, bg);

                // Left colored border
                let status_color = TaskStatus::from_str(&task.status)
                    .map(|s| {
                        let (r, g, b) = s.color_rgb();
                        Color32::from_rgb(r, g, b)
                    })
                    .unwrap_or(Color32::GRAY);

                painter.rect_filled(
                    Rect::from_min_size(card_rect.min, Vec2::new(CARD_BORDER_WIDTH, card_height)),
                    egui::Rounding {
                        nw: CARD_ROUNDING,
                        sw: CARD_ROUNDING,
                        ne: 0.0,
                        se: 0.0,
                    },
                    status_color,
                );

                // Card text
                let text_x = card_rect.min.x + CARD_BORDER_WIDTH + CARD_PADDING;
                let mut text_y = card_rect.min.y + 6.0;

                // Task ID (short)
                let short_id = &task.id.to_string()[..8];
                painter.text(
                    Pos2::new(text_x, text_y),
                    egui::Align2::LEFT_TOP,
                    short_id,
                    egui::FontId::monospace(9.0),
                    CARD_TEXT_DIM,
                );
                text_y += 14.0;

                // Subject (truncated)
                let max_chars = ((col_width - 24.0) / 6.5) as usize;
                let subject = if task.subject.len() > max_chars {
                    format!("{}...", &task.subject[..max_chars.saturating_sub(3)])
                } else {
                    task.subject.clone()
                };
                painter.text(
                    Pos2::new(text_x, text_y),
                    egui::Align2::LEFT_TOP,
                    subject,
                    egui::FontId::proportional(11.0),
                    CARD_TEXT,
                );
                text_y += 16.0;

                // Owner
                if let Some(ref title) = task.owner_title {
                    painter.text(
                        Pos2::new(text_x, text_y),
                        egui::Align2::LEFT_TOP,
                        title,
                        egui::FontId::proportional(9.0),
                        CARD_TEXT_DIM,
                    );
                }

                // Handle double-click to focus terminal
                if card_resp.double_clicked() {
                    if let Some(owner) = task.owner {
                        return KanbanInteraction::FocusTerminal(owner);
                    }
                }

                card_y += card_height + CARD_GAP;
            }
        }

        interaction
    }
}

#[derive(Debug)]
pub enum KanbanInteraction {
    None,
    Clicked,
    FocusTerminal(Uuid),
    ExpandTask(Uuid),
    CollapseTask,
    DragStart,
    ResizeStart,
}
