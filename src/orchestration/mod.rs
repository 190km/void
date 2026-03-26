// src/orchestration/mod.rs — Orchestration session management

pub mod prompt;
pub mod template;
pub mod worktree;

use uuid::Uuid;

/// Active orchestration session info for a workspace.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OrchestrationSession {
    /// The group ID for this orchestration.
    pub group_id: Uuid,

    /// Group name.
    pub group_name: String,

    /// Terminal ID of the leader (orchestrator).
    pub leader_id: Option<Uuid>,

    /// Whether the kanban board panel is visible.
    pub kanban_visible: bool,

    /// Whether the network view panel is visible.
    pub network_visible: bool,

    /// UUID of the kanban board canvas panel.
    pub kanban_panel_id: Option<Uuid>,

    /// UUID of the network view canvas panel.
    pub network_panel_id: Option<Uuid>,

    /// Template used to start this session (if any).
    pub template: Option<String>,
}

impl OrchestrationSession {
    pub fn new(group_id: Uuid, group_name: String, leader_id: Option<Uuid>) -> Self {
        Self {
            group_id,
            group_name,
            leader_id,
            kanban_visible: true,
            network_visible: true,
            kanban_panel_id: None,
            network_panel_id: None,
            template: None,
        }
    }
}
