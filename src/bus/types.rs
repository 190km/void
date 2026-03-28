// src/bus/types.rs

use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use alacritty_terminal::term::Term;
use uuid::Uuid;

use crate::terminal::pty::EventProxy;

// ---------------------------------------------------------------------------
// Terminal Handle — lightweight reference to a live terminal
// ---------------------------------------------------------------------------

/// A cloneable, thread-safe reference to a terminal's internal state.
///
/// Created by cloning the `Arc` fields from `PtyHandle`. Does not own
/// the terminal — just provides read/write access to it.
#[derive(Clone)]
pub struct TerminalHandle {
    /// Unique identifier for this terminal (same as TerminalPanel.id).
    pub id: Uuid,

    /// The alacritty terminal state machine. Lock to read the grid,
    /// cursor position, scrollback, terminal mode flags, etc.
    pub term: Arc<Mutex<Term<EventProxy>>>,

    /// The PTY writer. Lock to inject bytes into the terminal's stdin.
    /// Writing b"command\r" is equivalent to the user typing "command" + Enter.
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,

    /// The terminal's current title (set by OSC 0/2 sequences from the shell).
    pub title: Arc<Mutex<String>>,

    /// Whether the child process is still running.
    pub alive: Arc<AtomicBool>,

    /// Timestamp of the last byte written to the terminal (user input or injection).
    pub last_input_at: Arc<Mutex<Instant>>,

    /// Timestamp of the last byte read from the terminal (program output).
    pub last_output_at: Arc<Mutex<Instant>>,

    /// The workspace this terminal belongs to.
    pub workspace_id: Uuid,
}

// ---------------------------------------------------------------------------
// Terminal Status — observable state for group coordination
// ---------------------------------------------------------------------------

/// The observable status of a terminal within a group.
///
/// Updated automatically by the bus (via output monitoring) or manually
/// by the orchestrator via `set_status`.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TerminalStatus {
    /// Shell prompt is visible, no command running.
    /// Detected when `last_output_at` has not changed for `idle_threshold`.
    #[default]
    Idle,

    /// A command is executing. Output is flowing.
    Running {
        /// The command string, if known (set by inject_command).
        command: Option<String>,
        /// When the command started.
        started_at: Instant,
    },

    /// Waiting for input or for a dependency.
    Waiting {
        /// Human-readable reason, e.g. "waiting for term B to finish".
        reason: Option<String>,
    },

    /// Last command completed successfully.
    Done {
        /// When the command finished.
        finished_at: Instant,
    },

    /// Last command failed.
    Error {
        /// Error message or exit code.
        message: String,
        /// When the error occurred.
        occurred_at: Instant,
    },
}

impl TerminalStatus {
    /// Short label for display in the title bar.
    pub fn label(&self) -> &str {
        match self {
            Self::Idle => "idle",
            Self::Running { .. } => "running",
            Self::Waiting { .. } => "waiting",
            Self::Done { .. } => "done",
            Self::Error { .. } => "error",
        }
    }

    /// Whether this status indicates active work.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running { .. } | Self::Waiting { .. })
    }
}

// ---------------------------------------------------------------------------
// Terminal Role — position within a group
// ---------------------------------------------------------------------------

/// A terminal's role within its group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalRole {
    /// Not part of any group. Default state.
    Standalone,

    /// The orchestrator/parent of an orchestrated group.
    /// Can send commands to workers, read their output, manage lifecycle.
    Orchestrator,

    /// A worker/child in an orchestrated group.
    /// Receives commands from the orchestrator, reports status back.
    Worker,

    /// A peer in a peer-mode group.
    /// Can communicate with any other peer in the same group.
    Peer,
}

impl TerminalRole {
    /// Arrow indicator for the title bar.
    ///
    /// Orchestrator: ▲ (pointing up — in command)
    /// Worker:       ▼ (pointing down — receiving orders)
    /// Peer:         ◆ (diamond — equal standing)
    /// Standalone:   (empty)
    pub fn indicator(&self) -> &str {
        match self {
            Self::Standalone => "",
            Self::Orchestrator => "\u{25B2}", // ▲
            Self::Worker => "\u{25BC}",       // ▼
            Self::Peer => "\u{25C6}",         // ◆
        }
    }
}

// ---------------------------------------------------------------------------
// Group Mode
// ---------------------------------------------------------------------------

/// How terminals in a group relate to each other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupMode {
    /// One orchestrator controls N workers.
    /// The orchestrator's UUID is stored here.
    Orchestrated { orchestrator: Uuid },

    /// All members are peers with equal capabilities.
    Peer,
}

// ---------------------------------------------------------------------------
// Terminal Group
// ---------------------------------------------------------------------------

/// A named collection of terminals that can communicate.
///
/// Groups are created explicitly via `void-ctl group create` or the bus API.
/// Terminals join and leave groups dynamically.
#[derive(Debug, Clone)]
pub struct TerminalGroup {
    /// Unique group identifier.
    pub id: Uuid,

    /// Human-readable group name (e.g., "build", "research", "deploy").
    /// Used in the title bar indicator: `[build ▼ running]`.
    pub name: String,

    /// How members relate to each other.
    pub mode: GroupMode,

    /// All terminal UUIDs in this group, including the orchestrator.
    pub members: Vec<Uuid>,

    /// When the group was created.
    pub created_at: Instant,

    /// Per-group context namespace. Keys are prefixed with `{group_name}:`
    /// in the shared context store.
    pub context_prefix: String,
}

impl TerminalGroup {
    /// Create a new group in orchestrated mode.
    pub fn new_orchestrated(name: impl Into<String>, orchestrator: Uuid) -> Self {
        let name = name.into();
        let context_prefix = format!("{}:", name);
        Self {
            id: Uuid::new_v4(),
            name,
            mode: GroupMode::Orchestrated { orchestrator },
            members: vec![orchestrator],
            created_at: Instant::now(),
            context_prefix,
        }
    }

    /// Create a new group in peer mode.
    pub fn new_peer(name: impl Into<String>, initial_member: Uuid) -> Self {
        let name = name.into();
        let context_prefix = format!("{}:", name);
        Self {
            id: Uuid::new_v4(),
            name,
            mode: GroupMode::Peer,
            members: vec![initial_member],
            created_at: Instant::now(),
            context_prefix,
        }
    }

    /// Add a member to the group.
    pub fn add_member(&mut self, terminal_id: Uuid) {
        if !self.members.contains(&terminal_id) {
            self.members.push(terminal_id);
        }
    }

    /// Remove a member from the group. Returns true if the member was found.
    pub fn remove_member(&mut self, terminal_id: Uuid) -> bool {
        if let Some(pos) = self.members.iter().position(|&id| id == terminal_id) {
            self.members.remove(pos);
            true
        } else {
            false
        }
    }

    /// Whether this terminal is the orchestrator of this group.
    pub fn is_orchestrator(&self, terminal_id: Uuid) -> bool {
        match &self.mode {
            GroupMode::Orchestrated { orchestrator } => *orchestrator == terminal_id,
            GroupMode::Peer => false,
        }
    }

    /// Get the role of a terminal in this group.
    pub fn role_of(&self, terminal_id: Uuid) -> Option<TerminalRole> {
        if !self.members.contains(&terminal_id) {
            return None;
        }
        match &self.mode {
            GroupMode::Orchestrated { orchestrator } => {
                if *orchestrator == terminal_id {
                    Some(TerminalRole::Orchestrator)
                } else {
                    Some(TerminalRole::Worker)
                }
            }
            GroupMode::Peer => Some(TerminalRole::Peer),
        }
    }

    /// Whether the group is empty (should be cleaned up).
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Number of members.
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}

// ---------------------------------------------------------------------------
// Context Entry
// ---------------------------------------------------------------------------

/// A single entry in the shared context store.
#[derive(Debug, Clone)]
pub struct ContextEntry {
    /// The stored value.
    pub value: String,

    /// Which terminal wrote this entry.
    pub source: Uuid,

    /// When this entry was written or last updated.
    pub updated_at: SystemTime,

    /// Optional time-to-live. The entry is considered expired after this duration.
    /// Expired entries are cleaned up lazily on next access.
    pub ttl: Option<Duration>,
}

impl ContextEntry {
    /// Whether this entry has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl {
            if let Ok(elapsed) = self.updated_at.elapsed() {
                return elapsed > ttl;
            }
        }
        false
    }
}

// ---------------------------------------------------------------------------
// Bus Events
// ---------------------------------------------------------------------------

/// Events emitted by the terminal bus.
///
/// External subscribers (via APC layer) and internal consumers (via the
/// event bus) receive these events. Events are non-blocking — if a
/// subscriber's channel is full, the event is dropped for that subscriber.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum BusEvent {
    /// A terminal was registered with the bus (new terminal spawned).
    TerminalRegistered { terminal_id: Uuid, title: String },

    /// A terminal's child process exited.
    TerminalExited { terminal_id: Uuid },

    /// Bytes were injected into a terminal by another terminal or void-ctl.
    CommandInjected {
        source: Option<Uuid>,
        target: Uuid,
        command: String,
    },

    /// A terminal's output buffer changed (new data from PTY).
    /// This event is coalesced — at most one per terminal per 100ms.
    OutputChanged { terminal_id: Uuid },

    /// A terminal's status changed (idle -> running, running -> done, etc.).
    StatusChanged {
        terminal_id: Uuid,
        old_status: String,
        new_status: String,
    },

    /// A terminal's title changed (OSC 0/2 from the shell).
    TitleChanged {
        terminal_id: Uuid,
        old_title: String,
        new_title: String,
    },

    /// A new group was created.
    GroupCreated {
        group_id: Uuid,
        name: String,
        mode: String,
    },

    /// A terminal joined a group.
    GroupMemberJoined {
        group_id: Uuid,
        terminal_id: Uuid,
        role: String,
    },

    /// A terminal left a group.
    GroupMemberLeft { group_id: Uuid, terminal_id: Uuid },

    /// A group was dissolved (last member left or explicit dissolve).
    GroupDissolved { group_id: Uuid, name: String },

    /// A context entry was created or updated.
    ContextUpdated { key: String, source: Uuid },

    /// A context entry was deleted.
    ContextDeleted { key: String },

    /// A direct message was sent between terminals.
    MessageSent {
        from: Uuid,
        to: Uuid,
        payload: String,
    },

    /// A broadcast message was sent to all members of a group.
    BroadcastSent {
        from: Uuid,
        group_id: Uuid,
        payload: String,
    },

    // ── Task Events ─────────────────────────────────────────────
    /// A new task was created.
    TaskCreated {
        task_id: Uuid,
        subject: String,
        group_id: Uuid,
    },

    /// A task's status changed.
    TaskStatusChanged {
        task_id: Uuid,
        old_status: String,
        new_status: String,
    },

    /// A task was assigned to a terminal.
    TaskAssigned { task_id: Uuid, owner: Uuid },

    /// A task was unassigned.
    TaskUnassigned { task_id: Uuid, old_owner: Uuid },

    /// A blocked task was unblocked (all dependencies completed).
    TaskUnblocked { task_id: Uuid },

    /// A task was completed.
    TaskCompleted {
        task_id: Uuid,
        result: Option<String>,
    },

    /// A task failed.
    TaskFailed {
        task_id: Uuid,
        reason: Option<String>,
    },

    /// A task was deleted.
    TaskDeleted { task_id: Uuid },
}

impl BusEvent {
    /// Short type name for filtering.
    pub fn event_type(&self) -> &str {
        match self {
            Self::TerminalRegistered { .. } => "terminal.registered",
            Self::TerminalExited { .. } => "terminal.exited",
            Self::CommandInjected { .. } => "command.injected",
            Self::OutputChanged { .. } => "output.changed",
            Self::StatusChanged { .. } => "status.changed",
            Self::TitleChanged { .. } => "title.changed",
            Self::GroupCreated { .. } => "group.created",
            Self::GroupMemberJoined { .. } => "group.member.joined",
            Self::GroupMemberLeft { .. } => "group.member.left",
            Self::GroupDissolved { .. } => "group.dissolved",
            Self::ContextUpdated { .. } => "context.updated",
            Self::ContextDeleted { .. } => "context.deleted",
            Self::MessageSent { .. } => "message.sent",
            Self::BroadcastSent { .. } => "broadcast.sent",
            Self::TaskCreated { .. } => "task.created",
            Self::TaskStatusChanged { .. } => "task.status_changed",
            Self::TaskAssigned { .. } => "task.assigned",
            Self::TaskUnassigned { .. } => "task.unassigned",
            Self::TaskUnblocked { .. } => "task.unblocked",
            Self::TaskCompleted { .. } => "task.completed",
            Self::TaskFailed { .. } => "task.failed",
            Self::TaskDeleted { .. } => "task.deleted",
        }
    }
}

// ---------------------------------------------------------------------------
// Event Filter
// ---------------------------------------------------------------------------

/// Filter for subscribing to specific event types and/or terminals.
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    /// If non-empty, only events of these types are delivered.
    pub event_types: Vec<String>,

    /// If non-empty, only events involving these terminal IDs are delivered.
    pub terminal_ids: Vec<Uuid>,

    /// If set, only events from this group are delivered.
    pub group_id: Option<Uuid>,
}

impl EventFilter {
    /// Whether this filter matches an event.
    pub fn matches(&self, event: &BusEvent) -> bool {
        // Type filter
        if !self.event_types.is_empty() && !self.event_types.iter().any(|t| t == event.event_type())
        {
            return false;
        }

        // Terminal filter (check if any relevant UUID matches)
        if !self.terminal_ids.is_empty() {
            let involved = self.involved_terminals(event);
            if !involved.iter().any(|id| self.terminal_ids.contains(id)) {
                return false;
            }
        }

        // Group filter
        if let Some(gid) = &self.group_id {
            match event {
                BusEvent::GroupCreated { group_id, .. }
                | BusEvent::GroupMemberJoined { group_id, .. }
                | BusEvent::GroupMemberLeft { group_id, .. }
                | BusEvent::GroupDissolved { group_id, .. }
                | BusEvent::BroadcastSent { group_id, .. } => {
                    if group_id != gid {
                        return false;
                    }
                }
                _ => {}
            }
        }

        true
    }

    fn involved_terminals(&self, event: &BusEvent) -> Vec<Uuid> {
        match event {
            BusEvent::TerminalRegistered { terminal_id, .. } => vec![*terminal_id],
            BusEvent::TerminalExited { terminal_id } => vec![*terminal_id],
            BusEvent::CommandInjected { source, target, .. } => {
                let mut v = vec![*target];
                if let Some(s) = source {
                    v.push(*s);
                }
                v
            }
            BusEvent::OutputChanged { terminal_id } => vec![*terminal_id],
            BusEvent::StatusChanged { terminal_id, .. } => vec![*terminal_id],
            BusEvent::TitleChanged { terminal_id, .. } => vec![*terminal_id],
            BusEvent::GroupMemberJoined { terminal_id, .. } => vec![*terminal_id],
            BusEvent::GroupMemberLeft { terminal_id, .. } => vec![*terminal_id],
            BusEvent::ContextUpdated { source, .. } => vec![*source],
            BusEvent::MessageSent { from, to, .. } => vec![*from, *to],
            BusEvent::BroadcastSent { from, .. } => vec![*from],
            BusEvent::TaskAssigned { owner, .. } => vec![*owner],
            BusEvent::TaskUnassigned { old_owner, .. } => vec![*old_owner],
            _ => vec![],
        }
    }
}

// ---------------------------------------------------------------------------
// Terminal Info — serializable summary for API responses
// ---------------------------------------------------------------------------

/// Lightweight terminal info for API responses (no Arc references).
#[derive(Debug, Clone)]
pub struct TerminalInfo {
    pub id: Uuid,
    pub title: String,
    pub alive: bool,
    pub workspace_id: Uuid,
    pub group_id: Option<Uuid>,
    pub group_name: Option<String>,
    pub role: TerminalRole,
    pub status: TerminalStatus,
    pub last_output_elapsed_ms: u64,
    pub last_input_elapsed_ms: u64,
}

// ---------------------------------------------------------------------------
// Group Info — serializable summary for API responses
// ---------------------------------------------------------------------------

/// Lightweight group info for API responses.
#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub id: Uuid,
    pub name: String,
    pub mode: String,
    pub orchestrator_id: Option<Uuid>,
    pub member_count: usize,
    pub members: Vec<GroupMemberInfo>,
}

#[derive(Debug, Clone)]
pub struct GroupMemberInfo {
    pub terminal_id: Uuid,
    pub title: String,
    pub role: TerminalRole,
    pub status: TerminalStatus,
    pub alive: bool,
}
