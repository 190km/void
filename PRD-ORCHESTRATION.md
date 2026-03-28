# PRD-ORCHESTRATION.md — Void Swarm Intelligence System

> **From terminal emulator to AI swarm cockpit.**
> This document specifies everything needed to turn Void's existing Terminal Bus
> into a full ClawTeam-class orchestration platform — with task management,
> visual swarm monitoring, and native AI agent coordination.

**Status:** Draft v1.0
**Author:** 190km + Claude
**Date:** 2026-03-26
**Branch:** `feat/terminal-orchestration` (builds on PR #16)
**Depends on:** `orchestration-communication.md` (existing 4800-line spec)
**Estimated new code:** ~6,000–8,000 lines of Rust

---

## Table of Contents

1.  [Executive Summary](#1-executive-summary)
2.  [What Exists Today (PR #16)](#2-what-exists-today-pr-16)
3.  [What This PRD Adds](#3-what-this-prd-adds)
4.  [Architecture Overview](#4-architecture-overview)
5.  [Task System](#5-task-system)
    - 5.1 [Task Model](#51-task-model)
    - 5.2 [Task Lifecycle](#52-task-lifecycle)
    - 5.3 [Dependency Graph](#53-dependency-graph)
    - 5.4 [Auto-Unblock Protocol](#54-auto-unblock-protocol)
    - 5.5 [Task Assignment & Ownership](#55-task-assignment--ownership)
    - 5.6 [Data Structures (Rust)](#56-data-structures-rust)
    - 5.7 [Bus Extensions](#57-bus-extensions)
    - 5.8 [void-ctl Task Commands](#58-void-ctl-task-commands)
    - 5.9 [TCP Server Extensions](#59-tcp-server-extensions)
6.  [Orchestration Mode — Sidebar Toggle](#6-orchestration-mode--sidebar-toggle)
    - 6.1 [Mode States](#61-mode-states)
    - 6.2 [Sidebar UI Spec](#62-sidebar-ui-spec)
    - 6.3 [Activation Flow](#63-activation-flow)
    - 6.4 [Deactivation Flow](#64-deactivation-flow)
    - 6.5 [Persistence](#65-persistence)
7.  [Canvas Element: Kanban Board](#7-canvas-element-kanban-board)
    - 7.1 [Overview](#71-overview)
    - 7.2 [Visual Design](#72-visual-design)
    - 7.3 [Columns & Swimlanes](#73-columns--swimlanes)
    - 7.4 [Task Cards](#74-task-cards)
    - 7.5 [Interactions](#75-interactions)
    - 7.6 [Auto-Layout](#76-auto-layout)
    - 7.7 [Data Binding](#77-data-binding)
    - 7.8 [Implementation: KanbanPanel struct](#78-implementation-kanbanpanel-struct)
    - 7.9 [Rendering Pipeline](#79-rendering-pipeline)
    - 7.10 [Minimap Integration](#710-minimap-integration)
8.  [Canvas Element: Network Visualization](#8-canvas-element-network-visualization)
    - 8.1 [Overview](#81-overview)
    - 8.2 [Visual Design](#82-visual-design)
    - 8.3 [Node Types](#83-node-types)
    - 8.4 [Edge Types](#84-edge-types)
    - 8.5 [Layout Algorithm](#85-layout-algorithm)
    - 8.6 [Animation & Particles](#86-animation--particles)
    - 8.7 [Interactions](#87-interactions)
    - 8.8 [Real-Time Data Binding](#88-real-time-data-binding)
    - 8.9 [Implementation: NetworkPanel struct](#89-implementation-networkpanel-struct)
    - 8.10 [Rendering Pipeline](#810-rendering-pipeline)
    - 8.11 [Minimap Integration](#811-minimap-integration)
9.  [Canvas Edge Overlay: Inter-Panel Connections](#9-canvas-edge-overlay-inter-panel-connections)
    - 9.1 [Overview](#91-overview)
    - 9.2 [Edge Types](#92-edge-types)
    - 9.3 [Rendering](#93-rendering)
    - 9.4 [Particle Animation](#94-particle-animation)
    - 9.5 [Implementation](#95-implementation)
10. [Agent Coordination Protocol](#10-agent-coordination-protocol)
    - 10.1 [Auto-Prompt Injection](#101-auto-prompt-injection)
    - 10.2 [Claude Code Integration](#102-claude-code-integration)
    - 10.3 [Codex Integration](#103-codex-integration)
    - 10.4 [Generic Agent Interface](#104-generic-agent-interface)
    - 10.5 [Leader Election](#105-leader-election)
    - 10.6 [Coordination Prompt Template](#106-coordination-prompt-template)
    - 10.7 [Agent Discovery Protocol](#107-agent-discovery-protocol)
11. [Orchestration Templates (TOML)](#11-orchestration-templates-toml)
    - 11.1 [Template Format](#111-template-format)
    - 11.2 [Built-in Templates](#112-built-in-templates)
    - 11.3 [Template Execution Engine](#113-template-execution-engine)
    - 11.4 [Variable Substitution](#114-variable-substitution)
12. [Git Worktree Isolation](#12-git-worktree-isolation)
    - 12.1 [Why Worktrees](#121-why-worktrees)
    - 12.2 [Worktree Lifecycle](#122-worktree-lifecycle)
    - 12.3 [Merge Protocol](#123-merge-protocol)
    - 12.4 [Implementation](#124-implementation)
13. [CanvasPanel Enum Extension](#13-canvaspanel-enum-extension)
    - 13.1 [New Variants](#131-new-variants)
    - 13.2 [Trait Unification](#132-trait-unification)
    - 13.3 [Persistence](#133-persistence)
14. [Command Palette Extensions](#14-command-palette-extensions)
15. [Keyboard Shortcuts](#15-keyboard-shortcuts)
16. [Configuration (TOML)](#16-configuration-toml)
17. [Security Model](#17-security-model)
18. [Performance Budget](#18-performance-budget)
19. [Implementation Plan — Phased](#19-implementation-plan--phased)
20. [File-by-File Change Map](#20-file-by-file-change-map)
21. [Testing Strategy](#21-testing-strategy)
22. [Open Questions](#22-open-questions)

---

## 1. Executive Summary

Void already has the hardest part done: a Terminal Bus (PR #16) with inter-terminal
communication, groups, messaging, shared context, and a `void-ctl` CLI. This is
roughly 2,300 lines of working Rust.

What's missing is the **intelligence layer** — the part that turns raw
communication primitives into actual swarm behavior. ClawTeam (3.3k stars,
HKUDS/ClawTeam) achieves this in Python with tmux as the visual layer. We're
going to do it in pure Rust with Void's infinite canvas as the visual layer —
which is fundamentally superior because:

1. **You can see all agents at once** — zoom out. Tmux gives you a fixed grid.
2. **Spatial arrangement conveys meaning** — leader in the center, workers around it.
3. **Canvas elements beyond terminals** — a kanban board and a network graph live
   alongside the terminals, all draggable, all zoomable.
4. **GPU-accelerated at 60fps** — animated message particles between agents.

The deliverable is: **when a user toggles "Orchestration Mode" in the sidebar,
Void transforms from a terminal emulator into an AI swarm cockpit.**

---

## 2. What Exists Today (PR #16)

A quick inventory of what's already built and working:

### Terminal Bus (`src/bus/mod.rs` — 1,186 lines)
- Terminal registry with `TerminalHandle` (Arc references to PTY state)
- Command injection (`inject_bytes`, `send_command`, `send_interrupt`)
- Output reading (`read_screen`, `read_output` with scrollback)
- Idle detection with configurable threshold
- Status management (Idle → Running → Done → Error)
- Permission model (orchestrator → worker injection rules)
- Event system with filtered subscriptions (`mpsc::channel`)
- Pending spawn/close queues (polled by VoidApp each frame)

### Groups (`src/bus/types.rs` — 544 lines)
- Orchestrated mode (one leader, N workers)
- Peer mode (all equal)
- Group lifecycle (create, join, leave, dissolve)
- Role-based indicators (▲ orchestrator, ▼ worker, ◆ peer)
- Group-scoped context namespacing

### Shared Context
- Key-value store with TTL and expiration
- Group-scoped namespacing (`group_name:key`)
- Direct messaging via special `_msg:` keys

### TCP Bus Server (`src/bus/server.rs` — 106 lines)
- JSON-RPC over localhost TCP
- OS-assigned port via `VOID_BUS_PORT` env var
- Dispatches to same APC handler methods

### void-ctl CLI (`src/bin/void-ctl.rs` — 506 lines)
- `list`, `send`, `read`, `wait-idle`, `status`
- `group create/join/leave`
- `context set/get/list`
- `message send/list`
- `spawn`, `close`

### Integration
- `VoidApp` owns `Arc<Mutex<TerminalBus>>`
- Terminals register on spawn, deregister on close
- `VOID_TERMINAL_ID` and `VOID_BUS_PORT` env vars set on shells
- Workspace-scoped terminal listing

### What's NOT There
- ❌ Task system (no kanban, no dependencies, no assignment)
- ❌ Orchestration mode toggle in sidebar
- ❌ Canvas kanban board element
- ❌ Canvas network visualization element
- ❌ Inter-panel connection lines / edges on canvas
- ❌ Auto-prompt injection for AI agents
- ❌ Git worktree isolation per agent
- ❌ Orchestration templates (TOML)
- ❌ Agent discovery protocol

---

## 3. What This PRD Adds

```
┌──────────────────────────────────────────────────────────────────┐
│                         VOID CANVAS                              │
│                                                                  │
│   ┌─────────────┐     ╔═══════════════╗     ┌─────────────┐     │
│   │  Terminal A  │────▶║  KANBAN BOARD ║◀────│  Terminal C  │     │
│   │  (Leader)    │     ║               ║     │  (Worker 2)  │     │
│   │  Claude Code │     ║  TODO │ DOING ║     │  Codex       │     │
│   └──────┬───────┘     ║  ─────┼────── ║     └──────────────┘     │
│          │             ║  T1   │ T3    ║                          │
│          │             ║  T2   │       ║                          │
│          ▼             ║       │ DONE  ║                          │
│   ┌─────────────┐     ║       │────── ║     ╔══════════════╗     │
│   │  Terminal B  │     ║       │ T4    ║     ║  NETWORK     ║     │
│   │  (Worker 1)  │     ╚═══════════════╝     ║  VIEW        ║     │
│   │  Claude Code │──────────────────────────▶║              ║     │
│   └─────────────┘                            ║  [A]──▶[B]   ║     │
│                                              ║   │ ╲        ║     │
│   Animated message particles                 ║   ▼  ╲▶[C]   ║     │
│   flow along the edge lines ════▶            ╚══════════════╝     │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

Seven major additions:

| # | Feature | Lines (est.) | Priority |
|---|---------|-------------|----------|
| 1 | Task System (bus layer) | ~800 | P0 |
| 2 | Sidebar Orchestration Toggle | ~300 | P0 |
| 3 | Kanban Board (canvas element) | ~1,500 | P0 |
| 4 | Network Visualization (canvas element) | ~1,800 | P0 |
| 5 | Inter-Panel Edge Overlay | ~600 | P1 |
| 6 | Agent Coordination Protocol | ~400 | P1 |
| 7 | Orchestration Templates | ~500 | P2 |
| 8 | Git Worktree Isolation | ~400 | P2 |
| — | CanvasPanel refactor + glue | ~600 | P0 |
| — | **Total** | **~6,900** | |

---

## 4. Architecture Overview

```
                    ┌─────────────────────────────────────────────┐
                    │            VoidApp (main loop)               │
                    │                                             │
                    │  ┌───────────────────────────────────────┐  │
                    │  │          Orchestration Layer           │  │
                    │  │                                       │  │
                    │  │  ┌─────────┐  ┌──────────┐  ┌──────┐ │  │
                    │  │  │  Task   │  │  Agent   │  │ Git  │ │  │
                    │  │  │  Engine │  │  Coord.  │  │ Work │ │  │
                    │  │  │         │  │  Proto.  │  │ tree │ │  │
                    │  │  └────┬────┘  └────┬─────┘  └──┬───┘ │  │
                    │  │       │            │           │      │  │
                    │  │  ┌────▼────────────▼───────────▼───┐  │  │
                    │  │  │                                  │  │  │
                    │  │  │       Terminal Bus (existing)    │  │  │
                    │  │  │                                  │  │  │
                    │  │  │  terminals │ groups │ context    │  │  │
                    │  │  │  messages  │ events │ statuses   │  │  │
                    │  │  │                                  │  │  │
                    │  │  └──────────────────────────────────┘  │  │
                    │  └───────────────────────────────────────┘  │
                    │                                             │
                    │  ┌───────────────────────────────────────┐  │
                    │  │           Canvas Layer                 │  │
                    │  │                                       │  │
                    │  │  ┌──────────┐ ┌────────┐ ┌─────────┐ │  │
                    │  │  │ Terminal │ │ Kanban │ │ Network │ │  │
                    │  │  │ Panels   │ │ Board  │ │ View    │ │  │
                    │  │  │ (exist.) │ │ (NEW)  │ │ (NEW)   │ │  │
                    │  │  └──────────┘ └────────┘ └─────────┘ │  │
                    │  │                                       │  │
                    │  │  ┌──────────────────────────────────┐ │  │
                    │  │  │  Edge Overlay (NEW)               │ │  │
                    │  │  │  Animated lines between panels   │ │  │
                    │  │  └──────────────────────────────────┘ │  │
                    │  └───────────────────────────────────────┘  │
                    │                                             │
                    │  ┌───────────────────────────────────────┐  │
                    │  │  Sidebar                               │  │
                    │  │  ┌──────────────────────────────────┐  │  │
                    │  │  │  [x] Orchestration Mode (NEW)    │  │  │
                    │  │  │      ├── Team: "build"           │  │  │
                    │  │  │      ├── Leader: Terminal A       │  │  │
                    │  │  │      ├── Workers: 2/3 active      │  │  │
                    │  │  │      └── Tasks: 3/7 done          │  │  │
                    │  │  └──────────────────────────────────┘  │  │
                    │  └───────────────────────────────────────┘  │
                    └─────────────────────────────────────────────┘
```

The key principle: **every new feature is a layer on top of the existing bus.**
The bus doesn't change. It gains new method calls (for tasks), and new consumers
(the kanban board, the network view) subscribe to its events.

---

## 5. Task System

### 5.1 Task Model

A **task** is a unit of work assigned to a terminal agent. Tasks live in the bus
alongside terminals and groups. They are the primary coordination primitive —
what ClawTeam calls the "shared kanban."

```
Task {
    id:           Uuid        — unique identifier
    subject:      String      — short description ("Implement OAuth2 flow")
    description:  String      — detailed instructions (optional, can be long)
    status:       TaskStatus  — pending | in_progress | completed | blocked | failed
    owner:        Option<Uuid>— terminal assigned to this task (None = unassigned)
    group_id:     Uuid        — which group this task belongs to
    created_by:   Uuid        — terminal that created the task
    created_at:   Instant     — when the task was created
    started_at:   Option<Instant> — when work began
    completed_at: Option<Instant> — when work finished
    blocked_by:   Vec<Uuid>   — task IDs that must complete first
    priority:     u8          — 0 (lowest) to 255 (highest), default 100
    tags:         Vec<String> — free-form labels ("backend", "auth", "urgent")
    result:       Option<String> — outcome summary set on completion
}
```

### 5.2 Task Lifecycle

```
                          ┌─────────────────────────────────────┐
                          │                                     │
                          ▼                                     │
   ╔══════════╗    ╔══════════════╗    ╔═══════════════╗       │
   ║ PENDING  ║───▶║ IN_PROGRESS  ║───▶║   COMPLETED   ║       │
   ╚══════════╝    ╚══════════════╝    ╚═══════════════╝       │
        │                │                                     │
        │                │              ╔═══════════════╗       │
        │                └─────────────▶║    FAILED     ║       │
        │                               ╚═══════════════╝       │
        │                                      │               │
        ▼                                      └───────────────┘
   ╔══════════╗                                  (retry)
   ║ BLOCKED  ║
   ╚══════════╝
        │
        │ (all blockers completed)
        │
        ▼
   Auto-transitions to PENDING
```

State transition rules:

| From | To | Trigger | Who can do it |
|------|----|---------|--------------|
| `pending` | `in_progress` | `task update <id> --status in_progress` | Owner or orchestrator |
| `pending` | `blocked` | Task has `blocked_by` with incomplete tasks | Automatic on create |
| `blocked` | `pending` | All `blocked_by` tasks reach `completed` | Automatic (bus tick) |
| `in_progress` | `completed` | `task update <id> --status completed` | Owner or orchestrator |
| `in_progress` | `failed` | `task update <id> --status failed` | Owner or orchestrator |
| `failed` | `pending` | `task update <id> --status pending` (retry) | Orchestrator only |
| `completed` | `pending` | `task update <id> --status pending` (redo) | Orchestrator only |

### 5.3 Dependency Graph

Tasks can declare dependencies on other tasks. This forms a DAG (directed acyclic
graph) that the bus validates on creation.

```
Example: Full-stack todo app build

    T1: Design API schema
        │
        ├──────────┬───────────┐
        ▼          ▼           ▼
    T2: JWT auth   T3: DB     T4: React UI
        │          layer           │
        │          │               │
        └────┬─────┘               │
             │                     │
             ▼                     │
    T5: Integration ◀──────────────┘
        tests
```

In void-ctl:

```bash
# Orchestrator creates tasks with dependencies
void-ctl task create "Design API schema"
# Returns: task_id = aaa

void-ctl task create "Implement JWT auth" --blocked-by aaa --assign $WORKER_1
void-ctl task create "Build database layer" --blocked-by aaa --assign $WORKER_2
void-ctl task create "Build React frontend" --assign $WORKER_3
void-ctl task create "Integration tests" --blocked-by bbb,ccc,ddd
```

The bus enforces:
- **No cycles.** If T1 blocks T2 and T2 blocks T1, the second `blocked-by` is rejected.
- **No self-blocks.** A task cannot block itself.
- **Cascading auto-unblock.** When T1 completes, T2 and T3 auto-transition from
  `blocked` → `pending`. If T2 has an owner, it can auto-start.

### 5.4 Auto-Unblock Protocol

Every frame, `TerminalBus::tick_tasks()` checks:

```rust
for task in tasks where task.status == TaskStatus::Blocked {
    let all_blockers_done = task.blocked_by
        .iter()
        .all(|blocker_id| {
            tasks.get(blocker_id)
                .map(|b| b.status == TaskStatus::Completed)
                .unwrap_or(true) // missing blocker = unblock
        });

    if all_blockers_done {
        task.status = TaskStatus::Pending;
        emit(BusEvent::TaskUnblocked { task_id: task.id });

        // If task has an owner and auto_start is enabled:
        if let Some(owner) = task.owner {
            // Notify the agent via message
            send_message(leader, owner, format!(
                "TASK_READY: {} — {}",
                task.id, task.subject
            ));
        }
    }
}
```

### 5.5 Task Assignment & Ownership

Tasks can be:
- **Unassigned** — `owner: None`. Visible in the kanban "backlog" column.
- **Assigned** — `owner: Some(terminal_id)`. The terminal is responsible for this task.
- **Self-assigned** — A worker can pick up an unassigned task:
  `void-ctl task assign <task_id>` (uses `$VOID_TERMINAL_ID`).

Assignment rules in orchestrated groups:
- Orchestrator can assign any task to any worker.
- Workers can self-assign unassigned tasks.
- Workers cannot reassign tasks owned by other workers.
- Workers can update status of their own tasks.

In peer groups:
- Any peer can assign any task to any peer (including self).
- Any peer can update any task's status.

### 5.6 Data Structures (Rust)

```rust
// src/bus/task.rs — NEW FILE

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

    /// Display color (egui Color32).
    pub fn color_rgb(&self) -> (u8, u8, u8) {
        match self {
            Self::Pending => (163, 163, 163),    // neutral-400
            Self::InProgress => (59, 130, 246),  // blue-500
            Self::Blocked => (234, 179, 8),      // yellow-500
            Self::Completed => (34, 197, 94),    // green-500
            Self::Failed => (239, 68, 68),       // red-500
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
    /// e.g. "Implement OAuth2 flow"
    pub subject: String,

    /// Detailed instructions (optional). Can be multi-line.
    /// The agent reads this when starting work.
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
    /// While any blocker is not Completed, this task stays Blocked.
    pub blocked_by: Vec<Uuid>,

    /// Priority (0 = lowest, 255 = highest). Default 100.
    /// Higher priority tasks are shown first in the kanban column.
    pub priority: u8,

    /// Free-form tags for filtering and display.
    /// e.g. ["backend", "auth", "p0"]
    pub tags: Vec<String>,

    /// Outcome summary, set when the task completes or fails.
    /// e.g. "All 47 tests passing" or "TypeError in auth.rs:42"
    pub result: Option<String>,
}

impl Task {
    pub fn new(
        subject: impl Into<String>,
        group_id: Uuid,
        created_by: Uuid,
    ) -> Self {
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
            .map(|id| format!("{}", &id.to_string()[..8]))
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
    pub blocking: Vec<Uuid>,       // tasks that this task blocks (reverse lookup)
    pub priority: u8,
    pub tags: Vec<String>,
    pub result: Option<String>,
    pub elapsed_ms: Option<u64>,
}

// ─────────────────────────────────────────────────────────────────
// Task Events (extend BusEvent enum)
// ─────────────────────────────────────────────────────────────────

// These variants are added to the existing BusEvent enum:
//
//   TaskCreated { task_id: Uuid, subject: String, group_id: Uuid }
//   TaskStatusChanged { task_id: Uuid, old_status: String, new_status: String }
//   TaskAssigned { task_id: Uuid, owner: Uuid }
//   TaskUnassigned { task_id: Uuid, old_owner: Uuid }
//   TaskUnblocked { task_id: Uuid }
//   TaskCompleted { task_id: Uuid, result: Option<String> }
//   TaskFailed { task_id: Uuid, reason: Option<String> }
//   TaskDeleted { task_id: Uuid }
```

### 5.7 Bus Extensions

New fields in `TerminalBus`:

```rust
// Added to TerminalBus struct:
pub struct TerminalBus {
    // ... existing fields ...

    /// All tasks, keyed by UUID.
    tasks: HashMap<Uuid, Task>,

    /// Reverse dependency index: task_id → vec of tasks that depend on it.
    /// Updated on task create/delete. Used for fast "what does this unblock?" lookups.
    task_dependents: HashMap<Uuid, Vec<Uuid>>,
}
```

New methods on `TerminalBus`:

```rust
impl TerminalBus {
    // ── Task CRUD ───────────────────────────────────────────────

    /// Create a new task in a group.
    ///
    /// # Arguments
    /// * `subject` — Short description
    /// * `group_id` — Group this task belongs to
    /// * `created_by` — Terminal creating the task (must be in group)
    /// * `blocked_by` — Task IDs that must complete first
    /// * `owner` — Terminal to assign (optional)
    /// * `priority` — 0-255, default 100
    /// * `tags` — Free-form labels
    /// * `description` — Detailed instructions
    ///
    /// Returns the new task's UUID.
    ///
    /// # Errors
    /// - `GroupNotFound` if group doesn't exist
    /// - `TerminalNotFound` if created_by or owner isn't registered
    /// - `CycleDetected` if blocked_by would create a cycle
    /// - `PermissionDenied` if a worker tries to create a task in orchestrated mode
    pub fn task_create(
        &mut self,
        subject: &str,
        group_id: Uuid,
        created_by: Uuid,
        blocked_by: Vec<Uuid>,
        owner: Option<Uuid>,
        priority: u8,
        tags: Vec<String>,
        description: &str,
    ) -> Result<Uuid, BusError> { ... }

    /// Update a task's status.
    ///
    /// Validates the state transition (see lifecycle diagram).
    /// Auto-triggers unblock checks on dependent tasks.
    pub fn task_update_status(
        &mut self,
        task_id: Uuid,
        new_status: TaskStatus,
        source: Uuid,
        result: Option<String>,
    ) -> Result<(), BusError> { ... }

    /// Assign a task to a terminal.
    pub fn task_assign(
        &mut self,
        task_id: Uuid,
        owner: Uuid,
        source: Uuid,
    ) -> Result<(), BusError> { ... }

    /// Unassign a task.
    pub fn task_unassign(
        &mut self,
        task_id: Uuid,
        source: Uuid,
    ) -> Result<(), BusError> { ... }

    /// Delete a task.
    pub fn task_delete(
        &mut self,
        task_id: Uuid,
        source: Uuid,
    ) -> Result<(), BusError> { ... }

    /// List all tasks in a group, optionally filtered.
    pub fn task_list(
        &self,
        group_id: Uuid,
        status_filter: Option<TaskStatus>,
        owner_filter: Option<Uuid>,
    ) -> Vec<TaskInfo> { ... }

    /// Get a single task.
    pub fn task_get(&self, task_id: Uuid) -> Option<TaskInfo> { ... }

    /// Wait for a set of tasks to complete (polling, with timeout).
    /// Returns true if all completed, false on timeout.
    pub fn task_wait(
        tasks: &[Uuid],
        bus: &Arc<Mutex<TerminalBus>>,
        timeout: std::time::Duration,
    ) -> bool { ... }

    // ── Task Engine (called from tick) ──────────────────────────

    /// Process task state transitions.
    ///
    /// Called every frame from VoidApp::update().
    /// - Checks blocked tasks for unblock conditions
    /// - Auto-starts tasks with owners when unblocked (sends message)
    /// - Cleans up expired tasks (optional TTL)
    pub fn tick_tasks(&mut self) { ... }

    // ── DAG Validation ──────────────────────────────────────────

    /// Check if adding `blocked_by` edges to `task_id` would create a cycle.
    fn detect_cycle(&self, task_id: Uuid, blocked_by: &[Uuid]) -> bool { ... }

    /// Rebuild the reverse dependency index.
    fn rebuild_dependents_index(&mut self) { ... }
}
```

### 5.8 void-ctl Task Commands

```
void-ctl task create <subject> [options]
    --group <name>              Group name (required if terminal is in multiple groups)
    --blocked-by <id1,id2,...>  Comma-separated task IDs
    --assign <terminal_id>      Assign to a specific terminal
    --assign-self               Assign to calling terminal
    --priority <0-255>          Priority (default: 100)
    --tag <tag1,tag2,...>       Comma-separated tags
    --description <text>        Detailed instructions
    --json                      Output as JSON

    Example:
    $ void-ctl task create "Implement JWT auth" --blocked-by aaa --assign-self --tag backend,auth
    Created task bbb: Implement JWT auth [blocked → pending when aaa completes]

void-ctl task list [options]
    --group <name>              Filter by group
    --status <status>           Filter by status (pending|in_progress|completed|blocked|failed)
    --owner <id|me>             Filter by owner ("me" = $VOID_TERMINAL_ID)
    --json                      Output as JSON

    Example:
    $ void-ctl task list --owner me
    ID       STATUS       SUBJECT                    PRIORITY
    bbb      in_progress  Implement JWT auth         100
    eee      pending      Write unit tests           80

void-ctl task update <task_id> --status <status> [options]
    --result <text>             Set outcome text (for completed/failed)

    Example:
    $ void-ctl task update bbb --status completed --result "All 12 tests passing"
    Task bbb: completed ✓

void-ctl task assign <task_id> [options]
    --to <terminal_id>          Assign to specific terminal (orchestrator only)
    (no --to flag)              Self-assign to $VOID_TERMINAL_ID

void-ctl task unassign <task_id>

void-ctl task delete <task_id>

void-ctl task wait [options]
    --all                       Wait for all tasks in group
    --ids <id1,id2,...>         Wait for specific tasks
    --timeout <seconds>         Timeout (default: 300)

    Example:
    $ void-ctl task wait --all --timeout 600
    Waiting... [3/7 done] [2 in progress] [2 blocked]
    All tasks completed in 4m 23s.

void-ctl task get <task_id> --json
```

### 5.9 TCP Server Extensions

All task commands dispatch through the same `dispatch_bus_method` function in
`src/bus/apc.rs`. New methods:

| JSON-RPC Method | Params | Returns |
|-----------------|--------|---------|
| `task.create` | `{subject, group_id, blocked_by?, owner?, priority?, tags?, description?}` | `{task_id}` |
| `task.update_status` | `{task_id, status, result?}` | `{ok: true}` |
| `task.assign` | `{task_id, owner}` | `{ok: true}` |
| `task.unassign` | `{task_id}` | `{ok: true}` |
| `task.delete` | `{task_id}` | `{ok: true}` |
| `task.list` | `{group_id?, status?, owner?}` | `[TaskInfo, ...]` |
| `task.get` | `{task_id}` | `TaskInfo` |
| `task.wait` | `{task_ids?, all?, timeout?}` | `{completed: bool, elapsed_ms}` |

---

## 6. Orchestration Mode — Sidebar Toggle

### 6.1 Mode States

Orchestration mode is a workspace-level toggle. Each workspace independently
decides whether orchestration is active.

```rust
// Added to Workspace struct:
pub struct Workspace {
    // ... existing fields ...

    /// Whether orchestration mode is active in this workspace.
    pub orchestration_enabled: bool,

    /// Active orchestration session info (populated when enabled).
    pub orchestration: Option<OrchestrationSession>,
}

#[derive(Debug, Clone)]
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

    /// UUID of the kanban board canvas panel (for positioning).
    pub kanban_panel_id: Option<Uuid>,

    /// UUID of the network view canvas panel (for positioning).
    pub network_panel_id: Option<Uuid>,

    /// Template used to start this session (if any).
    pub template: Option<String>,
}
```

### 6.2 Sidebar UI Spec

When the "Terminals" tab is active in the sidebar, a new section appears at the
bottom:

```
┌─────────────────────────────────────┐
│  WORKSPACES  │  TERMINALS           │
├─────────────────────────────────────┤
│                                     │
│  ▸ Terminal A     ● idle            │
│  ▸ Terminal B     ● idle            │
│  ▸ Terminal C     ● idle            │
│                                     │
│  + New Terminal                     │
│                                     │
├─────────────────────────────────────┤  ◀─── new divider
│                                     │
│  ⚡ ORCHESTRATION                   │
│                                     │
│  ┌─────────────────────────────┐    │
│  │  [  ] Enable Orchestration  │    │  ◀─── toggle checkbox
│  └─────────────────────────────┘    │
│                                     │
│  (enable to create agent teams      │
│   with task tracking and swarm      │
│   visualization)                    │
│                                     │
└─────────────────────────────────────┘
```

When orchestration is **enabled**, the section expands:

```
├─────────────────────────────────────┤
│                                     │
│  ⚡ ORCHESTRATION                    │
│                                     │
│  ┌─────────────────────────────┐    │
│  │  [✓] Enable Orchestration   │    │
│  └─────────────────────────────┘    │
│                                     │
│  Team: build                        │
│  Mode: orchestrated                 │
│                                     │
│  ▲ Leader                           │
│  ┌─────────────────────────────┐    │
│  │ Terminal A        ● running │    │
│  │ claude code                  │    │
│  └─────────────────────────────┘    │
│                                     │
│  ▼ Workers                          │
│  ┌─────────────────────────────┐    │
│  │ Terminal B        ● running │    │
│  │ Task: Implement OAuth2      │    │
│  └─────────────────────────────┘    │
│  ┌─────────────────────────────┐    │
│  │ Terminal C        ● idle    │    │
│  │ Task: (none)                │    │
│  └─────────────────────────────┘    │
│                                     │
│  📋 Tasks: 3/7 done                │
│  ├── 2 in progress                 │
│  ├── 1 blocked                     │
│  └── 1 pending                     │
│                                     │
│  ┌─────────────────────────────┐    │
│  │  Show Kanban Board    [✓]   │    │
│  │  Show Network View    [✓]   │    │
│  └─────────────────────────────┘    │
│                                     │
│  ┌─────────────────────────────┐    │
│  │  + Spawn Worker              │    │
│  │  ⟳ From Template...          │    │
│  └─────────────────────────────┘    │
│                                     │
└─────────────────────────────────────┘
```

### 6.3 Activation Flow

When the user checks "Enable Orchestration":

1. **Create group.** A new orchestrated group is created in the bus. The user is
   prompted for a name (or a default is generated: "team-1").

2. **Designate leader.** The currently focused terminal becomes the orchestrator.
   If no terminal is focused, the first terminal in the workspace is used.

3. **Remaining terminals become workers.** All other terminals in the workspace
   auto-join the group as workers.

4. **Spawn canvas elements.** A KanbanPanel and a NetworkPanel are created on the
   canvas, positioned to the right of the existing terminal layout.

5. **Inject coordination prompts.** Each terminal receives a coordination prompt
   (via PTY injection) that teaches the agent how to use `void-ctl task` and
   `void-ctl message` commands. See §10.

6. **Start bus tick.** The task tick and status tick run every frame.

### 6.4 Deactivation Flow

When the user unchecks "Enable Orchestration":

1. **Dissolve group.** All terminals leave the group. The group is dissolved.
2. **Remove canvas elements.** KanbanPanel and NetworkPanel are removed from the
   workspace panels list.
3. **Stop task tick.** Tasks are deleted (or optionally preserved for history).
4. **Terminals keep running.** No terminals are closed. They just lose their
   orchestration roles and go back to standalone mode.

### 6.5 Persistence

The orchestration state is saved alongside workspace state:

```rust
// Added to WorkspaceState in persistence.rs:
pub struct WorkspaceState {
    // ... existing fields ...
    pub orchestration_enabled: bool,
    pub orchestration_group_name: Option<String>,
    pub orchestration_leader_id: Option<String>,
    pub orchestration_kanban_visible: bool,
    pub orchestration_network_visible: bool,
}
```

On restore, if orchestration was enabled:
- Recreate the group
- Re-register terminals with their roles
- Respawn kanban and network panels
- Tasks are NOT persisted across sessions (they live in memory only). Future
  enhancement: persist tasks to `~/.void/tasks/` as JSON.

---

## 7. Canvas Element: Kanban Board

### 7.1 Overview

The KanbanPanel is a new variant of `CanvasPanel` that renders a task board
directly on the infinite canvas. It reads task data from the bus every frame and
renders a multi-column kanban view.

It's draggable, resizable, and zoomable — just like terminal panels. It sits
alongside terminals on the same canvas, so you can zoom out and see terminals +
kanban + network view all at once.

### 7.2 Visual Design

```
╔═══════════════════════════════════════════════════════════════════════╗
║  📋 Kanban — build                                            ▼ ✕   ║
╠═══════════════════════════════════════════════════════════════════════╣
║                                                                     ║
║  BLOCKED (1)    PENDING (2)     IN PROGRESS (2)    DONE (2)         ║
║  ────────────   ────────────    ────────────────   ──────────       ║
║                                                                     ║
║  ┌──────────┐   ┌──────────┐   ┌──────────────┐   ┌──────────┐     ║
║  │ T5       │   │ T6       │   │ T2           │   │ T1       │     ║
║  │ Integr.  │   │ Unit     │   │ JWT auth     │   │ API      │     ║
║  │ tests    │   │ tests    │   │              │   │ schema   │     ║
║  │          │   │          │   │ ▼ Terminal B  │   │          │     ║
║  │ ⏳ 2 dep │   │ 80 prio  │   │ 🔵 3m 42s    │   │ ✅ 12m   │     ║
║  │ ⚠ T2,T3  │   │          │   └──────────────┘   └──────────┘     ║
║  └──────────┘   └──────────┘                                        ║
║                               ┌──────────────┐   ┌──────────┐     ║
║                 ┌──────────┐   │ T3           │   │ T4       │     ║
║                 │ T7       │   │ DB layer     │   │ React    │     ║
║                 │ Deploy   │   │              │   │ frontend │     ║
║                 │ script   │   │ ▼ Terminal C  │   │          │     ║
║                 │          │   │ 🔵 1m 15s    │   │ ✅ 8m    │     ║
║                 │ unassign │   └──────────────┘   └──────────┘     ║
║                 └──────────┘                                        ║
║                                                                     ║
╚═══════════════════════════════════════════════════════════════════════╝
```

Colors:
- Title bar: Same style as terminal panels (dark bg, colored accent)
- Column headers: `Color32::from_rgb(82, 82, 91)` (zinc-600)
- Blocked cards: Yellow left border `#EAB308`
- Pending cards: Gray left border `#A3A3A3`
- In Progress cards: Blue left border `#3B82F6`
- Completed cards: Green left border `#22C55E`
- Failed cards: Red left border `#EF4444`
- Card background: `Color32::from_rgb(39, 39, 42)` (zinc-800)
- Card hover: `Color32::from_rgb(52, 52, 59)` (zinc-700)

### 7.3 Columns & Swimlanes

Default columns (left to right):

| Column | Shows tasks with status | Header color |
|--------|------------------------|-------------|
| BLOCKED | `TaskStatus::Blocked` | Yellow |
| PENDING | `TaskStatus::Pending` | Gray |
| IN PROGRESS | `TaskStatus::InProgress` | Blue |
| DONE | `TaskStatus::Completed` | Green |
| FAILED | `TaskStatus::Failed` | Red |

FAILED column is only shown if there are failed tasks. BLOCKED column is only
shown if there are blocked tasks.

Within each column, tasks are sorted by:
1. Priority (highest first)
2. Created time (oldest first)

Optional swimlane mode (toggled via title bar button): group tasks by owner.
Each row is an agent, showing only that agent's tasks across the columns.

```
╔═══════════════════════════════════════════════════════════╗
║  📋 Kanban — build                           ☰ ▼ ✕      ║
╠═══════════════════════════════════════════════════════════╣
║              PENDING    IN PROGRESS    DONE               ║
║  ──────────  ────────   ────────────   ──────            ║
║  Terminal B  │ T6    │  │ T2 auth  │  │ T1  │            ║
║              └───────┘  └──────────┘  └─────┘            ║
║  ──────────  ────────   ────────────   ──────            ║
║  Terminal C  │ T7    │  │ T3 DB    │  │ T4  │            ║
║              └───────┘  └──────────┘  └─────┘            ║
╚═══════════════════════════════════════════════════════════╝
```

### 7.4 Task Cards

Each task card shows:

```
┌─────────────────────────┐
│  T2                      │  ← task ID (short hash)
│  Implement JWT auth      │  ← subject (max 2 lines, truncated)
│                          │
│  ▼ Terminal B            │  ← owner (with role indicator)
│  🔵 3m 42s               │  ← status dot + elapsed time
│  #backend #auth          │  ← tags (if any, max 3)
│  ⚠ blocked by: T1       │  ← dependency info (if blocked)
└─────────────────────────┘
```

Card dimensions:
- Width: fills column (column_width - 2*padding)
- Min height: 60px
- Max height: 120px (scrollable if content overflows)
- Padding between cards: 6px
- Card corner radius: 6px
- Left colored border: 3px wide

### 7.5 Interactions

| Action | Behavior |
|--------|----------|
| Click card | Expand card to show full description and result |
| Double-click card | Focus the terminal that owns this task (pan canvas to it) |
| Right-click card | Context menu: Assign, Change Status, Delete, Copy ID |
| Drag title bar | Move the kanban panel on the canvas |
| Drag corner/edge | Resize the kanban panel |
| Scroll inside | Scroll columns vertically (when they overflow) |
| Hover card | Show full subject in tooltip if truncated |

When a card is expanded:

```
┌───────────────────────────────────┐
│  T2: Implement JWT auth           │
│                                   │
│  Status: in_progress              │
│  Owner: Terminal B (▼ worker)     │
│  Priority: 100                    │
│  Tags: backend, auth              │
│  Blocked by: T1 (completed ✓)    │
│  Elapsed: 3m 42s                  │
│                                   │
│  Description:                     │
│  ────────────────────────────     │
│  Implement JWT authentication     │
│  with refresh tokens. Use the     │
│  jsonwebtoken crate. Endpoints:   │
│  POST /auth/login                 │
│  POST /auth/refresh               │
│  POST /auth/logout                │
│                                   │
│  Result: (not yet)                │
│                                   │
│  [Assign] [Complete] [Delete]     │
└───────────────────────────────────┘
```

### 7.6 Auto-Layout

When orchestration mode is first enabled, the kanban board auto-positions to the
right of the terminal cluster:

```rust
fn position_kanban(terminals: &[Rect]) -> Pos2 {
    let max_x = terminals.iter().map(|r| r.max.x).fold(f32::MIN, f32::max);
    let min_y = terminals.iter().map(|r| r.min.y).fold(f32::MAX, f32::min);
    Pos2::new(max_x + PANEL_GAP * 2.0, min_y)
}
```

Default size: `800 x 500`.

### 7.7 Data Binding

The KanbanPanel does NOT own task data. It reads from the bus every frame:

```rust
impl KanbanPanel {
    fn update(&mut self, bus: &TerminalBus) {
        if let Some(group_id) = self.group_id {
            self.cached_tasks = bus.task_list(group_id, None, None);
            self.cached_group = bus.get_group(group_id);
        }
    }
}
```

This is polled in `VoidApp::update()` alongside the existing bus tick. The kanban
is a **read-only view** of bus state — it never mutates the bus directly. User
interactions (assign, complete, delete) go through the bus API.

### 7.8 Implementation: KanbanPanel struct

```rust
// src/kanban/mod.rs — NEW FILE

use egui::{Color32, Pos2, Rect, Vec2};
use uuid::Uuid;

use crate::bus::task::{TaskInfo, TaskStatus};
use crate::bus::types::GroupInfo;

// ─── Colors ─────────────────────────────────────────────────────

const KANBAN_BG: Color32 = Color32::from_rgb(24, 24, 27);      // zinc-900
const KANBAN_BORDER: Color32 = Color32::from_rgb(39, 39, 42);   // zinc-800
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
const CARD_HEIGHT_MAX: f32 = 110.0;
const CARD_GAP: f32 = 6.0;
const CARD_ROUNDING: f32 = 6.0;
const CARD_BORDER_WIDTH: f32 = 3.0;
const CARD_PADDING: f32 = 8.0;

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

    /// Scroll offset per column (keyed by column index).
    column_scroll: [f32; 5],

    /// Currently expanded task card (shown as overlay).
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

    /// Refresh cached data from the bus. Called every frame.
    pub fn sync_from_bus(&mut self, bus: &crate::bus::TerminalBus) {
        if let Some(gid) = self.group_id {
            self.cached_tasks = bus.task_list(gid, None, None);
            self.cached_group = bus.get_group(gid);
        }
    }

    /// Render the kanban board.
    ///
    /// Returns any interaction that happened (task click, focus request, etc.)
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        transform: egui::emath::TSTransform,
        screen_clip: Rect,
    ) -> KanbanInteraction {
        // ... rendering logic (see §7.9)
        KanbanInteraction::None
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
        // Sort each column by priority (desc) then creation order
        for col in &mut columns {
            col.sort_by(|a, b| b.priority.cmp(&a.priority));
        }
        columns
    }
}

#[derive(Debug)]
pub enum KanbanInteraction {
    None,
    FocusTerminal(Uuid),       // double-click a card → pan to terminal
    ExpandTask(Uuid),          // click a card → show details
    CollapseTask,              // click away from expanded card
    AssignTask(Uuid, Uuid),    // assign task to terminal
    CompleteTask(Uuid),        // mark task complete
    DeleteTask(Uuid),          // delete task
    DragStart,                 // title bar drag
    ResizeStart,               // edge/corner drag
}
```

### 7.9 Rendering Pipeline

```
KanbanPanel::show()
│
├── 1. Transform position to screen coordinates
│       let screen_pos = transform * self.position;
│       let screen_size = self.size * transform.scaling;
│
├── 2. Frustum cull — skip if entirely outside screen_clip
│       if !screen_clip.intersects(screen_rect) { return; }
│
├── 3. Draw panel background + border + shadow
│       painter.rect_filled(screen_rect, BORDER_RADIUS, KANBAN_BG);
│       painter.rect_stroke(screen_rect, BORDER_RADIUS, Stroke::new(1.0, border_color));
│
├── 4. Draw title bar
│       "📋 Kanban — {group_name}"
│       Right side: swimlane toggle button, minimize, close
│
├── 5. Draw column headers
│       For each visible column:
│       │  Draw header bg + label + task count
│       │  "PENDING (3)" "IN PROGRESS (2)" etc.
│
├── 6. Draw task cards per column
│       For each column:
│       │  Apply column_scroll[col]
│       │  For each task in column:
│       │  │  Draw card background with left colored border
│       │  │  Draw task ID (short)
│       │  │  Draw subject (truncated to 2 lines)
│       │  │  Draw owner row (icon + terminal title)
│       │  │  Draw status dot + elapsed time
│       │  │  Draw tags (if any)
│       │  │  Draw blocker info (if blocked)
│       │  │  Handle click/double-click/hover
│
├── 7. Draw expanded card overlay (if any)
│       Full task details panel, positioned over the card
│
└── 8. Handle interactions
        Drag (title bar), Resize (edges), Scroll (columns)
```

### 7.10 Minimap Integration

The minimap (`src/canvas/minimap.rs`) renders small rectangles for each panel.
Kanban panels appear as a distinct color:

```rust
// In minimap rendering:
match panel {
    CanvasPanel::Terminal(_) => Color32::from_rgb(70, 70, 80),  // existing
    CanvasPanel::Kanban(_) => Color32::from_rgb(59, 130, 246),  // blue
    CanvasPanel::Network(_) => Color32::from_rgb(168, 85, 247), // purple
}
```

---

## 8. Canvas Element: Network Visualization

### 8.1 Overview

The NetworkPanel renders a live graph of agents (terminals) as nodes and their
communications (messages, command injections, task dependencies) as animated edges.
This is the "swarm brain" view — the equivalent of ClawTeam's teaser image showing
agents orchestrating.

### 8.2 Visual Design

```
╔══════════════════════════════════════════════════════════════════╗
║  🕸️ Network — build                                       ▼ ✕  ║
╠══════════════════════════════════════════════════════════════════╣
║                                                                  ║
║                          ┌───────────┐                          ║
║                          │  ▲ Leader  │                          ║
║                          │ Terminal A │                          ║
║                          │ ● running  │                          ║
║                          └─────┬─────┘                          ║
║                         ╱      │      ╲                         ║
║                     ╱╱╱        │        ╲╲╲                     ║
║                  ╱╱╱           │           ╲╲╲                  ║
║              ╱╱╱               │               ╲╲╲              ║
║          ┌───────────┐         │         ┌───────────┐          ║
║          │ ▼ Worker  │         │         │ ▼ Worker  │          ║
║          │Terminal B  │         │         │Terminal C  │          ║
║          │ ● running │ ◀ ○ ○ ○ ○ ○ ○ ▶  │ ● idle   │          ║
║          │           │   (messages)      │           │          ║
║          │ T2: auth  │                   │ T3: DB    │          ║
║          └───────────┘                   └───────────┘          ║
║                                                                  ║
║  ○ = message particle   ─── = command flow                      ║
║  ● = status indicator   ═══ = task dependency                   ║
║                                                                  ║
║  Legend: [messages: 12] [commands: 8] [tasks: 7]                ║
║                                                                  ║
╚══════════════════════════════════════════════════════════════════╝
```

### 8.3 Node Types

Each node represents a terminal in the group:

```rust
pub struct NetworkNode {
    /// Terminal ID this node represents.
    pub terminal_id: Uuid,

    /// Position within the network panel (local coordinates).
    pub pos: Pos2,

    /// Visual radius (scales with number of tasks/activity).
    pub radius: f32,

    /// Current role indicator.
    pub role: TerminalRole,

    /// Node color (matches terminal's accent color).
    pub color: Color32,

    /// Current status label.
    pub status: String,

    /// Active task subject (if any).
    pub active_task: Option<String>,

    /// Terminal title.
    pub title: String,

    /// Activity pulse (0.0 - 1.0, decays over time).
    /// Increases when messages are sent/received.
    pub activity: f32,
}
```

Node rendering:

```
Orchestrator node (larger):
    ┌─────────────────────┐
    │  ▲ Terminal A        │
    │  "Leader"            │
    │  ● running           │
    │                      │
    │  Tasks: 0 own, 7 mgd │
    └─────────────────────┘
    Outer glow: subtle colored ring when active

Worker node:
    ┌─────────────────┐
    │  ▼ Terminal B    │
    │  ● running       │
    │  T2: JWT auth    │
    └─────────────────┘
    Left border: matches task status color

Dead/disconnected node:
    ┌ ─ ─ ─ ─ ─ ─ ─ ─┐
    │  ▼ Terminal D    │
    │  ✕ exited        │
    └ ─ ─ ─ ─ ─ ─ ─ ─┘
    Dashed border, dimmed
```

Node sizes:
- Orchestrator: `radius = 45.0`
- Worker (active): `radius = 35.0`
- Worker (idle): `radius = 30.0`
- Worker (dead): `radius = 25.0`

### 8.4 Edge Types

```rust
#[derive(Debug, Clone)]
pub struct NetworkEdge {
    /// Source node (terminal ID).
    pub from: Uuid,

    /// Destination node (terminal ID).
    pub to: Uuid,

    /// Type of connection.
    pub edge_type: EdgeType,

    /// How many events have flowed along this edge.
    pub event_count: u32,

    /// Particles currently in flight along this edge.
    pub particles: Vec<EdgeParticle>,

    /// Edge thickness (scales with event_count).
    pub thickness: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    /// Leader → Worker command injection.
    /// Rendered as: solid arrow, blue.
    Command,

    /// Direct message between terminals.
    /// Rendered as: dashed line, white/gray.
    Message,

    /// Task dependency (task in A blocked by task in B).
    /// Rendered as: dotted line, yellow.
    Dependency,

    /// Broadcast from leader to all workers.
    /// Rendered as: thick solid arrow, purple.
    Broadcast,
}

impl EdgeType {
    pub fn color(&self) -> Color32 {
        match self {
            Self::Command => Color32::from_rgb(59, 130, 246),    // blue-500
            Self::Message => Color32::from_rgb(163, 163, 163),   // neutral-400
            Self::Dependency => Color32::from_rgb(234, 179, 8),  // yellow-500
            Self::Broadcast => Color32::from_rgb(168, 85, 247),  // purple-500
        }
    }

    pub fn dash_pattern(&self) -> Option<(f32, f32)> {
        match self {
            Self::Command => None,              // solid
            Self::Message => Some((6.0, 4.0)),  // dashed
            Self::Dependency => Some((3.0, 3.0)), // dotted
            Self::Broadcast => None,             // solid (thick)
        }
    }

    pub fn base_thickness(&self) -> f32 {
        match self {
            Self::Command => 2.0,
            Self::Message => 1.5,
            Self::Dependency => 1.0,
            Self::Broadcast => 3.0,
        }
    }
}
```

### 8.5 Layout Algorithm

Nodes are positioned using a simple force-directed layout within the panel:

```rust
/// Force-directed layout for network nodes.
///
/// Runs a fixed number of iterations per frame to converge smoothly.
/// Uses three forces:
///   1. Repulsion: all nodes repel each other (inverse square)
///   2. Attraction: connected nodes attract (spring)
///   3. Center gravity: all nodes pulled toward panel center
///
/// The orchestrator node is pinned to the center.
pub fn layout_step(nodes: &mut [NetworkNode], edges: &[NetworkEdge], center: Pos2) {
    const REPULSION: f32 = 8000.0;
    const ATTRACTION: f32 = 0.01;
    const CENTER_GRAVITY: f32 = 0.005;
    const DAMPING: f32 = 0.85;
    const MAX_VELOCITY: f32 = 5.0;
    const ITERATIONS_PER_FRAME: usize = 3;

    for _ in 0..ITERATIONS_PER_FRAME {
        let mut forces: Vec<Vec2> = vec![Vec2::ZERO; nodes.len()];

        // Repulsion (all pairs)
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                let delta = nodes[i].pos - nodes[j].pos;
                let dist_sq = delta.length_sq().max(1.0);
                let force = delta.normalized() * (REPULSION / dist_sq);
                forces[i] += force;
                forces[j] -= force;
            }
        }

        // Attraction (connected pairs)
        for edge in edges {
            let i = nodes.iter().position(|n| n.terminal_id == edge.from);
            let j = nodes.iter().position(|n| n.terminal_id == edge.to);
            if let (Some(i), Some(j)) = (i, j) {
                let delta = nodes[j].pos - nodes[i].pos;
                let force = delta * ATTRACTION;
                forces[i] += force;
                forces[j] -= force;
            }
        }

        // Center gravity
        for (i, node) in nodes.iter().enumerate() {
            let to_center = center - node.pos;
            forces[i] += to_center * CENTER_GRAVITY;
        }

        // Apply forces (skip pinned orchestrator)
        for (i, node) in nodes.iter_mut().enumerate() {
            if node.role == TerminalRole::Orchestrator {
                node.pos = center; // pinned
                continue;
            }
            let velocity = forces[i].clamp_length_max(MAX_VELOCITY) * DAMPING;
            node.pos += velocity;
        }
    }
}
```

### 8.6 Animation & Particles

When a message or command is sent between terminals, an animated particle travels
along the edge:

```rust
#[derive(Debug, Clone)]
pub struct EdgeParticle {
    /// Progress along the edge (0.0 = source, 1.0 = destination).
    pub t: f32,

    /// Speed (units per second). Default: 0.8.
    pub speed: f32,

    /// Size (radius). Default: 3.0.
    pub size: f32,

    /// Color (inherits from edge type).
    pub color: Color32,

    /// Trail length (number of past positions to draw).
    pub trail_length: usize,
}
```

Particle behavior:
- Spawned when a `BusEvent::CommandInjected` or `BusEvent::MessageSent` event fires.
- Travels from source to destination over ~1.5 seconds.
- Has a fading trail (4 ghost positions behind it).
- When it reaches `t >= 1.0`, the destination node pulses briefly.
- Multiple particles can be in flight on the same edge simultaneously.

Frame update:

```rust
fn tick_particles(&mut self, dt: f32) {
    for edge in &mut self.edges {
        // Advance existing particles
        edge.particles.retain_mut(|p| {
            p.t += p.speed * dt;
            p.t < 1.0  // remove when arrived
        });
    }
}
```

The destination node's `activity` field pulses when a particle arrives:

```rust
if particle.t >= 1.0 {
    if let Some(node) = nodes.iter_mut().find(|n| n.terminal_id == edge.to) {
        node.activity = 1.0; // will decay over time
    }
}
```

Activity decay: `node.activity *= 0.95` per frame (60fps → ~50 frames to reach 0.05).

### 8.7 Interactions

| Action | Behavior |
|--------|----------|
| Click node | Focus that terminal panel on the canvas (pan to it) |
| Hover node | Show tooltip with terminal details + active task |
| Hover edge | Show tooltip with event count and last message preview |
| Drag title bar | Move the network panel on the canvas |
| Drag corner/edge | Resize the network panel |
| Click legend item | Toggle visibility of that edge type |
| Scroll wheel | Zoom the internal graph layout |

### 8.8 Real-Time Data Binding

The NetworkPanel subscribes to bus events via `bus.subscribe()`:

```rust
impl NetworkPanel {
    pub fn new(position: Pos2, group_id: Uuid, bus: &mut TerminalBus) -> Self {
        let filter = EventFilter {
            group_id: Some(group_id),
            ..Default::default()
        };
        let (sub_id, event_rx) = bus.subscribe(filter);

        Self {
            // ...
            subscription_id: sub_id,
            event_rx,
            // ...
        }
    }

    /// Process pending events. Called every frame.
    pub fn process_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                BusEvent::CommandInjected { source, target, .. } => {
                    self.spawn_particle(source, Some(target), EdgeType::Command);
                }
                BusEvent::MessageSent { from, to, .. } => {
                    self.spawn_particle(Some(from), Some(to), EdgeType::Message);
                }
                BusEvent::BroadcastSent { from, group_id, .. } => {
                    // Spawn particles to all workers
                    for node in &self.nodes {
                        if node.terminal_id != from {
                            self.spawn_particle(Some(from), Some(node.terminal_id), EdgeType::Broadcast);
                        }
                    }
                }
                BusEvent::StatusChanged { terminal_id, new_status, .. } => {
                    if let Some(node) = self.nodes.iter_mut().find(|n| n.terminal_id == terminal_id) {
                        node.status = new_status;
                    }
                }
                BusEvent::GroupMemberJoined { terminal_id, .. } => {
                    self.add_node(terminal_id);
                }
                BusEvent::GroupMemberLeft { terminal_id, .. } => {
                    self.remove_node(terminal_id);
                }
                BusEvent::TaskCreated { .. } | BusEvent::TaskStatusChanged { .. } => {
                    self.update_task_edges();
                }
                _ => {}
            }
        }
    }
}
```

### 8.9 Implementation: NetworkPanel struct

```rust
// src/network/mod.rs — NEW FILE

use egui::{Color32, Pos2, Rect, Vec2};
use std::sync::mpsc;
use uuid::Uuid;

use crate::bus::types::*;

const NETWORK_BG: Color32 = Color32::from_rgb(17, 17, 21);
const NETWORK_BORDER: Color32 = Color32::from_rgb(39, 39, 42);
const GRID_COLOR: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 8);
const TITLE_BAR_HEIGHT: f32 = 32.0;

pub struct NetworkPanel {
    pub id: Uuid,
    pub position: Pos2,
    pub size: Vec2,
    pub z_index: u32,
    pub focused: bool,

    /// Group this view is bound to.
    pub group_id: Uuid,

    /// Nodes (one per terminal in group).
    nodes: Vec<NetworkNode>,

    /// Edges (connections between nodes).
    edges: Vec<NetworkEdge>,

    /// Event subscription for real-time updates.
    subscription_id: Uuid,
    event_rx: mpsc::Receiver<BusEvent>,

    /// Edge type visibility toggles.
    show_commands: bool,
    show_messages: bool,
    show_dependencies: bool,
    show_broadcasts: bool,

    /// Internal zoom level (for the graph, not the canvas zoom).
    internal_zoom: f32,

    /// Drag state.
    pub drag_virtual_pos: Option<Pos2>,
    pub resize_virtual_rect: Option<Rect>,

    /// Animation time accumulator.
    anim_time: f32,

    /// Stats counters.
    total_messages: u32,
    total_commands: u32,
    total_tasks: u32,
}
```

### 8.10 Rendering Pipeline

```
NetworkPanel::show()
│
├── 1. Transform + frustum cull (same as KanbanPanel)
│
├── 2. Draw panel background
│       Dark background with subtle dot grid
│
├── 3. Draw title bar
│       "🕸️ Network — {group_name}"
│       Right: edge type toggles, minimize, close
│
├── 4. Process events (non-blocking)
│       Drain event_rx, update nodes/edges/particles
│
├── 5. Layout step (force-directed)
│       Move nodes toward equilibrium
│
├── 6. Draw edges
│       For each edge:
│       │  Compute bezier curve between nodes
│       │  Draw line (solid/dashed/dotted based on type)
│       │  Draw arrowhead at destination
│       │  Draw particles along the curve
│
├── 7. Draw nodes
│       For each node:
│       │  Draw node background (rounded rect)
│       │  Draw role indicator (▲/▼/◆)
│       │  Draw terminal title
│       │  Draw status dot + label
│       │  Draw active task (if any)
│       │  Draw activity glow (pulsing ring)
│
├── 8. Draw legend
│       Bottom of panel: event type colors + counts
│
└── 9. Tick animations
        Advance particles, decay activity, update time
```

### 8.11 Minimap Integration

Network panels appear as purple rectangles in the minimap (see §7.10).

---

## 9. Canvas Edge Overlay: Inter-Panel Connections

### 9.1 Overview

When orchestration mode is active, **visible connection lines** are drawn between
terminal panels on the canvas itself (not inside the network panel). These are
the actual spatial connections — showing which terminal talks to which.

This layer renders ABOVE the canvas background but BELOW the panel contents. It
uses the same particle animation system as the network panel.

### 9.2 Edge Types

Same as NetworkEdge types (Command, Message, Dependency, Broadcast), but rendered
between the actual terminal panel rectangles on the canvas.

### 9.3 Rendering

```
                         Canvas Space
    ┌─────────────┐                    ┌─────────────┐
    │  Terminal A  │                    │  Terminal B  │
    │  (Leader)    ├────── ○ ○ ○ ──────▶  (Worker)    │
    │              │    command flow    │              │
    └──────────────┘                    └──────────────┘
          │                                    ▲
          │         ┌─────────────┐            │
          └──────── ○ ○ ○ ○ ──▶  │  Terminal C  │ ─── ○ ○ ┘
              broadcast          │  (Worker)    │  message
                                 └─────────────┘
```

Connection points: edges connect from the closest edge/corner of the source panel
to the closest edge/corner of the destination panel. They use cubic Bezier curves
with control points offset perpendicular to the direct line.

### 9.4 Particle Animation

Same as network panel particles, but in canvas coordinates. Particles travel
along the Bezier curves at a rate that's independent of zoom level (so they look
the same whether you're zoomed in or zoomed out).

### 9.5 Implementation

```rust
// src/canvas/edges.rs — NEW FILE

use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};
use uuid::Uuid;
use std::collections::HashMap;

use crate::bus::types::BusEvent;

/// An overlay that draws animated connection lines between panels on the canvas.
pub struct CanvasEdgeOverlay {
    /// Active edges between panels.
    edges: Vec<CanvasEdge>,

    /// Particles in flight.
    particles: Vec<CanvasParticle>,

    /// Whether the overlay is active.
    pub enabled: bool,
}

struct CanvasEdge {
    from: Uuid,  // terminal panel ID
    to: Uuid,    // terminal panel ID
    edge_type: EdgeType,
    event_count: u32,
    last_event_at: std::time::Instant,
}

struct CanvasParticle {
    from: Uuid,
    to: Uuid,
    t: f32,
    speed: f32,
    color: Color32,
    size: f32,
}

impl CanvasEdgeOverlay {
    pub fn new() -> Self {
        Self {
            edges: Vec::new(),
            particles: Vec::new(),
            enabled: false,
        }
    }

    /// Register a new communication event. Creates edge if needed, spawns particle.
    pub fn on_event(&mut self, event: &BusEvent) {
        // ... match event, create/update edges, spawn particles
    }

    /// Draw all edges and particles.
    ///
    /// Called from VoidApp::update() AFTER drawing the canvas background
    /// but BEFORE drawing panels.
    ///
    /// `panel_rects` maps terminal UUID → screen-space rect.
    pub fn draw(
        &self,
        painter: &Painter,
        panel_rects: &HashMap<Uuid, Rect>,
        transform: egui::emath::TSTransform,
        dt: f32,
    ) {
        if !self.enabled { return; }

        for edge in &self.edges {
            let from_rect = panel_rects.get(&edge.from);
            let to_rect = panel_rects.get(&edge.to);
            if let (Some(from), Some(to)) = (from_rect, to_rect) {
                self.draw_edge(painter, from, to, edge);
            }
        }

        for particle in &self.particles {
            let from_rect = panel_rects.get(&particle.from);
            let to_rect = panel_rects.get(&particle.to);
            if let (Some(from), Some(to)) = (from_rect, to_rect) {
                self.draw_particle(painter, from, to, particle);
            }
        }
    }

    /// Tick animations.
    pub fn tick(&mut self, dt: f32) {
        self.particles.retain_mut(|p| {
            p.t += p.speed * dt;
            p.t < 1.0
        });

        // Fade old edges (reduce opacity if no events for 30s)
        let now = std::time::Instant::now();
        self.edges.retain(|e| now.duration_since(e.last_event_at).as_secs() < 120);
    }

    fn draw_edge(&self, painter: &Painter, from: &Rect, to: &Rect, edge: &CanvasEdge) {
        let (start, end) = closest_edge_points(from, to);

        // Cubic bezier control points
        let mid = Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
        let perpendicular = Vec2::new(-(end.y - start.y), end.x - start.x).normalized();
        let offset = perpendicular * 30.0;

        let cp1 = Pos2::new(mid.x + offset.x, mid.y + offset.y);
        let cp2 = Pos2::new(mid.x - offset.x, mid.y - offset.y);

        // Draw bezier as line segments
        let color = edge.edge_type.color();
        let thickness = edge.edge_type.base_thickness();
        let points = bezier_points(start, cp1, cp2, end, 32);

        for i in 0..points.len() - 1 {
            painter.line_segment(
                [points[i], points[i + 1]],
                Stroke::new(thickness, color),
            );
        }

        // Arrowhead at end
        draw_arrowhead(painter, points[points.len() - 2], end, color, thickness);
    }

    fn draw_particle(&self, painter: &Painter, from: &Rect, to: &Rect, particle: &CanvasParticle) {
        let (start, end) = closest_edge_points(from, to);
        let pos = lerp_pos(start, end, particle.t);
        painter.circle_filled(pos, particle.size, particle.color);

        // Trail (3 ghost positions behind)
        for i in 1..=3 {
            let trail_t = (particle.t - 0.03 * i as f32).max(0.0);
            let trail_pos = lerp_pos(start, end, trail_t);
            let alpha = 255 - (i * 60) as u8;
            let trail_color = Color32::from_rgba_unmultiplied(
                particle.color.r(), particle.color.g(), particle.color.b(), alpha
            );
            painter.circle_filled(trail_pos, particle.size * 0.6, trail_color);
        }
    }
}

/// Find the closest points on the edges of two rectangles.
fn closest_edge_points(a: &Rect, b: &Rect) -> (Pos2, Pos2) {
    let a_center = a.center();
    let b_center = b.center();

    let start = rect_edge_intersection(a, a_center, b_center);
    let end = rect_edge_intersection(b, b_center, a_center);

    (start, end)
}

/// Find where a ray from `inside` toward `target` exits a rectangle.
fn rect_edge_intersection(rect: &Rect, inside: Pos2, target: Pos2) -> Pos2 {
    let dx = target.x - inside.x;
    let dy = target.y - inside.y;

    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        return inside;
    }

    let mut t_min = f32::MAX;

    // Check all 4 edges
    if dx != 0.0 {
        // Left edge
        let t = (rect.min.x - inside.x) / dx;
        let y = inside.y + t * dy;
        if t > 0.0 && t < t_min && y >= rect.min.y && y <= rect.max.y { t_min = t; }
        // Right edge
        let t = (rect.max.x - inside.x) / dx;
        let y = inside.y + t * dy;
        if t > 0.0 && t < t_min && y >= rect.min.y && y <= rect.max.y { t_min = t; }
    }
    if dy != 0.0 {
        // Top edge
        let t = (rect.min.y - inside.y) / dy;
        let x = inside.x + t * dx;
        if t > 0.0 && t < t_min && x >= rect.min.x && x <= rect.max.x { t_min = t; }
        // Bottom edge
        let t = (rect.max.y - inside.y) / dy;
        let x = inside.x + t * dx;
        if t > 0.0 && t < t_min && x >= rect.min.x && x <= rect.max.x { t_min = t; }
    }

    if t_min == f32::MAX {
        inside
    } else {
        Pos2::new(inside.x + t_min * dx, inside.y + t_min * dy)
    }
}
```

---

## 10. Agent Coordination Protocol

### 10.1 Auto-Prompt Injection

When orchestration mode is activated and a terminal is designated as leader or
worker, a **coordination prompt** is injected into the terminal's PTY. This is a
block of text that teaches the AI agent how to use the orchestration tools.

The injection happens by writing to the PTY writer — the same mechanism used by
`bus.send_command()`. It appears as if the user typed (or pasted) the text.

For AI agents specifically, the prompt is sent as a special comment that the
agent can parse:

```bash
# ─── VOID ORCHESTRATION PROTOCOL ────────────────────────────────
# You are running inside Void, an infinite canvas terminal emulator
# with built-in swarm intelligence. Your terminal ID is: $VOID_TERMINAL_ID
# Your role: LEADER | WORKER
# Your team: $TEAM_NAME
# Bus port: $VOID_BUS_PORT
#
# Available commands (use void-ctl):
#   void-ctl task create "subject" --assign $WORKER_ID
#   void-ctl task list --owner me
#   void-ctl task update $TASK_ID --status completed --result "summary"
#   void-ctl task wait --all --timeout 600
#   void-ctl message send $TERMINAL_ID "message text"
#   void-ctl message list
#   void-ctl list           (see all terminals)
#   void-ctl send $ID "cmd" (inject command into another terminal)
#   void-ctl read $ID       (read terminal output)
#   void-ctl context set key value
#   void-ctl context get key
# ─────────────────────────────────────────────────────────────────
```

### 10.2 Claude Code Integration

Claude Code detects `VOID_TERMINAL_ID` in the environment and enters
orchestration mode. The coordination prompt is written to a file that Claude
Code reads as part of its system context:

```bash
# Written by Void when orchestration is enabled:
mkdir -p /tmp/void-orchestration
cat > /tmp/void-orchestration/protocol.md << 'VOID_PROTO'
# Void Orchestration Protocol

You are the LEADER of team "build" in Void's swarm intelligence system.
You have access to void-ctl commands to coordinate worker agents.

## Your Workers
- Terminal B: available for backend tasks
- Terminal C: available for frontend tasks

## Workflow
1. Create tasks: `void-ctl task create "Build auth module" --assign $WORKER_B_ID`
2. Monitor progress: `void-ctl task list`
3. Read worker output: `void-ctl read $WORKER_ID --lines 50`
4. Send instructions: `void-ctl message send $WORKER_ID "Use JWT, not session cookies"`
5. Wait for completion: `void-ctl task wait --all`
6. Collect results: `void-ctl context get result_auth`

## Rules
- Always create tasks before assigning work
- Check task status before sending new commands
- Use void-ctl message for coordination, not void-ctl send (which injects raw commands)
- Set context values for shared state: `void-ctl context set api_schema '{"endpoints": [...]}'`
VOID_PROTO

export VOID_ORCHESTRATION_PROTOCOL="/tmp/void-orchestration/protocol.md"
```

### 10.3 Codex Integration

Codex uses a similar approach. The protocol file is set as `CODEX_INSTRUCTIONS`:

```bash
export CODEX_INSTRUCTIONS="/tmp/void-orchestration/protocol.md"
```

### 10.4 Generic Agent Interface

For any CLI agent that doesn't have a special integration, the coordination prompt
is simply echoed to the terminal as a comment block. The agent sees it in its
terminal history and can reference it.

Additionally, Void sets these environment variables on every spawned terminal:

```
VOID_TERMINAL_ID=<uuid>
VOID_BUS_PORT=<port>
VOID_TEAM_NAME=<team_name>          (when in orchestration mode)
VOID_ROLE=leader|worker|peer        (when in orchestration mode)
VOID_GROUP_ID=<group_uuid>          (when in orchestration mode)
VOID_ORCHESTRATION_PROTOCOL=<path>  (path to protocol.md)
```

### 10.5 Leader Election

When orchestration mode is activated:

1. **Explicit:** The user designates which terminal is the leader via the sidebar
   or command palette.
2. **Default:** The currently focused terminal becomes the leader.
3. **Template:** The template specifies which agent type is the leader.

Leader responsibilities (enforced by the bus permission model):
- Only the leader can create tasks.
- Only the leader can assign tasks to workers.
- Only the leader can broadcast commands to all workers.
- Workers can self-assign unassigned tasks.
- Workers can update status of their own tasks.
- Workers can send messages to the leader or other workers.

### 10.6 Coordination Prompt Template

The full prompt varies by role. Here's the leader prompt:

```markdown
# Void Orchestration — Leader Protocol

You are the **Leader** agent in a Void orchestration team.

## Environment
- Terminal ID: `{terminal_id}`
- Team: `{team_name}`
- Group ID: `{group_id}`
- Workers: {worker_count}
- Bus Port: `{bus_port}`

## Your Responsibilities
1. **Plan** — Break the goal into tasks
2. **Assign** — Create tasks and assign to workers
3. **Monitor** — Check progress, read worker output
4. **Coordinate** — Share context, resolve blockers
5. **Collect** — Gather results, verify quality

## Commands Reference

### Task Management
```bash
# Create a task (auto-assigns to best available worker)
void-ctl task create "Implement user authentication" \
  --assign {worker_1_id} \
  --priority 100 \
  --tag backend,auth

# Create dependent tasks
void-ctl task create "Write integration tests" \
  --blocked-by {task_1_id},{task_2_id} \
  --assign {worker_2_id}

# Check all task statuses
void-ctl task list --json

# Wait for all tasks to complete
void-ctl task wait --all --timeout 600
```

### Worker Communication
```bash
# Read a worker's terminal output (last 50 lines)
void-ctl read {worker_id} --lines 50

# Send a message to a worker
void-ctl message send {worker_id} "Use the jsonwebtoken crate, not jwt-simple"

# Share data via context
void-ctl context set api_schema '{"users": "/api/v1/users", "auth": "/api/v1/auth"}'

# Broadcast a command to all workers
void-ctl send {worker_id} "cargo test"
```

### Monitoring
```bash
# List all terminals and their status
void-ctl list

# Check if a terminal is idle
void-ctl wait-idle {worker_id}

# Get shared context
void-ctl context list
```

## Best Practices
- Create ALL tasks before assigning work (so dependencies are clear)
- Use `void-ctl context set` to share schemas, configs, and decisions
- Check worker output before assuming completion
- Use `--blocked-by` for task ordering instead of manual sequencing
- Set task results on completion: `void-ctl task update {id} --status completed --result "summary"`
```

Worker prompt is similar but focused on:
- Checking own tasks: `void-ctl task list --owner me`
- Updating task status: `void-ctl task update {id} --status in_progress`
- Reporting results: `void-ctl task update {id} --status completed --result "..."`
- Messaging leader: `void-ctl message send {leader_id} "Need clarification on X"`
- Reading shared context: `void-ctl context get api_schema`

### 10.7 Agent Discovery Protocol

An agent can detect it's in Void and discover the orchestration system:

```bash
# Check if we're in Void
if [ -n "$VOID_TERMINAL_ID" ]; then
    echo "Running in Void terminal: $VOID_TERMINAL_ID"

    # Check if orchestration is active
    if [ -n "$VOID_TEAM_NAME" ]; then
        echo "Team: $VOID_TEAM_NAME, Role: $VOID_ROLE"

        # Read the protocol file for detailed instructions
        if [ -f "$VOID_ORCHESTRATION_PROTOCOL" ]; then
            cat "$VOID_ORCHESTRATION_PROTOCOL"
        fi

        # List team members
        void-ctl list --json
    fi
fi
```

---

## 11. Orchestration Templates (TOML)

### 11.1 Template Format

Templates define pre-configured team setups. Stored in `~/.void/templates/` or
bundled with Void.

```toml
# ~/.void/templates/fullstack-build.toml

[team]
name = "fullstack-{timestamp}"
mode = "orchestrated"
description = "Full-stack application build team"

[leader]
title = "Architect"
command = "claude"  # CLI command to run in the terminal
prompt = """
You are the lead architect. Break down the following goal into tasks
and coordinate the workers to build it:

Goal: {goal}
"""

[[worker]]
name = "backend"
title = "Backend Developer"
command = "claude"
prompt = """
You are a backend developer. Wait for tasks from the leader.
Focus on API design, database schemas, and server logic.
Tech stack: Rust + Axum + PostgreSQL
"""

[[worker]]
name = "frontend"
title = "Frontend Developer"
command = "claude"
prompt = """
You are a frontend developer. Wait for tasks from the leader.
Focus on React components, state management, and UI/UX.
Tech stack: React + TypeScript + Tailwind
"""

[[worker]]
name = "tester"
title = "QA Engineer"
command = "claude"
prompt = """
You are a QA engineer. Wait for tasks from the leader.
Focus on writing tests, reviewing code quality, and integration testing.
"""

[layout]
# How to arrange terminals on the canvas
pattern = "star"  # star | grid | row | column
# star: leader in center, workers around it
# grid: leader top-left, workers fill grid
# row: all in a horizontal row
# column: all in a vertical column

[kanban]
visible = true
position = "right"  # right | bottom | auto

[network]
visible = true
position = "bottom-right"  # bottom-right | right | auto
```

### 11.2 Built-in Templates

Void ships with these templates:

| Template | Agents | Description |
|----------|--------|-------------|
| `duo` | 1 leader + 1 worker | Simple pair programming |
| `trio` | 1 leader + 2 workers | Small team build |
| `fullstack` | 1 leader + 3 workers | Frontend + Backend + QA |
| `research` | 1 leader + 4 workers | Parallel research exploration |
| `hedge-fund` | 1 PM + 5 analysts + 1 risk | Investment analysis (ClawTeam-inspired) |

### 11.3 Template Execution Engine

```rust
// src/orchestration/template.rs — NEW FILE

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct OrcTemplate {
    pub team: TeamConfig,
    pub leader: AgentConfig,
    #[serde(default)]
    pub worker: Vec<AgentConfig>,
    #[serde(default)]
    pub layout: LayoutConfig,
    #[serde(default)]
    pub kanban: PanelConfig,
    #[serde(default)]
    pub network: PanelConfig,
}

#[derive(Debug, Deserialize)]
pub struct TeamConfig {
    pub name: String,
    pub mode: String,          // "orchestrated" | "peer"
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    #[serde(default)]
    pub name: String,
    pub title: String,
    #[serde(default = "default_command")]
    pub command: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Default)]
pub struct LayoutConfig {
    #[serde(default = "default_pattern")]
    pub pattern: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct PanelConfig {
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_position")]
    pub position: String,
}

fn default_command() -> String { "claude".to_string() }
fn default_pattern() -> String { "star".to_string() }
fn default_true() -> bool { true }
fn default_position() -> String { "auto".to_string() }

impl OrcTemplate {
    /// Load a template from a TOML file.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read template: {}", e))?;
        toml::from_str(&content)
            .map_err(|e| format!("Failed to parse template: {}", e))
    }

    /// Load a built-in template by name.
    pub fn builtin(name: &str) -> Option<Self> {
        let toml_str = match name {
            "duo" => include_str!("../../templates/duo.toml"),
            "trio" => include_str!("../../templates/trio.toml"),
            "fullstack" => include_str!("../../templates/fullstack.toml"),
            "research" => include_str!("../../templates/research.toml"),
            "hedge-fund" => include_str!("../../templates/hedge-fund.toml"),
            _ => return None,
        };
        toml::from_str(toml_str).ok()
    }

    /// Apply variable substitution.
    pub fn substitute(&mut self, vars: &std::collections::HashMap<String, String>) {
        let sub = |s: &mut String| {
            for (key, val) in vars {
                *s = s.replace(&format!("{{{}}}", key), val);
            }
        };

        sub(&mut self.team.name);
        sub(&mut self.team.description);
        sub(&mut self.leader.prompt);
        for w in &mut self.worker {
            sub(&mut w.prompt);
            sub(&mut w.title);
        }
    }

    /// Total number of agents (leader + workers).
    pub fn agent_count(&self) -> usize {
        1 + self.worker.len()
    }
}
```

### 11.4 Variable Substitution

Templates support `{variable}` placeholders:

| Variable | Value |
|----------|-------|
| `{goal}` | User-provided goal text |
| `{team_name}` | Team name |
| `{timestamp}` | Unix timestamp |
| `{cwd}` | Current working directory |
| `{terminal_id}` | Terminal's UUID |
| `{leader_id}` | Leader terminal's UUID |
| `{worker_N_id}` | N-th worker's UUID |

---

## 12. Git Worktree Isolation

### 12.1 Why Worktrees

When multiple AI agents work on the same codebase simultaneously, they create
merge conflicts if they all edit files on the same branch. Git worktrees solve
this: each agent gets its own working directory on its own branch, sharing the
same `.git` directory.

### 12.2 Worktree Lifecycle

```
1. Team spawns
   └── For each worker:
       └── git worktree add /tmp/void-wt/{team}/{agent} -b void/{team}/{agent}

2. Worker works on its branch
   └── Edits files, commits normally

3. Worker completes task
   └── void-ctl task update $ID --status completed
   └── git add -A && git commit -m "Task: {subject}"

4. Leader merges
   └── For each completed worker:
       └── git merge void/{team}/{agent}
       └── Resolve conflicts (or report to user)

5. Team dissolves
   └── git worktree remove /tmp/void-wt/{team}/{agent}
   └── git branch -d void/{team}/{agent}
```

### 12.3 Merge Protocol

The leader agent (or the user) initiates merge:

```bash
# Leader merges worker branches
void-ctl workspace merge $TEAM $WORKER_NAME

# Or merge all completed workers
void-ctl workspace merge-all $TEAM
```

### 12.4 Implementation

```rust
// src/orchestration/worktree.rs — NEW FILE

use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

pub struct WorktreeManager {
    /// Base directory for worktrees.
    base_dir: PathBuf,

    /// Mapping: terminal_id → worktree path.
    worktrees: std::collections::HashMap<Uuid, PathBuf>,
}

impl WorktreeManager {
    pub fn new() -> Self {
        let base_dir = std::env::temp_dir().join("void-worktrees");
        std::fs::create_dir_all(&base_dir).ok();
        Self {
            base_dir,
            worktrees: std::collections::HashMap::new(),
        }
    }

    /// Create a worktree for a terminal.
    ///
    /// Returns the path to the worktree directory.
    pub fn create(
        &mut self,
        terminal_id: Uuid,
        team_name: &str,
        agent_name: &str,
        repo_root: &Path,
    ) -> Result<PathBuf, String> {
        let branch_name = format!("void/{}/{}", team_name, agent_name);
        let wt_path = self.base_dir.join(team_name).join(agent_name);

        // Create the worktree
        let output = Command::new("git")
            .current_dir(repo_root)
            .args(["worktree", "add", wt_path.to_str().unwrap(), "-b", &branch_name])
            .output()
            .map_err(|e| format!("git worktree add failed: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "git worktree add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        self.worktrees.insert(terminal_id, wt_path.clone());
        Ok(wt_path)
    }

    /// Get the worktree path for a terminal.
    pub fn get(&self, terminal_id: Uuid) -> Option<&PathBuf> {
        self.worktrees.get(&terminal_id)
    }

    /// Remove a worktree.
    pub fn remove(&mut self, terminal_id: Uuid, repo_root: &Path) -> Result<(), String> {
        if let Some(wt_path) = self.worktrees.remove(&terminal_id) {
            Command::new("git")
                .current_dir(repo_root)
                .args(["worktree", "remove", wt_path.to_str().unwrap(), "--force"])
                .output()
                .map_err(|e| format!("git worktree remove failed: {}", e))?;
        }
        Ok(())
    }

    /// Merge a worker's branch back to main.
    pub fn merge(
        &self,
        terminal_id: Uuid,
        repo_root: &Path,
        team_name: &str,
        agent_name: &str,
    ) -> Result<(), String> {
        let branch_name = format!("void/{}/{}", team_name, agent_name);

        let output = Command::new("git")
            .current_dir(repo_root)
            .args(["merge", &branch_name, "--no-edit"])
            .output()
            .map_err(|e| format!("git merge failed: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Merge conflict: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }

    /// Clean up all worktrees for a team.
    pub fn cleanup_team(&mut self, team_name: &str, repo_root: &Path) {
        let team_dir = self.base_dir.join(team_name);
        let ids_to_remove: Vec<Uuid> = self.worktrees
            .iter()
            .filter(|(_, path)| path.starts_with(&team_dir))
            .map(|(id, _)| *id)
            .collect();

        for id in ids_to_remove {
            self.remove(id, repo_root).ok();
        }

        std::fs::remove_dir_all(&team_dir).ok();
    }
}
```

---

## 13. CanvasPanel Enum Extension

### 13.1 New Variants

```rust
// src/panel.rs — MODIFIED

pub enum CanvasPanel {
    Terminal(TerminalPanel),
    Kanban(KanbanPanel),       // NEW
    Network(NetworkPanel),     // NEW
}
```

### 13.2 Trait Unification

Every method on `CanvasPanel` must handle all variants. The existing match arms
are extended:

```rust
impl CanvasPanel {
    pub fn id(&self) -> Uuid {
        match self {
            Self::Terminal(t) => t.id,
            Self::Kanban(k) => k.id,
            Self::Network(n) => n.id,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::Terminal(t) => &t.title,
            Self::Kanban(_) => "Kanban",
            Self::Network(_) => "Network",
        }
    }

    pub fn position(&self) -> Pos2 {
        match self {
            Self::Terminal(t) => t.position,
            Self::Kanban(k) => k.position,
            Self::Network(n) => n.position,
        }
    }

    pub fn set_position(&mut self, pos: Pos2) {
        match self {
            Self::Terminal(t) => t.position = pos,
            Self::Kanban(k) => k.position = pos,
            Self::Network(n) => n.position = pos,
        }
    }

    pub fn size(&self) -> Vec2 {
        match self {
            Self::Terminal(t) => t.size,
            Self::Kanban(k) => k.size,
            Self::Network(n) => n.size,
        }
    }

    pub fn is_alive(&self) -> bool {
        match self {
            Self::Terminal(t) => t.is_alive(),
            Self::Kanban(_) => true,  // always alive
            Self::Network(_) => true, // always alive
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        transform: egui::emath::TSTransform,
        screen_clip: Rect,
    ) -> PanelInteraction {
        match self {
            Self::Terminal(t) => t.show(ui, transform, screen_clip),
            Self::Kanban(k) => {
                let ki = k.show(ui, transform, screen_clip);
                // Convert KanbanInteraction to PanelInteraction
                match ki {
                    KanbanInteraction::DragStart => PanelInteraction::DragStart,
                    KanbanInteraction::FocusTerminal(id) => PanelInteraction::FocusRequest(id),
                    _ => PanelInteraction::None,
                }
            }
            Self::Network(n) => {
                let ni = n.show(ui, transform, screen_clip);
                // Convert NetworkInteraction to PanelInteraction
                match ni {
                    NetworkInteraction::DragStart => PanelInteraction::DragStart,
                    NetworkInteraction::FocusTerminal(id) => PanelInteraction::FocusRequest(id),
                    _ => PanelInteraction::None,
                }
            }
        }
    }

    // ... etc for all other methods
    // Methods that only apply to terminals (handle_input, sync_title)
    // are no-ops for Kanban and Network panels.
}
```

### 13.3 Persistence

Kanban and Network panels are NOT persisted to disk. They are recreated from
the orchestration session state when the workspace is restored. This keeps
persistence simple.

```rust
impl CanvasPanel {
    pub fn to_saved(&self) -> Option<PanelState> {
        match self {
            Self::Terminal(t) => Some(t.to_saved()),
            Self::Kanban(_) => None,  // not persisted
            Self::Network(_) => None, // not persisted
        }
    }
}

// In workspace save: filter out None values
pub fn to_saved(&self) -> WorkspaceState {
    WorkspaceState {
        panels: self.panels.iter().filter_map(|p| p.to_saved()).collect(),
        // ...
    }
}
```

---

## 14. Command Palette Extensions

New commands in the command palette (`Ctrl+Shift+P`):

| Command | Action |
|---------|--------|
| `Orchestration: Enable` | Toggle orchestration mode on |
| `Orchestration: Disable` | Toggle orchestration mode off |
| `Orchestration: Set Leader` | Make focused terminal the leader |
| `Orchestration: Spawn Worker` | Spawn a new worker terminal |
| `Orchestration: From Template...` | Show template picker |
| `Orchestration: Show Kanban` | Show/hide kanban board |
| `Orchestration: Show Network` | Show/hide network view |
| `Task: Create` | Create a task (prompt for subject) |
| `Task: List` | Show task list overlay |
| `Task: Complete Focused` | Complete the focused terminal's current task |

---

## 15. Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
| Toggle orchestration | `Ctrl+Shift+O` |
| Show kanban | `Ctrl+Shift+K` |
| Show network | `Ctrl+Shift+N` (if not taken by minimap) |
| Create task | `Ctrl+Shift+Enter` (when orchestration active) |
| Focus next agent | `Ctrl+Tab` (cycles through group terminals) |

---

## 16. Configuration (TOML)

```toml
# ~/.void/config.toml

[orchestration]
# Default agent CLI command
default_agent = "claude"

# Auto-inject coordination prompt on orchestration enable
auto_inject_prompt = true

# Git worktree isolation
enable_worktrees = true
worktree_base_dir = "/tmp/void-worktrees"

# Template search paths
template_dirs = ["~/.void/templates"]

# Kanban defaults
kanban_default_width = 800
kanban_default_height = 500

# Network view defaults
network_default_width = 600
network_default_height = 500

# Edge overlay
show_edge_overlay = true
particle_speed = 0.8
particle_trail_length = 3

# Task defaults
task_default_priority = 100
task_auto_start = true  # auto-start when unblocked and has owner
```

---

## 17. Security Model

The existing bus security model (§14 in orchestration-communication.md) applies.
Additional rules for tasks:

| Operation | Orchestrator | Worker (own task) | Worker (other's task) | Standalone |
|-----------|:---:|:---:|:---:|:---:|
| task.create | ✅ | ❌ | ❌ | N/A |
| task.assign | ✅ | Self only | ❌ | N/A |
| task.update (own) | ✅ | ✅ | ❌ | N/A |
| task.update (other) | ✅ | ❌ | ❌ | N/A |
| task.delete | ✅ | ❌ | ❌ | N/A |
| task.list | ✅ | ✅ (own team) | ✅ (own team) | N/A |
| task.get | ✅ | ✅ | ✅ | N/A |

---

## 18. Performance Budget

| Component | Target | Constraint |
|-----------|--------|-----------|
| Bus tick (tasks + statuses) | < 0.5ms per frame | Must not block egui paint |
| Kanban render | < 1ms per frame | Frustum cull when off-screen |
| Network render | < 2ms per frame | Force layout is O(n²), cap at 50 nodes |
| Edge overlay render | < 0.5ms per frame | Max 100 active particles |
| Particle physics | < 0.2ms per frame | Simple linear interpolation |
| Event processing | < 0.1ms per frame | Non-blocking channel drain |

Total orchestration overhead: **< 4.3ms per frame** (leaves plenty of room in a
16.6ms budget at 60fps).

Optimization strategies:
- Frustum culling: skip rendering panels outside the viewport
- Event coalescing: batch status updates, don't emit per-character
- Particle pooling: reuse particle objects instead of allocating
- Layout convergence: reduce force iterations when graph is stable

---

## 19. Implementation Plan — Phased

### Phase 1: Foundation (Week 1)
**Goal:** Task system + sidebar toggle. No visual panels yet.

Files to create:
- `src/bus/task.rs` — Task struct, TaskStatus, TaskInfo
- `src/orchestration/mod.rs` — OrchestrationSession

Files to modify:
- `src/bus/mod.rs` — Add task HashMap, task methods, tick_tasks()
- `src/bus/apc.rs` — Add task.* method dispatchers
- `src/bin/void-ctl.rs` — Add task subcommands
- `src/sidebar/mod.rs` — Add orchestration section
- `src/state/workspace.rs` — Add orchestration_enabled flag
- `src/app.rs` — Call tick_tasks() in update loop

Deliverable: You can enable orchestration in the sidebar, create tasks via
void-ctl, and see them in a terminal-based kanban (void-ctl task list).

### Phase 2: Kanban Board (Week 2)
**Goal:** Kanban canvas element rendering tasks visually.

Files to create:
- `src/kanban/mod.rs` — KanbanPanel struct + rendering

Files to modify:
- `src/panel.rs` — Add `Kanban` variant to CanvasPanel
- `src/state/workspace.rs` — Spawn kanban panel on orchestration enable
- `src/canvas/minimap.rs` — Render kanban as blue rect
- `src/app.rs` — Handle KanbanInteraction in update loop

Deliverable: A draggable kanban board on the canvas showing tasks by column.

### Phase 3: Network View (Week 3)
**Goal:** Network visualization with animated particles.

Files to create:
- `src/network/mod.rs` — NetworkPanel struct + rendering
- `src/canvas/edges.rs` — CanvasEdgeOverlay

Files to modify:
- `src/panel.rs` — Add `Network` variant to CanvasPanel
- `src/state/workspace.rs` — Spawn network panel on orchestration enable
- `src/canvas/minimap.rs` — Render network as purple rect
- `src/app.rs` — Handle NetworkInteraction, draw edge overlay

Deliverable: A live network graph with animated message particles between agents.
Connection lines visible on the canvas between terminal panels.

### Phase 4: Agent Protocol (Week 4)
**Goal:** Auto-prompt injection, templates, worktrees.

Files to create:
- `src/orchestration/prompt.rs` — Coordination prompt generation
- `src/orchestration/template.rs` — TOML template engine
- `src/orchestration/worktree.rs` — Git worktree manager
- `templates/duo.toml`
- `templates/trio.toml`
- `templates/fullstack.toml`
- `templates/research.toml`
- `templates/hedge-fund.toml`

Files to modify:
- `src/state/workspace.rs` — Inject prompts on orchestration enable
- `src/terminal/pty.rs` — Set additional env vars
- `src/sidebar/mod.rs` — Template picker
- `src/command_palette/commands.rs` — New orchestration commands

Deliverable: Full end-to-end flow. Enable orchestration → agents auto-receive
coordination prompts → work together with task tracking → visible on canvas.

---

## 20. File-by-File Change Map

```
NEW FILES:
  src/bus/task.rs                 (~250 lines) Task model + TaskStatus
  src/kanban/mod.rs               (~800 lines) KanbanPanel struct + rendering
  src/network/mod.rs              (~900 lines) NetworkPanel struct + rendering
  src/canvas/edges.rs             (~400 lines) CanvasEdgeOverlay
  src/orchestration/mod.rs        (~100 lines) OrchestrationSession
  src/orchestration/prompt.rs     (~200 lines) Prompt generation
  src/orchestration/template.rs   (~200 lines) TOML template engine
  src/orchestration/worktree.rs   (~150 lines) Git worktree manager
  templates/duo.toml              (~30 lines)
  templates/trio.toml             (~40 lines)
  templates/fullstack.toml        (~60 lines)
  templates/research.toml         (~50 lines)
  templates/hedge-fund.toml       (~80 lines)

MODIFIED FILES:
  src/bus/mod.rs                  (+300 lines) Task storage + methods + tick
  src/bus/types.rs                (+50 lines)  New BusEvent variants for tasks
  src/bus/apc.rs                  (+150 lines) Task method dispatchers
  src/bin/void-ctl.rs             (+200 lines) Task subcommands
  src/panel.rs                    (+80 lines)  Kanban + Network variants
  src/sidebar/mod.rs              (+200 lines) Orchestration section
  src/state/workspace.rs          (+100 lines) Orchestration session management
  src/app.rs                      (+100 lines) Tick tasks, edge overlay, interactions
  src/canvas/minimap.rs           (+20 lines)  Color for new panel types
  src/command_palette/commands.rs  (+30 lines)  New commands
  src/shortcuts/default_bindings.rs (+10 lines) New shortcuts
  src/terminal/pty.rs             (+20 lines)  Additional env vars
  src/main.rs                     (+5 lines)   Module declarations
  Cargo.toml                      (+2 lines)   toml dependency

TOTAL: ~3,900 new lines + ~1,265 modified lines ≈ ~5,165 lines of change
(within the 6,000-8,000 estimate including comments and whitespace)
```

---

## 21. Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_lifecycle() {
        let mut bus = TerminalBus::new();
        // Register terminals, create group, create tasks
        // Verify status transitions
        // Verify auto-unblock
        // Verify permission enforcement
    }

    #[test]
    fn task_dependency_cycle_detection() {
        // T1 blocked by T2, T2 blocked by T1 → error
    }

    #[test]
    fn task_auto_unblock() {
        // T2 blocked by T1. Complete T1 → T2 becomes pending
    }

    #[test]
    fn task_permission_worker_cannot_create() {
        // Worker tries to create task → PermissionDenied
    }

    #[test]
    fn task_self_assign() {
        // Worker self-assigns unassigned task → ok
    }

    #[test]
    fn kanban_column_sorting() {
        // Tasks sorted by priority within columns
    }

    #[test]
    fn network_force_layout_convergence() {
        // Layout converges after N iterations
    }

    #[test]
    fn edge_particle_lifecycle() {
        // Particle spawns, travels, arrives, gets cleaned up
    }

    #[test]
    fn template_parsing() {
        // Load TOML, verify fields, test variable substitution
    }

    #[test]
    fn worktree_create_and_cleanup() {
        // Create worktree, verify path, remove, verify cleanup
    }
}
```

### Integration Tests

```bash
# Test: full orchestration flow via void-ctl
# 1. Start Void with orchestration enabled
# 2. void-ctl task create "Test task"
# 3. void-ctl task list → verify task appears
# 4. void-ctl task update $ID --status completed
# 5. Verify dependent task unblocks
```

### Manual Testing Checklist

- [ ] Enable orchestration via sidebar toggle
- [ ] Kanban appears on canvas, shows empty columns
- [ ] Network view appears on canvas, shows leader node
- [ ] Spawn worker → node appears in network view
- [ ] Create task → card appears in kanban PENDING column
- [ ] Assign task → card shows owner
- [ ] Start task → card moves to IN PROGRESS
- [ ] Complete task → card moves to DONE, dependents unblock
- [ ] Send message → particle animates in network view
- [ ] Send command → particle animates on canvas edge overlay
- [ ] Disable orchestration → kanban and network removed
- [ ] Load template → all agents spawn with prompts
- [ ] Zoom out → see entire swarm (terminals + kanban + network)
- [ ] Zoom in → interact with individual panels

---

## 22. Open Questions

1. **Task persistence across sessions?** Currently tasks live in memory only.
   Should we persist to `~/.void/tasks.json`? Pro: survive restarts. Con: stale
   tasks from old sessions.

2. **Multiple simultaneous teams per workspace?** Currently one team per workspace.
   Supporting multiple teams adds complexity to the sidebar and kanban.

3. **Remote orchestration?** ClawTeam supports cross-machine via NFS/ZeroMQ.
   We could add a WebSocket layer to the bus server for remote terminals.
   Deferred to v2.

4. **Kanban drag-and-drop?** Should users be able to drag cards between columns
   to change status? This is intuitive but might conflict with agent autonomy.

5. **Network panel: 3D view?** A 3D force-directed graph would look amazing
   (we already have wgpu), but adds significant complexity. Deferred.

6. **Agent binary detection?** Should Void auto-detect which agent CLIs are
   installed and offer only those in the template picker?

7. **Sound effects?** A subtle chime when a task completes or a message arrives.
   Could be annoying. Make it configurable.

8. **Shared terminal view?** The network panel could embed a tiny preview of each
   terminal's screen (like a thumbnail). Feasible with the existing grid reader
   but expensive at scale.

---

*End of PRD-ORCHESTRATION.md*

*This document specifies ~5,000-8,000 lines of new Rust code to transform Void
from an infinite canvas terminal emulator into a full AI swarm intelligence
cockpit. Every feature builds on the existing Terminal Bus foundation (PR #16).
Zero external dependencies beyond `toml` for template parsing. 100% Rust.
Cross-platform. GPU-accelerated.*