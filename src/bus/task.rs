// src/bus/task.rs — Task model for orchestration
//
// Tasks are units of work assigned to terminal agents. They live in the bus
// alongside terminals and groups, forming the primary coordination primitive.

use std::collections::HashMap;
use std::time::Instant;

use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────
// Task Status
// ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    /// Task is ready to be worked on.
    Pending,

    /// Task is actively being worked on by its owner.
    InProgress,

    /// Task is waiting for blocker tasks to complete.
    Blocked,

    /// Task completed successfully.
    Completed,

    /// Task failed. Can be retried by setting status back to Pending.
    Failed,
}

impl TaskStatus {
    pub fn label(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "in_progress" => Some(Self::InProgress),
            "completed" => Some(Self::Completed),
            "blocked" => Some(Self::Blocked),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }

    /// Kanban column index (for rendering order).
    pub fn column(&self) -> usize {
        match self {
            Self::Blocked => 0,
            Self::Pending => 1,
            Self::InProgress => 2,
            Self::Completed => 3,
            Self::Failed => 4,
        }
    }

    /// Display color as (r, g, b).
    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            Self::Pending => (163, 163, 163),   // neutral-400
            Self::InProgress => (59, 130, 246), // blue-500
            Self::Blocked => (234, 179, 8),     // yellow-500
            Self::Completed => (34, 197, 94),   // green-500
            Self::Failed => (239, 68, 68),      // red-500
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Task
// ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Task {
    /// Unique identifier.
    pub id: Uuid,

    /// Short description shown on kanban cards.
    pub subject: String,

    /// Detailed instructions (optional). Can be multi-line.
    pub description: String,

    /// Current status.
    pub status: TaskStatus,

    /// Terminal assigned to this task. None = unassigned.
    pub owner: Option<Uuid>,

    /// Group this task belongs to.
    pub group_id: Uuid,

    /// Terminal that created this task (usually the orchestrator).
    pub created_by: Uuid,

    /// When the task was created.
    pub created_at: Instant,

    /// When work started (status -> InProgress).
    pub started_at: Option<Instant>,

    /// When work completed (status -> Completed).
    pub completed_at: Option<Instant>,

    /// Task IDs that must be Completed before this task can start.
    pub blocked_by: Vec<Uuid>,

    /// Priority (0 = lowest, 255 = highest). Default 100.
    pub priority: u8,

    /// Free-form tags for filtering and display.
    pub tags: Vec<String>,

    /// Outcome summary, set when the task completes or fails.
    pub result: Option<String>,
}

impl Task {
    pub fn new(subject: impl Into<String>, group_id: Uuid, created_by: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            subject: subject.into(),
            description: String::new(),
            status: TaskStatus::Pending,
            owner: None,
            group_id,
            created_by,
            created_at: Instant::now(),
            started_at: None,
            completed_at: None,
            blocked_by: Vec::new(),
            priority: 100,
            tags: Vec::new(),
            result: None,
        }
    }

    /// Check if this task should be in Blocked state.
    pub fn should_be_blocked(&self, all_tasks: &HashMap<Uuid, Task>) -> bool {
        if self.blocked_by.is_empty() {
            return false;
        }
        self.blocked_by.iter().any(|blocker_id| {
            all_tasks
                .get(blocker_id)
                .map(|t| t.status != TaskStatus::Completed)
                .unwrap_or(false) // missing blocker = don't block
        })
    }

    /// Duration since work started (if in progress).
    pub fn elapsed(&self) -> Option<std::time::Duration> {
        self.started_at.map(|t| t.elapsed())
    }

    /// Short owner label for kanban card display.
    pub fn owner_short_id(&self) -> String {
        self.owner
            .map(|id| id.to_string()[..8].to_string())
            .unwrap_or_else(|| "unassigned".to_string())
    }
}

// ─────────────────────────────────────────────────────────────────
// Task Info — serializable for API responses
// ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub id: Uuid,
    pub subject: String,
    pub description: String,
    pub status: String,
    pub owner: Option<Uuid>,
    pub owner_title: Option<String>,
    pub group_id: Uuid,
    pub group_name: Option<String>,
    pub created_by: Uuid,
    pub blocked_by: Vec<Uuid>,
    pub blocking: Vec<Uuid>,
    pub priority: u8,
    pub tags: Vec<String>,
    pub result: Option<String>,
    pub elapsed_ms: Option<u64>,
}
