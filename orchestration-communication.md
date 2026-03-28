# Terminal Orchestration & Communication System

> Void is not just a terminal emulator. It is a workspace where terminals collaborate.

---

## Table of Contents

1. [Vision](#1-vision)
2. [Architecture Overview](#2-architecture-overview)
3. [Core Concepts](#3-core-concepts)
4. [Data Structures](#4-data-structures)
5. [Terminal Bus — In-Process Registry](#5-terminal-bus--in-process-registry)
6. [Terminal Groups](#6-terminal-groups)
7. [Communication Protocol — APC Escape Sequences](#7-communication-protocol--apc-escape-sequences)
8. [APC Interception Layer](#8-apc-interception-layer)
9. [void-ctl CLI](#9-void-ctl-cli)
10. [Title Bar Status Integration](#10-title-bar-status-integration)
11. [Shared Context Store](#11-shared-context-store)
12. [Event & Subscription System](#12-event--subscription-system)
13. [Integration with Existing Code](#13-integration-with-existing-code)
14. [Security Model](#14-security-model)
15. [Usage Scenarios](#15-usage-scenarios)
16. [API Reference](#16-api-reference)
17. [Testing Strategy](#17-testing-strategy)
18. [Future Extensions](#18-future-extensions)

---

## 1. Vision

Every terminal in Void runs in the same process. They share the same memory space.
They already have `Arc<Mutex<>>` handles to each other's state machines. They already
have writers that can inject bytes. They already have grid readers that can extract text.

The terminals are *already connected*. They just don't know it yet.

This document describes the system that makes that connection explicit: a Terminal Bus
for in-process communication, an APC escape sequence protocol for child-process access
through the existing PTY pipe, and a Group system that lets terminals form teams — with
one orchestrator directing workers, or peers collaborating as equals.

The primary use case: an AI agent (Claude Code) running in terminal A orchestrates
other terminals — sending commands, reading output, sharing discoveries — while the
user watches everything happen simultaneously on the infinite canvas.

### Design Principles

- **Zero external dependencies for the bus.** The in-process layer uses only `std::sync`
  and `std::collections`. No async runtime. No message broker. Just Rust.

- **No socket, no server, no auth tokens.** Communication happens through the PTY
  pipe that already exists, using APC (Application Program Command) escape sequences.
  The same pipe that carries keyboard input and terminal output carries orchestration
  commands. Cross-platform by default — works identically on Windows, Linux, and macOS.

- **Opt-in complexity.** A terminal that never joins a group behaves exactly as it does
  today. The orchestration system is additive, not invasive.

- **Shell-native interface.** The `void-ctl` CLI writes APC sequences to stdout and
  reads responses from stdin — through the PTY pipe. Any process that can run a shell
  command can orchestrate terminals. No SDK required.

---

## 2. Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│  VoidApp Process                                                    │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                     Terminal Bus                               │  │
│  │                                                               │  │
│  │  ┌─────────────────────────────────────────────────────────┐  │  │
│  │  │  Terminal Registry                                      │  │  │
│  │  │  HashMap<Uuid, TerminalHandle>                          │  │  │
│  │  │                                                         │  │  │
│  │  │  ┌──────────┐  ┌──────────┐  ┌──────────┐             │  │  │
│  │  │  │ Term A   │  │ Term B   │  │ Term C   │  ...        │  │  │
│  │  │  │ writer ──┤  │ writer ──┤  │ writer ──┤             │  │  │
│  │  │  │ term   ──┤  │ term   ──┤  │ term   ──┤             │  │  │
│  │  │  │ status   │  │ status   │  │ status   │             │  │  │
│  │  │  └──────────┘  └──────────┘  └──────────┘             │  │  │
│  │  └─────────────────────────────────────────────────────────┘  │  │
│  │                                                               │  │
│  │  ┌─────────────────────────────────────────────────────────┐  │  │
│  │  │  Group Registry                                         │  │  │
│  │  │  HashMap<Uuid, TerminalGroup>                           │  │  │
│  │  │                                                         │  │  │
│  │  │  ┌──────────────────────┐  ┌──────────────────────┐    │  │  │
│  │  │  │ Group: "build"       │  │ Group: "research"     │    │  │  │
│  │  │  │ mode: Orchestrated   │  │ mode: Peer            │    │  │  │
│  │  │  │ parent: Term A       │  │ members: [D, E, F]    │    │  │  │
│  │  │  │ workers: [B, C]      │  │                       │    │  │  │
│  │  │  └──────────────────────┘  └──────────────────────┘    │  │  │
│  │  └─────────────────────────────────────────────────────────┘  │  │
│  │                                                               │  │
│  │  ┌─────────────────────────────────────────────────────────┐  │  │
│  │  │  Shared Context Store                                   │  │  │
│  │  │  HashMap<String, ContextEntry>                          │  │  │
│  │  │                                                         │  │  │
│  │  │  "test_results"   => "142 passed, 0 failed"            │  │  │
│  │  │  "lint_output"    => "warning: unused variable..."     │  │  │
│  │  │  "build:status"   => "success"                         │  │  │
│  │  └─────────────────────────────────────────────────────────┘  │  │
│  │                                                               │  │
│  │  ┌─────────────────────────────────────────────────────────┐  │  │
│  │  │  Event Bus                                              │  │  │
│  │  │  broadcast::Sender<BusEvent>                            │  │  │
│  │  │  -> subscribers receive filtered events                 │  │  │
│  │  └─────────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  APC Interception (in each terminal's reader thread)          │  │
│  │                                                               │  │
│  │  Reader thread scans PTY output for \x1b_VOID;...\x1b\\      │  │
│  │  Strips APC sequences before feeding to VTE parser            │  │
│  │  Routes commands to TerminalBus                               │  │
│  │  Writes response APC back to PTY stdin                        │  │
│  │                                                               │  │
│  │  No socket. No auth token. No extra port.                     │  │
│  │  The PTY pipe IS the communication channel.                   │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘

          ┌────────────────────────────────────────────┐
          │  void-ctl (standalone Rust binary)         │
          │                                            │
          │  Runs inside a Void terminal               │
          │  Writes APC request to stdout (→ PTY)      │
          │  Reads APC response from stdin  (← PTY)    │
          │  Reads VOID_TERMINAL_ID from env           │
          │                                            │
          │  Subcommands:                              │
          │    list, send, read, wait-idle,            │
          │    group create/join/leave/list,            │
          │    context set/get/list/delete,             │
          │    status, spawn, close                     │
          │                                            │
          │  Used by Claude Code, scripts, humans      │
          └────────────────────────────────────────────┘
```

### Data Flow — Command Injection

```
Claude Code (in Term A)
  │
  │  $ void-ctl send <term-B-id> "cargo test"
  │
  ▼
void-ctl binary (child process in Term A's PTY)
  │  writes to stdout: \x1b_VOID;req-1;inject;{"target":"<B>","command":"cargo test"}\x1b\\
  │
  ▼
Term A's PTY pipe (stdout → PTY slave → PTY master → Void reader thread)
  │
  ▼
Term A's Reader Thread (pty.rs)
  │  scans bytes, finds \x1b_VOID;... APC sequence
  │  strips it from buffer (VTE parser never sees it)
  │  parses method: "inject", params: {target, command}
  │  calls bus.inject_bytes(target_B, "cargo test\r")
  │
  ▼
Terminal Bus
  │  looks up TerminalHandle for <B>
  │  locks writer: Arc<Mutex<Box<dyn Write>>>
  │  writer.write_all(b"cargo test\r")
  │  writer.flush()
  │  updates status: Running { command: "cargo test" }
  │  emits BusEvent::CommandInjected { source: A, target: B }
  │
  ▼
Terminal B's PTY
  │  receives "cargo test\r" on stdin
  │  shell executes cargo test
  │  output flows through reader thread -> Term state machine
  │
  ▼
Terminal B's screen updates (visible on canvas)
  Title bar shows: [build ▼ running]
  │
  ▼
Reader Thread writes response APC back to Term A's PTY stdin:
  \x1b_VOID-R;req-1;{"ok":true}\x1b\\
  │
  ▼
void-ctl reads response from stdin, prints "Sent."
```

### Data Flow — Output Reading

```
Claude Code (in Term A)
  │
  │  $ void-ctl read <term-B-id> --lines 50
  │
  ▼
void-ctl binary
  │  writes to stdout: \x1b_VOID;req-2;read_output;{"target":"<B>","lines":50}\x1b\\
  │
  ▼
Term A's Reader Thread
  │  intercepts APC, calls bus.read_output(target_B, 50)
  │
  ▼
Terminal Bus
  │  looks up TerminalHandle for <B>
  │  locks term: Arc<Mutex<Term<EventProxy>>>
  │  iterates grid rows, extracts text per cell
  │  returns last 50 lines of visible + scrollback
  │
  ▼
Reader Thread writes response APC to Term A's PTY stdin:
  \x1b_VOID-R;req-2;{"lines":["$ cargo test","running 42 tests",...]}\x1b\\
  │
  ▼
void-ctl reads response from stdin, prints each line
  │
  ▼
Claude Code captures it in a variable
  TEST_OUTPUT=$(void-ctl read <term-B-id> --lines 50)
```

---

## 3. Core Concepts

### 3.1 Terminal Handle

A `TerminalHandle` is a lightweight, cloneable reference to a terminal's internal state.
It holds `Arc` clones of the same objects that `PtyHandle` owns. Creating a handle does
not create a new terminal — it creates a *window* into an existing one.

Since `PtyHandle` already stores `term`, `writer`, `title`, `alive`, `last_input_at`,
and `last_output_at` as `Arc<Mutex<>>` / `Arc<AtomicBool>`, cloning these Arcs into a
TerminalHandle is zero-cost and does not change PtyHandle's ownership model.

### 3.2 Terminal Group

A group is a named collection of terminals that can communicate. Groups have two modes:

**Orchestrated Mode**: One terminal is the orchestrator (parent). It can send commands
to workers, read their output, and manage their lifecycle. Workers know who their parent
is and can send messages back. This is the model for AI agent orchestration.

```
    ┌──────────────┐
    │ Orchestrator  │
    │  (Term A)     │
    └──┬─────┬──────┘
       │     │
  ┌────▼──┐ ┌▼──────┐
  │Worker │ │Worker  │
  │(Term B)│ │(Term C)│
  └───────┘ └────────┘
```

**Peer Mode**: All terminals are equal. Any member can send to any other member. There
is no parent. This is the model for collaborative workflows where multiple agents
work on different aspects of a problem and share findings.

```
  ┌────────┐     ┌────────┐
  │ Peer A  │◄───►│ Peer B  │
  └────┬────┘     └────┬────┘
       │               │
       │  ┌────────┐   │
       └──► Peer C  ◄──┘
          └────────┘
```

### 3.3 Terminal Status

Each terminal in a group has a status that is visible in its title bar:

| Status    | Meaning                                        | Title Indicator |
|-----------|------------------------------------------------|-----------------|
| `idle`    | Shell prompt visible, waiting for input         | `[group ▲ idle]` or `[group ▼ idle]` |
| `running` | Command is executing, output is flowing         | `[group ▼ running]` |
| `waiting` | Waiting for input or for another terminal       | `[group ▼ waiting]` |
| `done`    | Last command completed, results available       | `[group ▼ done]` |
| `error`   | Last command failed (non-zero exit or timeout)  | `[group ▼ error]` |

The `▲` arrow indicates orchestrator. The `▼` arrow indicates worker. The `◆` diamond
indicates peer mode.

### 3.4 Shared Context

The shared context is a key-value store scoped to the entire bus (global) or to a
specific group (namespaced). It lets terminals share structured data without going
through the terminal's text buffer.

Context entries have:
- A key (string)
- A value (string, can be multi-line)
- A source terminal ID (who wrote it)
- A timestamp (when it was written)
- An optional TTL (time-to-live, auto-expire)

### 3.5 Bus Events

Every significant action on the bus produces an event. Terminals (or internal
subscribers) can subscribe to events with filters:

- `CommandInjected { source, target, command }`
- `OutputChanged { terminal_id }`
- `StatusChanged { terminal_id, old_status, new_status }`
- `TerminalRegistered { terminal_id }`
- `TerminalExited { terminal_id }`
- `GroupCreated { group_id, name }`
- `GroupMemberJoined { group_id, terminal_id, role }`
- `GroupMemberLeft { group_id, terminal_id }`
- `ContextUpdated { key, source_terminal }`
- `MessageSent { from, to, payload }`

---

## 4. Data Structures

### 4.1 Complete Type Definitions

```rust
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
#[derive(Debug, Clone, PartialEq)]
pub enum TerminalStatus {
    /// Shell prompt is visible, no command running.
    /// Detected when `last_output_at` has not changed for `idle_threshold`.
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

impl Default for TerminalStatus {
    fn default() -> Self {
        Self::Idle
    }
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
            Self::Orchestrator => "\u{25B2}",  // ▲
            Self::Worker => "\u{25BC}",        // ▼
            Self::Peer => "\u{25C6}",          // ◆
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
pub enum BusEvent {
    /// A terminal was registered with the bus (new terminal spawned).
    TerminalRegistered {
        terminal_id: Uuid,
        title: String,
    },

    /// A terminal's child process exited.
    TerminalExited {
        terminal_id: Uuid,
    },

    /// Bytes were injected into a terminal by another terminal or void-ctl.
    CommandInjected {
        source: Option<Uuid>,
        target: Uuid,
        command: String,
    },

    /// A terminal's output buffer changed (new data from PTY).
    /// This event is coalesced — at most one per terminal per 100ms.
    OutputChanged {
        terminal_id: Uuid,
    },

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
    GroupMemberLeft {
        group_id: Uuid,
        terminal_id: Uuid,
    },

    /// A group was dissolved (last member left or explicit dissolve).
    GroupDissolved {
        group_id: Uuid,
        name: String,
    },

    /// A context entry was created or updated.
    ContextUpdated {
        key: String,
        source: Uuid,
    },

    /// A context entry was deleted.
    ContextDeleted {
        key: String,
    },

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
        if !self.event_types.is_empty()
            && !self.event_types.iter().any(|t| t == event.event_type())
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
```

---

## 5. Terminal Bus — In-Process Registry

The bus is the heart of the orchestration system. It is a single struct owned by
`VoidApp` behind an `Arc<Mutex<>>`. All operations go through the bus.

### 5.1 Bus Implementation

```rust
// src/bus/mod.rs

pub mod types;

use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use alacritty_terminal::grid::Dimensions;
use uuid::Uuid;

use types::*;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// How long a terminal must be silent before it is considered idle.
const IDLE_THRESHOLD: Duration = Duration::from_secs(2);

/// Maximum number of events buffered per subscriber before dropping.
const EVENT_CHANNEL_CAPACITY: usize = 256;

/// Maximum number of lines that can be read in a single read_output call.
const MAX_READ_LINES: usize = 10_000;

// ---------------------------------------------------------------------------
// Terminal Bus
// ---------------------------------------------------------------------------

/// The central registry and communication hub for all terminals.
///
/// Thread-safe: all public methods acquire internal locks as needed.
/// The bus itself is behind `Arc<Mutex<TerminalBus>>` in VoidApp.
pub struct TerminalBus {
    /// All registered terminals, keyed by UUID.
    terminals: HashMap<Uuid, TerminalHandle>,

    /// Terminal status (separate from TerminalHandle to avoid nested locking).
    statuses: HashMap<Uuid, TerminalStatus>,

    /// All active groups, keyed by UUID.
    groups: HashMap<Uuid, TerminalGroup>,

    /// Mapping from terminal ID to its group ID (if any).
    terminal_to_group: HashMap<Uuid, Uuid>,

    /// Shared context store.
    context: HashMap<String, ContextEntry>,

    /// Event subscribers. Each subscriber gets a Sender end.
    /// Subscribers are identified by a unique ID for cleanup.
    subscribers: Vec<(Uuid, EventFilter, mpsc::Sender<BusEvent>)>,
}

impl TerminalBus {
    /// Create a new, empty bus.
    pub fn new() -> Self {
        Self {
            terminals: HashMap::new(),
            statuses: HashMap::new(),
            groups: HashMap::new(),
            terminal_to_group: HashMap::new(),
            context: HashMap::new(),
            subscribers: Vec::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Terminal Registration
    // -----------------------------------------------------------------------

    /// Register a terminal with the bus.
    ///
    /// Called by `Workspace::spawn_terminal()` after creating a PtyHandle.
    /// The `handle` is built from cloned `Arc`s of the PtyHandle's fields.
    pub fn register(&mut self, handle: TerminalHandle) {
        let id = handle.id;
        let title = handle.title.lock().map(|t| t.clone()).unwrap_or_default();

        self.statuses.insert(id, TerminalStatus::Idle);
        self.terminals.insert(id, handle);

        self.emit(BusEvent::TerminalRegistered {
            terminal_id: id,
            title,
        });
    }

    /// Deregister a terminal from the bus.
    ///
    /// Called by `Workspace::close_panel()` or when a terminal's child process exits.
    /// Automatically removes the terminal from its group.
    pub fn deregister(&mut self, terminal_id: Uuid) {
        // Remove from group first
        if let Some(group_id) = self.terminal_to_group.remove(&terminal_id) {
            self.remove_from_group_inner(terminal_id, group_id);
        }

        self.terminals.remove(&terminal_id);
        self.statuses.remove(&terminal_id);

        self.emit(BusEvent::TerminalExited { terminal_id });
    }

    // -----------------------------------------------------------------------
    // Terminal Queries
    // -----------------------------------------------------------------------

    /// List all registered terminals with their current info.
    pub fn list_terminals(&self) -> Vec<TerminalInfo> {
        self.terminals
            .values()
            .map(|h| self.build_terminal_info(h))
            .collect()
    }

    /// Get info for a specific terminal.
    pub fn get_terminal(&self, id: Uuid) -> Option<TerminalInfo> {
        self.terminals.get(&id).map(|h| self.build_terminal_info(h))
    }

    /// Check if a terminal is alive.
    pub fn is_alive(&self, id: Uuid) -> Option<bool> {
        self.terminals
            .get(&id)
            .map(|h| h.alive.load(Ordering::Relaxed))
    }

    fn build_terminal_info(&self, handle: &TerminalHandle) -> TerminalInfo {
        let title = handle
            .title
            .lock()
            .map(|t| t.clone())
            .unwrap_or_default();
        let alive = handle.alive.load(Ordering::Relaxed);
        let status = self
            .statuses
            .get(&handle.id)
            .cloned()
            .unwrap_or_default();
        let group_id = self.terminal_to_group.get(&handle.id).copied();
        let (group_name, role) = if let Some(gid) = group_id {
            let group = self.groups.get(&gid);
            let name = group.map(|g| g.name.clone());
            let role = group
                .and_then(|g| g.role_of(handle.id))
                .unwrap_or(TerminalRole::Standalone);
            (name, role)
        } else {
            (None, TerminalRole::Standalone)
        };
        let last_output_elapsed_ms = handle
            .last_output_at
            .lock()
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        let last_input_elapsed_ms = handle
            .last_input_at
            .lock()
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);

        TerminalInfo {
            id: handle.id,
            title,
            alive,
            workspace_id: handle.workspace_id,
            group_id,
            group_name,
            role,
            status,
            last_output_elapsed_ms,
            last_input_elapsed_ms,
        }
    }

    // -----------------------------------------------------------------------
    // Command Injection
    // -----------------------------------------------------------------------

    /// Inject bytes into a terminal's PTY stdin.
    ///
    /// This is the primary mechanism for one terminal to send commands to another.
    /// The bytes are written directly to the PTY writer, exactly as if the user
    /// had typed them.
    ///
    /// To send a command and press Enter: `inject_bytes(target, b"cargo test\r")`
    /// To send Ctrl+C: `inject_bytes(target, b"\x03")`
    ///
    /// # Arguments
    /// * `target` - UUID of the target terminal
    /// * `bytes` - Raw bytes to inject (including \r for Enter, \x03 for Ctrl+C, etc.)
    /// * `source` - UUID of the terminal that initiated the injection (for audit trail)
    ///
    /// # Errors
    /// Returns an error if the target terminal is not found, is dead, or the write fails.
    pub fn inject_bytes(
        &mut self,
        target: Uuid,
        bytes: &[u8],
        source: Option<Uuid>,
    ) -> Result<(), BusError> {
        let handle = self
            .terminals
            .get(&target)
            .ok_or(BusError::TerminalNotFound(target))?;

        if !handle.alive.load(Ordering::Relaxed) {
            return Err(BusError::TerminalDead(target));
        }

        // Permission check: in orchestrated mode, only the orchestrator can inject
        // into workers. Workers cannot inject into the orchestrator or other workers.
        if let Some(src) = source {
            self.check_injection_permission(src, target)?;
        }

        // Write to PTY
        let mut writer = handle
            .writer
            .lock()
            .map_err(|_| BusError::LockFailed("writer"))?;
        writer
            .write_all(bytes)
            .map_err(|e| BusError::WriteFailed(e.to_string()))?;
        writer
            .flush()
            .map_err(|e| BusError::WriteFailed(e.to_string()))?;
        drop(writer);

        // Update status to Running
        let command_str = String::from_utf8_lossy(bytes)
            .trim_end_matches('\r')
            .trim_end_matches('\n')
            .to_string();

        if !command_str.is_empty() && bytes != b"\x03" {
            self.statuses.insert(
                target,
                TerminalStatus::Running {
                    command: Some(command_str.clone()),
                    started_at: Instant::now(),
                },
            );
        }

        self.emit(BusEvent::CommandInjected {
            source,
            target,
            command: command_str,
        });

        Ok(())
    }

    /// Send a command string to a terminal (convenience wrapper).
    ///
    /// Appends \r (Enter) to the command. Use `inject_bytes` for raw byte control.
    pub fn send_command(
        &mut self,
        target: Uuid,
        command: &str,
        source: Option<Uuid>,
    ) -> Result<(), BusError> {
        let mut bytes = command.as_bytes().to_vec();
        bytes.push(b'\r');
        self.inject_bytes(target, &bytes, source)
    }

    /// Send Ctrl+C (SIGINT) to a terminal.
    pub fn send_interrupt(&mut self, target: Uuid, source: Option<Uuid>) -> Result<(), BusError> {
        self.inject_bytes(target, b"\x03", source)
    }

    /// Check whether `source` is allowed to inject into `target`.
    fn check_injection_permission(
        &self,
        source: Uuid,
        target: Uuid,
    ) -> Result<(), BusError> {
        let source_group = self.terminal_to_group.get(&source);
        let target_group = self.terminal_to_group.get(&target);

        match (source_group, target_group) {
            // Both in the same group
            (Some(sg), Some(tg)) if sg == tg => {
                let group = &self.groups[sg];
                match &group.mode {
                    GroupMode::Orchestrated { orchestrator } => {
                        // Orchestrator can inject into any worker
                        if *orchestrator == source {
                            Ok(())
                        }
                        // Workers can send messages to orchestrator (limited)
                        else if *orchestrator == target {
                            Ok(())
                        }
                        // Workers cannot inject into other workers
                        else {
                            Err(BusError::PermissionDenied(
                                "workers cannot inject into other workers".into(),
                            ))
                        }
                    }
                    GroupMode::Peer => {
                        // Peers can inject into any other peer
                        Ok(())
                    }
                }
            }
            // Not in the same group — allow (no group restrictions apply)
            _ => Ok(()),
        }
    }

    // -----------------------------------------------------------------------
    // Output Reading
    // -----------------------------------------------------------------------

    /// Read the visible screen content of a terminal.
    ///
    /// Returns the text currently displayed on the terminal screen, line by line.
    /// This is equivalent to what the user sees in the terminal panel.
    ///
    /// # Arguments
    /// * `target` - UUID of the terminal to read
    ///
    /// # Returns
    /// A vector of strings, one per screen line.
    pub fn read_screen(&self, target: Uuid) -> Result<Vec<String>, BusError> {
        let handle = self
            .terminals
            .get(&target)
            .ok_or(BusError::TerminalNotFound(target))?;

        let term = handle
            .term
            .lock()
            .map_err(|_| BusError::LockFailed("term"))?;

        let content = term.renderable_content();
        let cols = term.columns();
        let lines = term.screen_lines();

        let mut result = Vec::with_capacity(lines);
        let mut current_line = String::with_capacity(cols);
        let mut current_row = 0i32;

        // Build initial empty lines
        for _ in 0..lines {
            result.push(String::new());
        }

        for indexed in content.display_iter {
            let row = indexed.point.line.0 as usize;
            if row < lines {
                let c = indexed.cell.c;
                if c != ' ' || !result[row].is_empty() {
                    // Pad with spaces if needed
                    let col = indexed.point.column.0;
                    while result[row].len() < col {
                        result[row].push(' ');
                    }
                    if c != '\0' {
                        result[row].push(c);
                    }
                }
            }
        }

        // Trim trailing whitespace from each line
        for line in &mut result {
            let trimmed = line.trim_end().to_string();
            *line = trimmed;
        }

        Ok(result)
    }

    /// Read the last N lines of output, including scrollback.
    ///
    /// This reads from the terminal's scrollback buffer, not just the visible screen.
    /// Useful for capturing command output that has scrolled off screen.
    ///
    /// # Arguments
    /// * `target` - UUID of the terminal to read
    /// * `lines` - Number of lines to read (from the bottom)
    ///
    /// # Returns
    /// A vector of strings, one per line, most recent last.
    pub fn read_output(
        &self,
        target: Uuid,
        lines: usize,
    ) -> Result<Vec<String>, BusError> {
        let lines = lines.min(MAX_READ_LINES);

        let handle = self
            .terminals
            .get(&target)
            .ok_or(BusError::TerminalNotFound(target))?;

        let term = handle
            .term
            .lock()
            .map_err(|_| BusError::LockFailed("term"))?;

        let grid = term.grid();
        let total_lines = grid.screen_lines() + grid.history_size();
        let cols = term.columns();
        let read_count = lines.min(total_lines);

        let mut result = Vec::with_capacity(read_count);

        // Read from the grid. In alacritty_terminal, line 0 is the topmost
        // visible line, negative lines are scrollback.
        // We want the last `read_count` lines of the entire buffer.

        let screen_lines = grid.screen_lines();
        let history = grid.history_size();

        // Start from (screen_lines - read_count) counting from the bottom
        let start_offset = if read_count <= screen_lines {
            // All within visible screen
            (screen_lines - read_count) as i32
        } else {
            // Need to go into scrollback
            -((read_count - screen_lines) as i32)
        };

        for i in 0..read_count {
            let line_idx = start_offset + i as i32;
            let mut line_str = String::with_capacity(cols);

            for col in 0..cols {
                let point = alacritty_terminal::index::Point::new(
                    alacritty_terminal::index::Line(line_idx),
                    alacritty_terminal::index::Column(col),
                );
                // Bounds check before accessing
                if line_idx >= -(history as i32) && line_idx < screen_lines as i32 {
                    let cell = &grid[point];
                    let c = cell.c;
                    if c == '\0' {
                        line_str.push(' ');
                    } else {
                        line_str.push(c);
                    }
                }
            }

            result.push(line_str.trim_end().to_string());
        }

        Ok(result)
    }

    /// Read the full screen content as a single string (lines joined with \n).
    pub fn read_screen_text(&self, target: Uuid) -> Result<String, BusError> {
        let lines = self.read_screen(target)?;
        Ok(lines.join("\n"))
    }

    /// Read the last N lines as a single string (lines joined with \n).
    pub fn read_output_text(&self, target: Uuid, lines: usize) -> Result<String, BusError> {
        let output = self.read_output(target, lines)?;
        Ok(output.join("\n"))
    }

    // -----------------------------------------------------------------------
    // Idle Detection
    // -----------------------------------------------------------------------

    /// Check if a terminal appears idle (no output for `IDLE_THRESHOLD`).
    pub fn is_idle(&self, target: Uuid) -> Result<bool, BusError> {
        let handle = self
            .terminals
            .get(&target)
            .ok_or(BusError::TerminalNotFound(target))?;

        let elapsed = handle
            .last_output_at
            .lock()
            .map(|t| t.elapsed())
            .map_err(|_| BusError::LockFailed("last_output_at"))?;

        Ok(elapsed >= IDLE_THRESHOLD)
    }

    /// Block until a terminal becomes idle or a timeout is reached.
    ///
    /// This is a polling implementation. The APC handler calls this in the
    /// reader thread to avoid blocking the bus mutex.
    ///
    /// # Arguments
    /// * `target` - UUID of the terminal to watch
    /// * `timeout` - Maximum time to wait
    /// * `quiet_period` - How long the terminal must be silent to be considered idle
    ///
    /// # Returns
    /// `true` if the terminal became idle, `false` if the timeout was reached.
    pub fn wait_idle_handle(
        handle: &TerminalHandle,
        timeout: Duration,
        quiet_period: Duration,
    ) -> bool {
        let deadline = Instant::now() + timeout;

        loop {
            if Instant::now() >= deadline {
                return false;
            }

            let elapsed = handle
                .last_output_at
                .lock()
                .map(|t| t.elapsed())
                .unwrap_or(Duration::ZERO);

            if elapsed >= quiet_period {
                return true;
            }

            // Don't hold any locks while sleeping
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Get a clone of a terminal handle for use outside the bus lock.
    ///
    /// This is used by `wait_idle` to poll without holding the bus mutex.
    pub fn get_handle(&self, target: Uuid) -> Option<TerminalHandle> {
        self.terminals.get(&target).cloned()
    }

    // -----------------------------------------------------------------------
    // Status Management
    // -----------------------------------------------------------------------

    /// Get the current status of a terminal.
    pub fn get_status(&self, target: Uuid) -> Option<&TerminalStatus> {
        self.statuses.get(&target)
    }

    /// Manually set the status of a terminal.
    ///
    /// Used by the orchestrator to mark terminals as waiting, done, or error.
    /// Also used internally after command injection.
    pub fn set_status(
        &mut self,
        target: Uuid,
        status: TerminalStatus,
        source: Option<Uuid>,
    ) -> Result<(), BusError> {
        if !self.terminals.contains_key(&target) {
            return Err(BusError::TerminalNotFound(target));
        }

        // Permission: only orchestrator or the terminal itself can set status
        if let Some(src) = source {
            if src != target {
                let target_group = self.terminal_to_group.get(&target);
                if let Some(gid) = target_group {
                    let group = &self.groups[gid];
                    if !group.is_orchestrator(src) {
                        return Err(BusError::PermissionDenied(
                            "only orchestrator can set worker status".into(),
                        ));
                    }
                }
            }
        }

        let old = self
            .statuses
            .get(&target)
            .map(|s| s.label().to_string())
            .unwrap_or_default();
        let new_label = status.label().to_string();

        self.statuses.insert(target, status);

        if old != new_label {
            self.emit(BusEvent::StatusChanged {
                terminal_id: target,
                old_status: old,
                new_status: new_label,
            });
        }

        Ok(())
    }

    /// Auto-update statuses based on output activity.
    ///
    /// Called periodically by VoidApp::update() (every frame).
    /// Transitions: Running -> Done (if idle for IDLE_THRESHOLD after a command).
    pub fn tick_statuses(&mut self) {
        let mut transitions = Vec::new();

        for (id, status) in &self.statuses {
            if let TerminalStatus::Running { started_at, .. } = status {
                if let Some(handle) = self.terminals.get(id) {
                    let output_elapsed = handle
                        .last_output_at
                        .lock()
                        .map(|t| t.elapsed())
                        .unwrap_or(Duration::ZERO);

                    // Terminal has been silent for IDLE_THRESHOLD after a command
                    if output_elapsed >= IDLE_THRESHOLD
                        && started_at.elapsed() > IDLE_THRESHOLD
                    {
                        transitions.push((*id, TerminalStatus::Done {
                            finished_at: Instant::now(),
                        }));
                    }
                }
            }
        }

        for (id, new_status) in transitions {
            let old_label = self.statuses.get(&id).map(|s| s.label().to_string()).unwrap_or_default();
            let new_label = new_status.label().to_string();
            self.statuses.insert(id, new_status);
            if old_label != new_label {
                self.emit(BusEvent::StatusChanged {
                    terminal_id: id,
                    old_status: old_label,
                    new_status: new_label,
                });
            }
        }
    }

    // -----------------------------------------------------------------------
    // Group Management
    // -----------------------------------------------------------------------

    /// Create a new group in orchestrated mode.
    ///
    /// The creating terminal becomes the orchestrator.
    pub fn create_orchestrated_group(
        &mut self,
        name: &str,
        orchestrator: Uuid,
    ) -> Result<Uuid, BusError> {
        if !self.terminals.contains_key(&orchestrator) {
            return Err(BusError::TerminalNotFound(orchestrator));
        }

        // Check if terminal is already in a group
        if self.terminal_to_group.contains_key(&orchestrator) {
            return Err(BusError::AlreadyInGroup(orchestrator));
        }

        // Check for duplicate group name
        if self.groups.values().any(|g| g.name == name) {
            return Err(BusError::GroupNameTaken(name.to_string()));
        }

        let group = TerminalGroup::new_orchestrated(name, orchestrator);
        let group_id = group.id;

        self.terminal_to_group.insert(orchestrator, group_id);
        self.groups.insert(group_id, group);

        self.emit(BusEvent::GroupCreated {
            group_id,
            name: name.to_string(),
            mode: "orchestrated".to_string(),
        });

        self.emit(BusEvent::GroupMemberJoined {
            group_id,
            terminal_id: orchestrator,
            role: "orchestrator".to_string(),
        });

        Ok(group_id)
    }

    /// Create a new group in peer mode.
    pub fn create_peer_group(
        &mut self,
        name: &str,
        creator: Uuid,
    ) -> Result<Uuid, BusError> {
        if !self.terminals.contains_key(&creator) {
            return Err(BusError::TerminalNotFound(creator));
        }

        if self.terminal_to_group.contains_key(&creator) {
            return Err(BusError::AlreadyInGroup(creator));
        }

        if self.groups.values().any(|g| g.name == name) {
            return Err(BusError::GroupNameTaken(name.to_string()));
        }

        let group = TerminalGroup::new_peer(name, creator);
        let group_id = group.id;

        self.terminal_to_group.insert(creator, group_id);
        self.groups.insert(group_id, group);

        self.emit(BusEvent::GroupCreated {
            group_id,
            name: name.to_string(),
            mode: "peer".to_string(),
        });

        self.emit(BusEvent::GroupMemberJoined {
            group_id,
            terminal_id: creator,
            role: "peer".to_string(),
        });

        Ok(group_id)
    }

    /// Join an existing group.
    ///
    /// In orchestrated mode, joining terminals become workers.
    /// In peer mode, joining terminals become peers.
    pub fn join_group(
        &mut self,
        terminal_id: Uuid,
        group_id: Uuid,
    ) -> Result<(), BusError> {
        if !self.terminals.contains_key(&terminal_id) {
            return Err(BusError::TerminalNotFound(terminal_id));
        }

        if self.terminal_to_group.contains_key(&terminal_id) {
            return Err(BusError::AlreadyInGroup(terminal_id));
        }

        let group = self
            .groups
            .get_mut(&group_id)
            .ok_or(BusError::GroupNotFound(group_id))?;

        let role = match &group.mode {
            GroupMode::Orchestrated { .. } => "worker",
            GroupMode::Peer => "peer",
        };

        group.add_member(terminal_id);
        self.terminal_to_group.insert(terminal_id, group_id);

        self.emit(BusEvent::GroupMemberJoined {
            group_id,
            terminal_id,
            role: role.to_string(),
        });

        Ok(())
    }

    /// Join a group by name (convenience wrapper).
    pub fn join_group_by_name(
        &mut self,
        terminal_id: Uuid,
        group_name: &str,
    ) -> Result<(), BusError> {
        let group_id = self
            .groups
            .values()
            .find(|g| g.name == group_name)
            .map(|g| g.id)
            .ok_or_else(|| BusError::GroupNotFound(Uuid::nil()))?;

        self.join_group(terminal_id, group_id)
    }

    /// Leave a group.
    ///
    /// If the orchestrator leaves, the group is dissolved.
    /// If the last member leaves, the group is dissolved.
    pub fn leave_group(&mut self, terminal_id: Uuid) -> Result<(), BusError> {
        let group_id = self
            .terminal_to_group
            .remove(&terminal_id)
            .ok_or(BusError::NotInGroup(terminal_id))?;

        self.remove_from_group_inner(terminal_id, group_id);
        Ok(())
    }

    fn remove_from_group_inner(&mut self, terminal_id: Uuid, group_id: Uuid) {
        let should_dissolve;

        if let Some(group) = self.groups.get_mut(&group_id) {
            group.remove_member(terminal_id);

            self.emit(BusEvent::GroupMemberLeft {
                group_id,
                terminal_id,
            });

            // Dissolve if empty or if the orchestrator left
            should_dissolve = group.is_empty() || group.is_orchestrator(terminal_id);
        } else {
            return;
        }

        if should_dissolve {
            self.dissolve_group(group_id);
        }
    }

    /// Dissolve a group, removing all members.
    pub fn dissolve_group(&mut self, group_id: Uuid) {
        if let Some(group) = self.groups.remove(&group_id) {
            // Remove all member mappings
            for member in &group.members {
                self.terminal_to_group.remove(member);
            }

            // Clean up group-scoped context
            let prefix = group.context_prefix.clone();
            self.context.retain(|k, _| !k.starts_with(&prefix));

            self.emit(BusEvent::GroupDissolved {
                group_id,
                name: group.name,
            });
        }
    }

    /// List all groups.
    pub fn list_groups(&self) -> Vec<GroupInfo> {
        self.groups
            .values()
            .map(|g| self.build_group_info(g))
            .collect()
    }

    /// Get info for a specific group.
    pub fn get_group(&self, group_id: Uuid) -> Option<GroupInfo> {
        self.groups.get(&group_id).map(|g| self.build_group_info(g))
    }

    /// Get info for a group by name.
    pub fn get_group_by_name(&self, name: &str) -> Option<GroupInfo> {
        self.groups
            .values()
            .find(|g| g.name == name)
            .map(|g| self.build_group_info(g))
    }

    fn build_group_info(&self, group: &TerminalGroup) -> GroupInfo {
        let members: Vec<GroupMemberInfo> = group
            .members
            .iter()
            .filter_map(|id| {
                let handle = self.terminals.get(id)?;
                let title = handle.title.lock().ok()?.clone();
                let role = group.role_of(*id)?;
                let status = self.statuses.get(id).cloned().unwrap_or_default();
                let alive = handle.alive.load(Ordering::Relaxed);
                Some(GroupMemberInfo {
                    terminal_id: *id,
                    title,
                    role,
                    status,
                    alive,
                })
            })
            .collect();

        let orchestrator_id = match &group.mode {
            GroupMode::Orchestrated { orchestrator } => Some(*orchestrator),
            GroupMode::Peer => None,
        };

        GroupInfo {
            id: group.id,
            name: group.name.clone(),
            mode: match &group.mode {
                GroupMode::Orchestrated { .. } => "orchestrated".to_string(),
                GroupMode::Peer => "peer".to_string(),
            },
            orchestrator_id,
            member_count: group.member_count(),
            members,
        }
    }

    // -----------------------------------------------------------------------
    // Broadcast & Messaging
    // -----------------------------------------------------------------------

    /// Send a command to all workers in a group (orchestrator only).
    ///
    /// The command is injected into each worker's PTY sequentially.
    pub fn broadcast_command(
        &mut self,
        group_id: Uuid,
        command: &str,
        source: Uuid,
    ) -> Result<Vec<Uuid>, BusError> {
        let group = self
            .groups
            .get(&group_id)
            .ok_or(BusError::GroupNotFound(group_id))?;

        // In orchestrated mode, only the orchestrator can broadcast
        if let GroupMode::Orchestrated { orchestrator } = &group.mode {
            if *orchestrator != source {
                return Err(BusError::PermissionDenied(
                    "only orchestrator can broadcast".into(),
                ));
            }
        }

        // Collect targets (all members except the source)
        let targets: Vec<Uuid> = group
            .members
            .iter()
            .filter(|&&id| id != source)
            .copied()
            .collect();

        // Inject command into each target
        for &target in &targets {
            // We call send_command which handles the \r appending
            let mut bytes = command.as_bytes().to_vec();
            bytes.push(b'\r');
            // Direct write, bypassing permission check (already validated above)
            if let Some(handle) = self.terminals.get(&target) {
                if handle.alive.load(Ordering::Relaxed) {
                    if let Ok(mut writer) = handle.writer.lock() {
                        let _ = writer.write_all(&bytes);
                        let _ = writer.flush();
                    }
                    self.statuses.insert(
                        target,
                        TerminalStatus::Running {
                            command: Some(command.to_string()),
                            started_at: Instant::now(),
                        },
                    );
                }
            }
        }

        self.emit(BusEvent::BroadcastSent {
            from: source,
            group_id,
            payload: command.to_string(),
        });

        Ok(targets)
    }

    /// Send a direct message between terminals (stored in context).
    ///
    /// Messages are stored as context entries with a special key format:
    /// `_msg:{from}:{to}:{timestamp}`
    pub fn send_message(
        &mut self,
        from: Uuid,
        to: Uuid,
        payload: &str,
    ) -> Result<(), BusError> {
        if !self.terminals.contains_key(&from) {
            return Err(BusError::TerminalNotFound(from));
        }
        if !self.terminals.contains_key(&to) {
            return Err(BusError::TerminalNotFound(to));
        }

        let key = format!(
            "_msg:{}:{}:{}",
            from,
            to,
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );

        self.context.insert(
            key,
            ContextEntry {
                value: payload.to_string(),
                source: from,
                updated_at: SystemTime::now(),
                ttl: Some(Duration::from_secs(3600)), // Messages expire after 1 hour
            },
        );

        self.emit(BusEvent::MessageSent {
            from,
            to,
            payload: payload.to_string(),
        });

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Shared Context
    // -----------------------------------------------------------------------

    /// Set a context value.
    ///
    /// Keys can be:
    /// - Global: `"key_name"` — visible to all terminals
    /// - Group-scoped: `"group_name:key_name"` — only visible within the group
    pub fn context_set(
        &mut self,
        key: &str,
        value: &str,
        source: Uuid,
        ttl: Option<Duration>,
    ) -> Result<(), BusError> {
        if !self.terminals.contains_key(&source) {
            return Err(BusError::TerminalNotFound(source));
        }

        self.context.insert(
            key.to_string(),
            ContextEntry {
                value: value.to_string(),
                source,
                updated_at: SystemTime::now(),
                ttl,
            },
        );

        self.emit(BusEvent::ContextUpdated {
            key: key.to_string(),
            source,
        });

        Ok(())
    }

    /// Get a context value.
    ///
    /// Returns None if the key does not exist or has expired.
    pub fn context_get(&mut self, key: &str) -> Option<String> {
        if let Some(entry) = self.context.get(key) {
            if entry.is_expired() {
                self.context.remove(key);
                return None;
            }
            Some(entry.value.clone())
        } else {
            None
        }
    }

    /// Get a context entry with metadata.
    pub fn context_get_entry(&mut self, key: &str) -> Option<ContextEntry> {
        if let Some(entry) = self.context.get(key) {
            if entry.is_expired() {
                self.context.remove(key);
                return None;
            }
            Some(entry.clone())
        } else {
            None
        }
    }

    /// List all context keys (excluding expired and messages).
    pub fn context_list(&mut self) -> Vec<(String, ContextEntry)> {
        // Clean up expired entries first
        self.context.retain(|_, v| !v.is_expired());

        self.context
            .iter()
            .filter(|(k, _)| !k.starts_with("_msg:"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Delete a context entry.
    pub fn context_delete(&mut self, key: &str) -> bool {
        let existed = self.context.remove(key).is_some();
        if existed {
            self.emit(BusEvent::ContextDeleted {
                key: key.to_string(),
            });
        }
        existed
    }

    /// List messages for a specific terminal (received messages).
    pub fn list_messages(&mut self, terminal_id: Uuid) -> Vec<(Uuid, String, SystemTime)> {
        let prefix = format!("_msg:");
        let target_str = terminal_id.to_string();

        self.context.retain(|_, v| !v.is_expired());

        self.context
            .iter()
            .filter_map(|(k, v)| {
                if !k.starts_with(&prefix) {
                    return None;
                }
                // Parse key format: _msg:{from}:{to}:{timestamp}
                let parts: Vec<&str> = k.splitn(4, ':').collect();
                if parts.len() == 4 && parts[2] == target_str {
                    let from = Uuid::parse_str(parts[1]).ok()?;
                    Some((from, v.value.clone(), v.updated_at))
                } else {
                    None
                }
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // Event System
    // -----------------------------------------------------------------------

    /// Subscribe to bus events with an optional filter.
    ///
    /// Returns a receiver and a subscription ID (for unsubscribing).
    pub fn subscribe(
        &mut self,
        filter: EventFilter,
    ) -> (Uuid, mpsc::Receiver<BusEvent>) {
        let (tx, rx) = mpsc::channel();
        let sub_id = Uuid::new_v4();
        self.subscribers.push((sub_id, filter, tx));
        (sub_id, rx)
    }

    /// Unsubscribe from bus events.
    pub fn unsubscribe(&mut self, subscription_id: Uuid) {
        self.subscribers.retain(|(id, _, _)| *id != subscription_id);
    }

    /// Emit an event to all matching subscribers.
    fn emit(&self, event: BusEvent) {
        for (_, filter, tx) in &self.subscribers {
            if filter.matches(&event) {
                // Non-blocking send. If the channel is full, drop the event
                // for this subscriber (they'll catch up on the next one).
                let _ = tx.send(event.clone());
            }
        }
    }

    /// Remove dead subscribers (disconnected channels).
    pub fn cleanup_subscribers(&mut self) {
        self.subscribers.retain(|(_, _, tx)| {
            // Try sending a dummy — if the receiver is dropped, remove
            // Actually, we can't do this without a real event.
            // Instead, we'll let send() errors accumulate and clean up
            // subscribers that have been failing.
            // For now, rely on explicit unsubscribe.
            true
        });
    }
}

// ---------------------------------------------------------------------------
// Bus Errors
// ---------------------------------------------------------------------------

/// Errors returned by bus operations.
#[derive(Debug)]
pub enum BusError {
    TerminalNotFound(Uuid),
    TerminalDead(Uuid),
    GroupNotFound(Uuid),
    GroupNameTaken(String),
    AlreadyInGroup(Uuid),
    NotInGroup(Uuid),
    PermissionDenied(String),
    LockFailed(&'static str),
    WriteFailed(String),
    Timeout,
}

impl std::fmt::Display for BusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TerminalNotFound(id) => write!(f, "terminal not found: {}", id),
            Self::TerminalDead(id) => write!(f, "terminal is dead: {}", id),
            Self::GroupNotFound(id) => write!(f, "group not found: {}", id),
            Self::GroupNameTaken(name) => write!(f, "group name already taken: {}", name),
            Self::AlreadyInGroup(id) => write!(f, "terminal already in a group: {}", id),
            Self::NotInGroup(id) => write!(f, "terminal not in a group: {}", id),
            Self::PermissionDenied(msg) => write!(f, "permission denied: {}", msg),
            Self::LockFailed(what) => write!(f, "failed to lock: {}", what),
            Self::WriteFailed(msg) => write!(f, "write failed: {}", msg),
            Self::Timeout => write!(f, "operation timed out"),
        }
    }
}

impl std::error::Error for BusError {}
```

---

## 6. Terminal Groups

### 6.1 Group Lifecycle

```
                  create_orchestrated_group()
                  or create_peer_group()
                           │
                           ▼
                    ┌──────────────┐
                    │   Created    │
                    │ (1 member)   │
                    └──────┬───────┘
                           │
              join_group() │ (other terminals join)
                           │
                    ┌──────▼───────┐
                    │   Active     │
                    │ (N members)  │
                    └──────┬───────┘
                           │
         leave_group()     │   orchestrator leaves
         or terminal dies  │   or dissolve_group()
                           │
                    ┌──────▼───────┐
                    │  Dissolved   │
                    │ (cleaned up) │
                    └──────────────┘
```

### 6.2 Orchestrated Group Workflow

```
# Step 1: Orchestrator creates a group
$ void-ctl group create build --mode orchestrated
Created group "build" (id: abc-123) in orchestrated mode
You are the orchestrator.

# Step 2: Workers join the group
# (from terminal B)
$ void-ctl group join build
Joined group "build" as worker

# (from terminal C)
$ void-ctl group join build
Joined group "build" as worker

# Step 3: Orchestrator sends commands to workers
$ void-ctl send --group build "cargo test --lib"
Sent to 2 workers: cargo test --lib

# Step 4: Orchestrator waits for all workers to finish
$ void-ctl wait-idle --group build --timeout 120
All terminals in group "build" are idle.

# Step 5: Orchestrator reads output from each worker
$ void-ctl read --group build --lines 20
--- Terminal B (worker) ---
running 42 tests
test result: ok. 42 passed; 0 failed

--- Terminal C (worker) ---
running 42 tests
test result: ok. 42 passed; 0 failed

# Step 6: Orchestrator stores results
$ void-ctl context set build:test_results "all tests passed"
```

### 6.3 Peer Group Workflow

```
# Step 1: Any terminal creates a peer group
$ void-ctl group create research --mode peer
Created group "research" (id: def-456) in peer mode

# Step 2: Others join
$ void-ctl group join research

# Step 3: Any peer can share context
$ void-ctl context set research:finding_1 "The auth middleware stores tokens in plaintext"
$ void-ctl context set research:finding_2 "Rate limiting is at 100 req/s per IP"

# Step 4: Any peer can read context
$ void-ctl context list --prefix research:
research:finding_1 = "The auth middleware stores tokens in plaintext"
research:finding_2 = "Rate limiting is at 100 req/s per IP"

# Step 5: Peers can send direct messages
$ void-ctl message send <peer-B-id> "Check the rate limiter in src/middleware/rate.rs"
```

### 6.4 Group Commands Reference

| Command | Orchestrated | Peer | Description |
|---------|-------------|------|-------------|
| `group create <name> --mode <mode>` | Creator = orchestrator | Creator = first peer | Create a new group |
| `group join <name>` | Joiner = worker | Joiner = peer | Join existing group |
| `group leave` | Leaves group | Leaves group | Leave current group |
| `group dissolve` | Orchestrator only | Any member | Dissolve the group |
| `group list` | Any | Any | List all groups |
| `group info <name>` | Any | Any | Show group details |
| `send --group <name> <cmd>` | Orchestrator only | Any peer | Broadcast command |
| `read --group <name>` | Any | Any | Read all members' output |
| `wait-idle --group <name>` | Any | Any | Wait for all members idle |

### 6.5 Auto-Grouping

Terminals spawned by an orchestrator (via `void-ctl spawn`) are automatically added
to the orchestrator's group as workers:

```
$ void-ctl spawn --count 3 --cwd /project
Spawned 3 terminals, added to group "build" as workers:
  - term-1 (e5f6a7b8...)
  - term-2 (c9d0e1f2...)
  - term-3 (a1b2c3d4...)
```

---

## 7. Communication Protocol — APC Escape Sequences

### 7.1 Transport

Communication flows through the **existing PTY pipe**. No socket, no extra port, no auth.

Child processes (like `void-ctl`) write APC (Application Program Command) escape
sequences to stdout. These travel through the PTY to Void's reader thread, which
intercepts them before the VTE parser sees them. Void processes the command via the
bus and writes the response APC back to the terminal's PTY stdin.

This is the same pattern terminals use for cursor position queries (`\e[6n` → `\e[row;colR`)
and OSC 52 clipboard operations. It's a standard terminal communication mechanism.

### 7.2 APC Sequence Format

**Request** (child process → Void, via PTY stdout):
```
\x1b_VOID;<request-id>;<method>;<json-params>\x1b\\
```

**Response** (Void → child process, via PTY stdin):
```
\x1b_VOID-R;<request-id>;<json-result>\x1b\\
```

Where:
- `\x1b_` = ESC _ = APC start (standard ECMA-48)
- `\x1b\\` = ESC \ = ST = String Terminator (standard ECMA-48)
- `VOID` = marker to distinguish from other APC sequences
- `<request-id>` = short random ID to match responses (e.g., "r1", "r2")
- `<method>` = bus method name (e.g., "list_terminals", "inject", "read_output")
- `<json-params>` = JSON-encoded parameters
- `<json-result>` = JSON-encoded result or error

### 7.3 Example Exchange

```
void-ctl writes to stdout:
  \x1b_VOID;r1;list_terminals;{}\x1b\\

Void reader thread intercepts, calls bus.list_terminals(), writes to PTY stdin:
  \x1b_VOID-R;r1;{"terminals":[{"id":"abc","title":"zsh","alive":true}]}\x1b\\

void-ctl reads from stdin, parses response, prints formatted output.
```

### 7.4 Security Model

No auth token needed. The PTY pipe IS the authentication:
- Only child processes of the terminal's shell can write to its PTY stdout
- Only the terminal's PTY master (owned by Void) can write to the PTY stdin
- A process in terminal A cannot write to terminal B's PTY pipe

This is strictly more secure than a TCP socket + token approach, because there is
no network surface at all.

### 7.5 Error Response Format

```json
{"error":{"code":-32000,"message":"terminal not found: abc-123"}}
```

Error codes remain the same as before (see section 7.8).

### 7.6 Methods

#### Terminal Methods

##### `list_terminals`

List all registered terminals.

APC request:
```
\x1b_VOID;r1;list_terminals;{}\x1b\\
```

APC response:
```json
{"terminals":[
    {"id":"a1b2c3d4-...","title":"zsh","alive":true,"workspace_id":"w1...",
     "group_id":"g1...","group_name":"build","role":"orchestrator",
     "status":"idle","last_output_ms":1523,"last_input_ms":4201},
    {"id":"e5f6a7b8-...","title":"zsh","alive":true,"workspace_id":"w1...",
     "group_id":"g1...","group_name":"build","role":"worker",
     "status":"running","last_output_ms":42,"last_input_ms":5000}
]}
```

##### `get_terminal`

Get info for a specific terminal.

```json
{"id":2,"method":"get_terminal","params":{"id":"a1b2c3d4-..."}}
```

##### `inject`

Inject raw bytes into a terminal's PTY. The `command` field is a string; `\r` is
appended automatically unless `raw` is true.

```json
{
    "id":3,
    "method": "inject",
    "params": {
        "target": "e5f6a7b8-...",
        "command": "cargo test",
        "raw": false
    }
}
```

With `raw: true`, the command string is sent as-is (for control characters):
```json
{
    "id":4,
    "method": "inject",
    "params": {
        "target": "e5f6a7b8-...",
        "command": "\u0003",
        "raw": true
    }
}
```

##### `read_output`

Read terminal output.

```json
{
    "id":5,
    "method": "read_output",
    "params": {
        "target": "e5f6a7b8-...",
        "lines": 50,
        "source": "scrollback"
    }
}
```

`source` can be:
- `"screen"` — current visible screen only
- `"scrollback"` — last N lines including scrollback (default)

Response:
```json
{
    "id":5,
    "result": {
        "lines": [
            "$ cargo test",
            "   Compiling void v0.1.0",
            "    Finished test [unoptimized] target(s) in 2.34s",
            "     Running unittests src/main.rs",
            "",
            "running 42 tests",
            "test result: ok. 42 passed; 0 failed; 0 ignored"
        ],
        "total_lines": 7
    }
}
```

##### `wait_idle`

Block until a terminal becomes idle (no output for N seconds).

```json
{
    "id":6,
    "method": "wait_idle",
    "params": {
        "target": "e5f6a7b8-...",
        "timeout_secs": 120,
        "quiet_secs": 2
    }
}
```

Response (success):
```json
{"id":6,"result":{"idle":true,"elapsed_secs":15.3}}
```

Response (timeout):
```json
{"id":6,"result":{"idle":false,"elapsed_secs":120.0}}
```

##### `set_status`

Manually set a terminal's status.

```json
{
    "id":7,
    "method": "set_status",
    "params": {
        "target": "e5f6a7b8-...",
        "status": "error",
        "message": "tests failed with exit code 1"
    }
}
```

##### `spawn`

Spawn a new terminal and optionally add it to a group.

```json
{
    "id":8,
    "method": "spawn",
    "params": {
        "cwd": "/home/user/project",
        "title": "test-runner",
        "group": "build",
        "count": 1
    }
}
```

Response:
```json
{
    "id":8,
    "result": {
        "terminals": [
            {"id": "new-uuid-...", "title": "test-runner"}
        ]
    }
}
```

##### `close`

Close a terminal (kills the PTY process).

```json
{"id":9,"method":"close","params":{"target":"e5f6a7b8-..."}}
```

#### Group Methods

##### `group_create`

```json
{
    "id":10,
    "method": "group_create",
    "params": {
        "name": "build",
        "mode": "orchestrated"
    }
}
```

Response:
```json
{"id":10,"result":{"group_id":"g1...","name":"build","mode":"orchestrated"}}
```

##### `group_join`

```json
{"id":11,"method":"group_join","params":{"group":"build"}}
```

##### `group_leave`

```json
{"id":12,"method":"group_leave","params":{}}
```

##### `group_dissolve`

```json
{"id":13,"method":"group_dissolve","params":{"group":"build"}}
```

##### `group_list`

```json
{"id":14,"method":"group_list","params":{}}
```

Response:
```json
{
    "id":14,
    "result": {
        "groups": [
            {
                "id": "g1...",
                "name": "build",
                "mode": "orchestrated",
                "orchestrator_id": "a1b2...",
                "member_count": 3,
                "members": [
                    {"id": "a1b2...", "title": "claude", "role": "orchestrator", "status": "idle"},
                    {"id": "e5f6...", "title": "zsh", "role": "worker", "status": "running"},
                    {"id": "c9d0...", "title": "zsh", "role": "worker", "status": "done"}
                ]
            }
        ]
    }
}
```

##### `group_broadcast`

Send a command to all workers/peers in a group.

```json
{
    "id":15,
    "method": "group_broadcast",
    "params": {
        "group": "build",
        "command": "cargo test --lib"
    }
}
```

##### `group_wait_idle`

Wait for all members of a group to become idle.

```json
{
    "id":16,
    "method": "group_wait_idle",
    "params": {
        "group": "build",
        "timeout_secs": 120,
        "quiet_secs": 2
    }
}
```

##### `group_read`

Read output from all members of a group.

```json
{
    "id":17,
    "method": "group_read",
    "params": {
        "group": "build",
        "lines": 20
    }
}
```

Response:
```json
{
    "id":17,
    "result": {
        "outputs": {
            "e5f6a7b8-...": {
                "title": "test-runner-1",
                "role": "worker",
                "lines": ["running 42 tests", "test result: ok. 42 passed"]
            },
            "c9d0e1f2-...": {
                "title": "test-runner-2",
                "role": "worker",
                "lines": ["running 18 tests", "test result: ok. 18 passed"]
            }
        }
    }
}
```

#### Context Methods

##### `context_set`

```json
{
    "id":20,
    "method": "context_set",
    "params": {
        "key": "test_results",
        "value": "all 60 tests passed",
        "ttl_secs": 3600
    }
}
```

##### `context_get`

```json
{"id":21,"method":"context_get","params":{"key":"test_results"}}
```

Response:
```json
{
    "id":21,
    "result": {
        "key": "test_results",
        "value": "all 60 tests passed",
        "source": "a1b2c3d4-...",
        "updated_at": "2026-03-24T12:34:56Z"
    }
}
```

##### `context_list`

```json
{"id":22,"method":"context_list","params":{"prefix":"build:"}}
```

##### `context_delete`

```json
{"id":23,"method":"context_delete","params":{"key":"test_results"}}
```

#### Message Methods

##### `message_send`

```json
{
    "id":30,
    "method": "message_send",
    "params": {
        "to": "e5f6a7b8-...",
        "payload": "Check src/auth.rs line 42"
    }
}
```

##### `message_list`

```json
{"id":31,"method":"message_list","params":{}}
```

#### Subscription Methods

##### `subscribe`

```json
{
    "id":40,
    "method": "subscribe",
    "params": {
        "events": ["status.changed", "output.changed"],
        "terminals": ["e5f6a7b8-..."],
        "group": "build"
    }
}
```

After subscribing, the server pushes notifications:
```json
{"jsonrpc":"2.0","method":"event","params":{"type":"status.changed","terminal_id":"e5f6...","old_status":"running","new_status":"done"}}
```

##### `unsubscribe`

```json
{"id":41,"method":"unsubscribe","params":{"subscription_id":"sub-uuid..."}}
```

### 7.8 Error Codes

| Code   | Meaning                              |
|--------|--------------------------------------|
| -32700 | Parse error (malformed JSON)         |
| -32600 | Invalid request (missing fields)     |
| -32601 | Method not found                     |
| -32602 | Invalid params                       |
| -32000 | Terminal not found                   |
| -32001 | Terminal is dead                     |
| -32002 | Group not found                      |
| -32003 | Group name taken                     |
| -32004 | Already in a group                   |
| -32005 | Not in a group                       |
| -32006 | Permission denied                    |
| -32007 | Lock failed (internal error)         |
| -32008 | Write failed                         |
| -32009 | Timeout                              |

---

## 8. APC Interception Layer

### 8.1 Overview

APC interception lives inside each terminal's existing reader thread. Before PTY output
reaches the VTE parser, the reader scans for `\x1b_VOID;` markers (APC escape sequences).
Matching bytes are extracted, dispatched to the terminal bus, and the response is written
back through the PTY as another APC sequence. Non-matching bytes pass through to the VTE
parser unchanged.

No socket. No auth token. No extra threads. The PTY pipe that already exists carries
orchestration commands alongside normal terminal output.

### 8.2 Reader Thread Modification

```rust
// In src/terminal/pty.rs — modified reader thread

use crate::bus::TerminalBus;
use std::sync::{Arc, Mutex};

/// Modified reader thread that intercepts APC sequences before VTE parsing.
fn start_reader_thread(
    mut reader: Box<dyn std::io::Read + Send>,
    term: Arc<Mutex<alacritty_terminal::Term<EventListener>>>,
    bus: Arc<Mutex<TerminalBus>>,
    terminal_id: Uuid,
    alive: Arc<AtomicBool>,
    last_output_at: Arc<Mutex<std::time::Instant>>,
    ctx: egui::Context,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buf = [0u8; 8192];
        let mut processor = alacritty_terminal::vte::ansi::Processor::new();
        let mut apc_accum = Vec::new(); // Accumulator for partial APC sequences

        loop {
            match reader.read(&mut buf) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    let data = &buf[..n];

                    // Extract APC commands, get remaining bytes for VTE
                    let (passthrough, commands) =
                        extract_void_commands(data, &mut apc_accum);

                    // Handle any extracted commands
                    for cmd_payload in commands {
                        let response = handle_bus_command(
                            &cmd_payload,
                            terminal_id,
                            &bus,
                        );
                        // Response is written back as APC via the PTY master side
                        // The bus handler queues the response for the next read
                    }

                    // Pass remaining bytes to VTE parser
                    if !passthrough.is_empty() {
                        let mut term = term.lock().unwrap();
                        for byte in &passthrough {
                            processor.advance(&mut *term, *byte);
                        }
                    }

                    // Update last output timestamp
                    if let Ok(mut t) = last_output_at.lock() {
                        *t = std::time::Instant::now();
                    }

                    ctx.request_repaint();
                }
                Err(_) => break,
            }
        }

        alive.store(false, std::sync::atomic::Ordering::Relaxed);
    })
}
```

### 8.3 APC Extraction Function

```rust
// In src/terminal/pty.rs

const APC_START: &[u8] = b"\x1b_VOID;";
const APC_END: u8 = 0x9C;       // ST (String Terminator)
const APC_END_ALT: &[u8] = b"\x1b\\"; // ESC \ (alternative ST)

/// Scan a byte buffer for `\x1b_VOID;...ST` sequences.
///
/// Returns (passthrough_bytes, extracted_command_payloads).
/// Handles partial sequences across read boundaries using the accumulator.
fn extract_void_commands(
    data: &[u8],
    accum: &mut Vec<u8>,
) -> (Vec<u8>, Vec<String>) {
    let mut passthrough = Vec::with_capacity(data.len());
    let mut commands = Vec::new();
    let mut i = 0;

    while i < data.len() {
        // If we're accumulating a partial APC sequence
        if !accum.is_empty() {
            // Look for ST (0x9C) or ESC \ to end the sequence
            if data[i] == APC_END {
                // Complete — extract payload (skip the "VOID;" prefix already consumed)
                if let Ok(payload) = std::str::from_utf8(accum) {
                    commands.push(payload.to_string());
                }
                accum.clear();
                i += 1;
                continue;
            }
            if data[i] == 0x1b && i + 1 < data.len() && data[i + 1] == b'\\' {
                // ESC \ terminator
                if let Ok(payload) = std::str::from_utf8(accum) {
                    commands.push(payload.to_string());
                }
                accum.clear();
                i += 2;
                continue;
            }
            accum.push(data[i]);
            i += 1;
            continue;
        }

        // Check for APC_START at current position
        if data[i] == 0x1b
            && i + APC_START.len() <= data.len()
            && &data[i..i + APC_START.len()] == APC_START
        {
            // Found start marker — begin accumulating (skip the marker itself)
            i += APC_START.len();
            continue;
        }

        // Check for partial APC_START at end of buffer
        if data[i] == 0x1b && i + APC_START.len() > data.len() {
            // Could be a partial match — check what we have
            let remaining = &data[i..];
            if APC_START.starts_with(remaining) {
                // Partial match at buffer boundary — save for next read
                accum.extend_from_slice(remaining);
                break;
            }
        }

        // Normal byte — pass through to VTE
        passthrough.push(data[i]);
        i += 1;
    }

    (passthrough, commands)
}
```

### 8.4 Command Handler

```rust
// In src/terminal/pty.rs

use serde_json::{json, Value};

/// Parse an APC payload, dispatch to the bus, return the JSON response.
///
/// Payload format: `{"jsonrpc":"2.0","id":1,"method":"list_terminals","params":{}}`
/// Response format: `\x1b_VOID;{"jsonrpc":"2.0","id":1,"result":{...}}\x1b\\`
fn handle_bus_command(
    payload: &str,
    caller_terminal: Uuid,
    bus: &Arc<Mutex<TerminalBus>>,
) -> Vec<u8> {
    let request: Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => {
            let err = json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": {"code": -32700, "message": "parse error"}
            });
            return format_apc_response(&err);
        }
    };

    let id = request["id"].clone();
    let method = request["method"].as_str().unwrap_or("");
    let params = &request["params"];

    let response = dispatch_bus_method(
        method,
        params,
        Some(caller_terminal),
        bus,
    );

    let response_json = match response {
        Ok(result) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }),
        Err((code, message)) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": code, "message": message},
        }),
    };

    format_apc_response(&response_json)
}

/// Wrap a JSON value in APC framing: ESC _ VOID; ... ESC \
fn format_apc_response(json: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"\x1b_VOID;");
    out.extend_from_slice(json.to_string().as_bytes());
    out.extend_from_slice(b"\x1b\\");
    out
}

/// Route a JSON-RPC method to the appropriate bus operation.
/// Same dispatch logic as before, but called inline from the reader thread.
fn dispatch_bus_method(
    method: &str,
    params: &Value,
    caller_terminal: Option<Uuid>,
    bus: &Arc<Mutex<TerminalBus>>,
) -> Result<Value, (i32, String)> {
    // Same match block as section 8 previously defined —
    // list_terminals, get_terminal, inject, read_output, wait_idle,
    // set_status, group_*, context_*, message_* — all unchanged.
    // The dispatch logic is identical; only the transport changed.
    //
    // See section 5 (Terminal Bus) for the full method list.
    todo!("dispatch logic — same as bus API")
}
```

### 8.5 Environment Variables

When spawning a new terminal, `PtyHandle::spawn()` sets one orchestration env var:

```rust
// In terminal/pty.rs — inside PtyHandle::spawn()

cmd.env("VOID_TERMINAL_ID", &panel_id);     // e.g., "550e8400-e29b-..."
cmd.env("VOID_WORKSPACE_ID", &workspace_id);
```

No `VOID_SOCKET` or `VOID_TOKEN` needed. The PTY pipe is the communication channel and
the OS process hierarchy is the authentication.

---

## 9. void-ctl CLI

### 9.1 Implementation

```rust
// src/bin/void-ctl.rs

use std::env;
use std::io::Write;
use std::process;

use serde_json::{json, Value};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let terminal_id = env::var("VOID_TERMINAL_ID").unwrap_or_else(|_| {
        eprintln!("error: VOID_TERMINAL_ID not set. Are you inside a Void terminal?");
        process::exit(1);
    });

    let mut client = VoidClient::new(&terminal_id);

    let subcommand = args[1].as_str();
    let sub_args = &args[2..];

    match subcommand {
        "list" => cmd_list(&mut client, sub_args),
        "send" => cmd_send(&mut client, sub_args),
        "read" => cmd_read(&mut client, sub_args),
        "wait-idle" => cmd_wait_idle(&mut client, sub_args),
        "status" => cmd_status(&mut client, sub_args),
        "group" => cmd_group(&mut client, sub_args),
        "context" => cmd_context(&mut client, sub_args),
        "message" => cmd_message(&mut client, sub_args),
        "spawn" => cmd_spawn(&mut client, sub_args),
        "close" => cmd_close(&mut client, sub_args),
        "help" | "--help" | "-h" => print_usage(),
        _ => {
            eprintln!("unknown command: {}", subcommand);
            print_usage();
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

struct VoidClient {
    terminal_id: String,
    next_id: u64,
}

impl VoidClient {
    fn new(terminal_id: &str) -> Self {
        Self {
            terminal_id: terminal_id.to_string(),
            next_id: 1,
        }
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;

        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        // Write APC sequence to stdout — the PTY master intercepts it
        let apc = format!("\x1b_VOID;{}\x1b\\", request);
        std::io::stdout()
            .write_all(apc.as_bytes())
            .map_err(|e| format!("write: {}", e))?;
        std::io::stdout().flush().map_err(|e| format!("flush: {}", e))?;

        // Read APC response from stdin
        // The PTY master injects the response as an APC sequence
        let response_str = read_apc_response()
            .map_err(|e| format!("read response: {}", e))?;

        let resp: Value = serde_json::from_str(&response_str)
            .map_err(|e| format!("parse: {}", e))?;

        if let Some(error) = resp.get("error") {
            Err(format!(
                "{} (code {})",
                error["message"].as_str().unwrap_or("unknown"),
                error["code"].as_i64().unwrap_or(0)
            ))
        } else {
            Ok(resp["result"].clone())
        }
    }
}

/// Read an APC response from stdin.
/// Scans for \x1b_VOID; prefix, reads until ST (\x1b\\).
fn read_apc_response() -> Result<String, String> {
    use std::io::Read;
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut buf = [0u8; 1];
    let mut state = 0; // 0=waiting for ESC, 1=got ESC, 2=got _, etc.
    let mut marker_pos = 0;
    let marker = b"\x1b_VOID;";
    let mut payload = Vec::new();

    // Scan for APC start marker
    loop {
        handle.read_exact(&mut buf).map_err(|e| e.to_string())?;
        if buf[0] == marker[marker_pos] {
            marker_pos += 1;
            if marker_pos == marker.len() {
                break; // Found full marker
            }
        } else {
            marker_pos = 0;
        }
    }

    // Read payload until ESC \ (ST)
    let mut prev_was_esc = false;
    loop {
        handle.read_exact(&mut buf).map_err(|e| e.to_string())?;
        if prev_was_esc && buf[0] == b'\\' {
            payload.pop(); // Remove the ESC we already pushed
            break;
        }
        prev_was_esc = buf[0] == 0x1b;
        if buf[0] == 0x9C {
            break; // Single-byte ST
        }
        payload.push(buf[0]);
    }

    String::from_utf8(payload).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn cmd_list(client: &mut VoidClient, _args: &[String]) {
    let result = client.call("list_terminals", json!({})).unwrap_or_else(|e| {
        eprintln!("error: {}", e);
        process::exit(1);
    });

    let terminals = result["terminals"].as_array().unwrap_or(&vec![]);

    if terminals.is_empty() {
        println!("No terminals registered.");
        return;
    }

    // Header
    println!(
        "{:<38} {:<20} {:<8} {:<15} {:<12} {:<10}",
        "ID", "TITLE", "ALIVE", "GROUP", "ROLE", "STATUS"
    );
    println!("{}", "-".repeat(103));

    for t in terminals {
        println!(
            "{:<38} {:<20} {:<8} {:<15} {:<12} {:<10}",
            t["id"].as_str().unwrap_or("-"),
            truncate(t["title"].as_str().unwrap_or("-"), 20),
            if t["alive"].as_bool().unwrap_or(false) { "yes" } else { "no" },
            t["group_name"].as_str().unwrap_or("-"),
            t["role"].as_str().unwrap_or("Standalone"),
            t["status"].as_str().unwrap_or("-"),
        );
    }
}

fn cmd_send(client: &mut VoidClient, args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: void-ctl send <target-id|--group NAME> <command>");
        process::exit(1);
    }

    if args[0] == "--group" {
        if args.len() < 3 {
            eprintln!("usage: void-ctl send --group <name> <command>");
            process::exit(1);
        }
        let group = &args[1];
        let command = args[2..].join(" ");
        let result = client
            .call("group_broadcast", json!({"group": group, "command": command}))
            .unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                process::exit(1);
            });
        println!(
            "Sent to {} terminals.",
            result["sent_to"].as_u64().unwrap_or(0)
        );
    } else {
        if args.len() < 2 {
            eprintln!("usage: void-ctl send <target-id> <command>");
            process::exit(1);
        }
        let target = &args[0];
        let command = args[1..].join(" ");
        client
            .call("inject", json!({"target": target, "command": command}))
            .unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                process::exit(1);
            });
        println!("Sent.");
    }
}

fn cmd_read(client: &mut VoidClient, args: &[String]) {
    let mut target = None;
    let mut group = None;
    let mut lines: u64 = 50;
    let mut source = "scrollback";

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--group" => {
                i += 1;
                group = Some(args[i].clone());
            }
            "--lines" => {
                i += 1;
                lines = args[i].parse().unwrap_or(50);
            }
            "--screen" => {
                source = "screen";
            }
            _ => {
                target = Some(args[i].clone());
            }
        }
        i += 1;
    }

    if let Some(group_name) = group {
        let result = client
            .call("group_read", json!({"group": group_name, "lines": lines}))
            .unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                process::exit(1);
            });

        if let Some(outputs) = result["outputs"].as_object() {
            for (id, data) in outputs {
                let title = data["title"].as_str().unwrap_or("?");
                let role = data["role"].as_str().unwrap_or("?");
                println!("--- {} ({}) [{}] ---", title, &id[..8], role);
                if let Some(output_lines) = data["lines"].as_array() {
                    for line in output_lines {
                        println!("{}", line.as_str().unwrap_or(""));
                    }
                }
                println!();
            }
        }
    } else if let Some(target_id) = target {
        let result = client
            .call(
                "read_output",
                json!({"target": target_id, "lines": lines, "source": source}),
            )
            .unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                process::exit(1);
            });

        if let Some(output_lines) = result["lines"].as_array() {
            for line in output_lines {
                println!("{}", line.as_str().unwrap_or(""));
            }
        }
    } else {
        eprintln!("usage: void-ctl read <target-id|--group NAME> [--lines N] [--screen]");
        process::exit(1);
    }
}

fn cmd_wait_idle(client: &mut VoidClient, args: &[String]) {
    let mut target = None;
    let mut group = None;
    let mut timeout: u64 = 60;
    let mut quiet: u64 = 2;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--group" => {
                i += 1;
                group = Some(args[i].clone());
            }
            "--timeout" => {
                i += 1;
                timeout = args[i].parse().unwrap_or(60);
            }
            "--quiet" => {
                i += 1;
                quiet = args[i].parse().unwrap_or(2);
            }
            _ => {
                target = Some(args[i].clone());
            }
        }
        i += 1;
    }

    if let Some(group_name) = group {
        let result = client
            .call(
                "group_wait_idle",
                json!({"group": group_name, "timeout_secs": timeout, "quiet_secs": quiet}),
            )
            .unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                process::exit(1);
            });

        if result["idle"].as_bool().unwrap_or(false) {
            println!("All terminals idle.");
        } else {
            println!("Timeout reached. Some terminals still active.");
            process::exit(2);
        }
    } else if let Some(target_id) = target {
        let result = client
            .call(
                "wait_idle",
                json!({"target": target_id, "timeout_secs": timeout, "quiet_secs": quiet}),
            )
            .unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                process::exit(1);
            });

        if result["idle"].as_bool().unwrap_or(false) {
            println!("Terminal idle.");
        } else {
            println!("Timeout reached.");
            process::exit(2);
        }
    } else {
        eprintln!("usage: void-ctl wait-idle <target-id|--group NAME> [--timeout N] [--quiet N]");
        process::exit(1);
    }
}

fn cmd_status(client: &mut VoidClient, args: &[String]) {
    if args.len() < 2 {
        eprintln!("usage: void-ctl status <target-id> <idle|running|waiting|done|error> [message]");
        process::exit(1);
    }

    let target = &args[0];
    let status = &args[1];
    let message = if args.len() > 2 {
        args[2..].join(" ")
    } else {
        String::new()
    };

    client
        .call(
            "set_status",
            json!({"target": target, "status": status, "message": message}),
        )
        .unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            process::exit(1);
        });

    println!("Status updated.");
}

fn cmd_group(client: &mut VoidClient, args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: void-ctl group <create|join|leave|dissolve|list|info> [args...]");
        process::exit(1);
    }

    match args[0].as_str() {
        "create" => {
            if args.len() < 2 {
                eprintln!("usage: void-ctl group create <name> [--mode orchestrated|peer]");
                process::exit(1);
            }
            let name = &args[1];
            let mode = if args.len() > 3 && args[2] == "--mode" {
                &args[3]
            } else {
                "orchestrated"
            };

            let result = client
                .call("group_create", json!({"name": name, "mode": mode}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    process::exit(1);
                });

            println!(
                "Created group \"{}\" ({}) in {} mode.",
                name,
                &result["group_id"].as_str().unwrap_or("?")[..8],
                mode
            );
        }

        "join" => {
            if args.len() < 2 {
                eprintln!("usage: void-ctl group join <name>");
                process::exit(1);
            }
            client
                .call("group_join", json!({"group": &args[1]}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    process::exit(1);
                });
            println!("Joined group \"{}\".", &args[1]);
        }

        "leave" => {
            client
                .call("group_leave", json!({}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    process::exit(1);
                });
            println!("Left group.");
        }

        "dissolve" => {
            if args.len() < 2 {
                eprintln!("usage: void-ctl group dissolve <name>");
                process::exit(1);
            }
            client
                .call("group_dissolve", json!({"group": &args[1]}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    process::exit(1);
                });
            println!("Group \"{}\" dissolved.", &args[1]);
        }

        "list" => {
            let result = client
                .call("group_list", json!({}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    process::exit(1);
                });

            let groups = result["groups"].as_array().unwrap_or(&vec![]);
            if groups.is_empty() {
                println!("No groups.");
                return;
            }

            for g in groups {
                println!(
                    "  {} ({}, {}, {} members)",
                    g["name"].as_str().unwrap_or("?"),
                    &g["id"].as_str().unwrap_or("?")[..8],
                    g["mode"].as_str().unwrap_or("?"),
                    g["member_count"].as_u64().unwrap_or(0),
                );
                if let Some(members) = g["members"].as_array() {
                    for m in members {
                        println!(
                            "    {} {:<20} {:<12} {}",
                            match m["role"].as_str().unwrap_or("") {
                                "Orchestrator" => "\u{25B2}",
                                "Worker" => "\u{25BC}",
                                "Peer" => "\u{25C6}",
                                _ => " ",
                            },
                            m["title"].as_str().unwrap_or("?"),
                            m["status"].as_str().unwrap_or("?"),
                            &m["id"].as_str().unwrap_or("?")[..8],
                        );
                    }
                }
            }
        }

        "info" => {
            if args.len() < 2 {
                eprintln!("usage: void-ctl group info <name>");
                process::exit(1);
            }
            // Reuse group_list and filter
            let result = client
                .call("group_list", json!({}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    process::exit(1);
                });

            let groups = result["groups"].as_array().unwrap_or(&vec![]);
            let group = groups.iter().find(|g| g["name"].as_str() == Some(&args[1]));
            match group {
                Some(g) => println!("{}", serde_json::to_string_pretty(g).unwrap()),
                None => {
                    eprintln!("Group \"{}\" not found.", &args[1]);
                    process::exit(1);
                }
            }
        }

        _ => {
            eprintln!("unknown group command: {}", args[0]);
            process::exit(1);
        }
    }
}

fn cmd_context(client: &mut VoidClient, args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: void-ctl context <set|get|list|delete> [args...]");
        process::exit(1);
    }

    match args[0].as_str() {
        "set" => {
            if args.len() < 3 {
                eprintln!("usage: void-ctl context set <key> <value> [--ttl SECS]");
                process::exit(1);
            }
            let key = &args[1];
            let value = &args[2];
            let ttl = if args.len() > 4 && args[3] == "--ttl" {
                args[4].parse::<u64>().ok()
            } else {
                None
            };

            let mut params = json!({"key": key, "value": value});
            if let Some(ttl) = ttl {
                params["ttl_secs"] = json!(ttl);
            }

            client.call("context_set", params).unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                process::exit(1);
            });
            println!("Set.");
        }

        "get" => {
            if args.len() < 2 {
                eprintln!("usage: void-ctl context get <key>");
                process::exit(1);
            }
            let result = client
                .call("context_get", json!({"key": &args[1]}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    process::exit(1);
                });

            if result["value"].is_null() {
                eprintln!("Key \"{}\" not found.", &args[1]);
                process::exit(1);
            }

            // Print raw value (for use in shell scripts / variable capture)
            print!("{}", result["value"].as_str().unwrap_or(""));
        }

        "list" => {
            let prefix = if args.len() > 1 && args[1] == "--prefix" && args.len() > 2 {
                &args[2]
            } else {
                ""
            };

            let result = client
                .call("context_list", json!({"prefix": prefix}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    process::exit(1);
                });

            if let Some(entries) = result["entries"].as_array() {
                for entry in entries {
                    let key = entry["key"].as_str().unwrap_or("?");
                    let value = entry["value"].as_str().unwrap_or("?");
                    let preview = if value.len() > 60 {
                        format!("{}...", &value[..60])
                    } else {
                        value.to_string()
                    };
                    println!("{} = {}", key, preview);
                }
            }
        }

        "delete" => {
            if args.len() < 2 {
                eprintln!("usage: void-ctl context delete <key>");
                process::exit(1);
            }
            let result = client
                .call("context_delete", json!({"key": &args[1]}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    process::exit(1);
                });
            if result["deleted"].as_bool().unwrap_or(false) {
                println!("Deleted.");
            } else {
                println!("Key not found.");
            }
        }

        _ => {
            eprintln!("unknown context command: {}", args[0]);
            process::exit(1);
        }
    }
}

fn cmd_message(client: &mut VoidClient, args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: void-ctl message <send|list> [args...]");
        process::exit(1);
    }

    match args[0].as_str() {
        "send" => {
            if args.len() < 3 {
                eprintln!("usage: void-ctl message send <target-id> <payload>");
                process::exit(1);
            }
            let to = &args[1];
            let payload = args[2..].join(" ");
            client
                .call("message_send", json!({"to": to, "payload": payload}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    process::exit(1);
                });
            println!("Sent.");
        }

        "list" => {
            let result = client
                .call("message_list", json!({}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    process::exit(1);
                });

            if let Some(messages) = result["messages"].as_array() {
                if messages.is_empty() {
                    println!("No messages.");
                    return;
                }
                for msg in messages {
                    println!(
                        "[from {}] {}",
                        &msg["from"].as_str().unwrap_or("?")[..8],
                        msg["payload"].as_str().unwrap_or("?"),
                    );
                }
            }
        }

        _ => {
            eprintln!("unknown message command: {}", args[0]);
            process::exit(1);
        }
    }
}

fn cmd_spawn(client: &mut VoidClient, args: &[String]) {
    let mut cwd = None;
    let mut title = None;
    let mut group = None;
    let mut count: u64 = 1;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--cwd" => { i += 1; cwd = Some(args[i].clone()); }
            "--title" => { i += 1; title = Some(args[i].clone()); }
            "--group" => { i += 1; group = Some(args[i].clone()); }
            "--count" => { i += 1; count = args[i].parse().unwrap_or(1); }
            _ => {}
        }
        i += 1;
    }

    let mut params = json!({"count": count});
    if let Some(cwd) = cwd { params["cwd"] = json!(cwd); }
    if let Some(title) = title { params["title"] = json!(title); }
    if let Some(group) = group { params["group"] = json!(group); }

    let result = client
        .call("spawn", params)
        .unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            process::exit(1);
        });

    if let Some(terminals) = result["terminals"].as_array() {
        for t in terminals {
            println!("Spawned: {} ({})", t["id"].as_str().unwrap_or("?"), t["title"].as_str().unwrap_or("?"));
        }
    }
}

fn cmd_close(client: &mut VoidClient, args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: void-ctl close <target-id>");
        process::exit(1);
    }
    client
        .call("close", json!({"target": &args[0]}))
        .unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            process::exit(1);
        });
    println!("Closed.");
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max - 3])
    } else {
        s.to_string()
    }
}

fn print_usage() {
    println!("void-ctl — control Void terminals from the command line");
    println!();
    println!("USAGE:");
    println!("  void-ctl <command> [args...]");
    println!();
    println!("TERMINAL COMMANDS:");
    println!("  list                                    List all terminals");
    println!("  send <id> <command>                     Send command to terminal");
    println!("  send --group <name> <command>           Broadcast to group");
    println!("  read <id> [--lines N] [--screen]        Read terminal output");
    println!("  read --group <name> [--lines N]         Read all group output");
    println!("  wait-idle <id> [--timeout N]            Wait for terminal idle");
    println!("  wait-idle --group <name> [--timeout N]  Wait for group idle");
    println!("  status <id> <status> [message]          Set terminal status");
    println!("  spawn [--cwd P] [--group G] [--count N] Spawn new terminal(s)");
    println!("  close <id>                              Close a terminal");
    println!();
    println!("GROUP COMMANDS:");
    println!("  group create <name> [--mode M]          Create group (orchestrated|peer)");
    println!("  group join <name>                       Join a group");
    println!("  group leave                             Leave current group");
    println!("  group dissolve <name>                   Dissolve a group");
    println!("  group list                              List all groups");
    println!("  group info <name>                       Show group details");
    println!();
    println!("CONTEXT COMMANDS:");
    println!("  context set <key> <value> [--ttl N]     Set shared context");
    println!("  context get <key>                       Get shared context");
    println!("  context list [--prefix P]               List context entries");
    println!("  context delete <key>                    Delete context entry");
    println!();
    println!("MESSAGE COMMANDS:");
    println!("  message send <id> <payload>             Send direct message");
    println!("  message list                            List received messages");
    println!();
    println!("ENVIRONMENT:");
    println!("  VOID_TERMINAL_ID  This terminal's UUID (auto-set)");
}
```

---

## 10. Title Bar Status Integration

### 10.1 Current Title Bar

The title bar is rendered in `terminal/panel.rs` inside the `render_title_bar()` method.
Currently it shows:
```
[color indicator] Terminal Title                    [X]
```

### 10.2 New Title Bar with Group Status

When a terminal is part of a group, the title bar shows:

```
[color] [group_name ROLE_ARROW status] Terminal Title      [X]
```

Examples:
```
[blue] [build ▲ idle] claude                               [X]   <- orchestrator, idle
[red]  [build ▼ running] zsh                               [X]   <- worker, running
[green][build ▼ done] zsh                                  [X]   <- worker, done
[gold] [research ◆ idle] claude                            [X]   <- peer, idle
```

### 10.3 Status Colors

| Status    | Text Color               | Background              |
|-----------|--------------------------|-------------------------|
| `idle`    | Muted gray (#888888)     | None                    |
| `running` | Bright cyan (#00CCFF)    | Subtle pulse animation  |
| `waiting` | Yellow (#FFCC00)         | None                    |
| `done`    | Green (#44CC44)          | Fades after 5 seconds   |
| `error`   | Red (#FF4444)            | Fades after 10 seconds  |

### 10.4 Rendering Implementation

```rust
// Addition to terminal/panel.rs — inside render_title_bar()

/// Render the group status badge in the title bar.
///
/// Called inside render_title_bar() after drawing the panel color indicator
/// and before drawing the title text.
fn render_group_badge(
    ui: &mut egui::Ui,
    group_name: &str,
    role: TerminalRole,
    status: &TerminalStatus,
    rect: egui::Rect,
) -> f32 {
    // Badge text: "[group_name ARROW status]"
    let arrow = role.indicator();
    let status_label = status.label();
    let badge_text = format!("{} {} {}", group_name, arrow, status_label);

    // Status color
    let status_color = match status {
        TerminalStatus::Idle => Color32::from_rgb(136, 136, 136),
        TerminalStatus::Running { .. } => Color32::from_rgb(0, 204, 255),
        TerminalStatus::Waiting { .. } => Color32::from_rgb(255, 204, 0),
        TerminalStatus::Done { .. } => Color32::from_rgb(68, 204, 68),
        TerminalStatus::Error { .. } => Color32::from_rgb(255, 68, 68),
    };

    // Background pill
    let font_id = egui::FontId::monospace(12.0);
    let galley = ui.painter().layout_no_wrap(
        badge_text.clone(),
        font_id.clone(),
        status_color,
    );
    let text_width = galley.size().x;

    let badge_rect = egui::Rect::from_min_size(
        egui::pos2(rect.min.x + 28.0, rect.min.y + 3.0),
        egui::vec2(text_width + 12.0, 18.0),
    );

    // Draw background pill with rounded corners
    let bg_color = Color32::from_rgba_premultiplied(
        status_color.r(),
        status_color.g(),
        status_color.b(),
        25,  // Very subtle background
    );
    ui.painter().rect_filled(badge_rect, 4.0, bg_color);

    // Draw border
    let border_color = Color32::from_rgba_premultiplied(
        status_color.r(),
        status_color.g(),
        status_color.b(),
        60,
    );
    ui.painter().rect_stroke(
        badge_rect,
        4.0,
        egui::Stroke::new(1.0, border_color),
    );

    // Draw text
    let text_pos = egui::pos2(
        badge_rect.min.x + 6.0,
        badge_rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(text_pos, galley, status_color);

    // Return width consumed (for title text offset)
    badge_rect.width() + 8.0
}

/// Get the group badge info for a terminal.
///
/// Returns (group_name, role, status) if the terminal is in a group.
/// Called from TerminalPanel::show() before render_title_bar().
fn get_group_badge_info(
    bus: &TerminalBus,
    terminal_id: Uuid,
) -> Option<(String, TerminalRole, TerminalStatus)> {
    let info = bus.get_terminal(terminal_id)?;
    let group_name = info.group_name?;
    Some((group_name, info.role, info.status))
}
```

### 10.5 Running Status Animation

When a terminal's status is `Running`, a subtle animation indicates activity:

```rust
/// Render a pulsing dot next to the status text to indicate active execution.
fn render_running_indicator(
    ui: &mut egui::Ui,
    center: egui::Pos2,
    time: f64,
) {
    // Pulsing opacity: sin wave between 0.3 and 1.0
    let pulse = (time * 3.0).sin() as f32 * 0.35 + 0.65;
    let color = Color32::from_rgba_premultiplied(
        0,
        (204.0 * pulse) as u8,
        (255.0 * pulse) as u8,
        (255.0 * pulse) as u8,
    );

    ui.painter().circle_filled(center, 3.0, color);
}
```

### 10.6 Canvas-Level Group Visualization

On the infinite canvas, terminals in the same group can be visually connected:

```rust
/// Draw subtle connection lines between grouped terminals on the canvas.
///
/// Called in app.rs during the canvas background layer, before panel rendering.
fn render_group_connections(
    painter: &egui::Painter,
    bus: &TerminalBus,
    panels: &[CanvasPanel],
    transform: egui::emath::TSTransform,
) {
    let groups = bus.list_groups();

    for group in &groups {
        if group.member_count < 2 {
            continue;
        }

        // Find panel positions for group members
        let member_centers: Vec<egui::Pos2> = group
            .members
            .iter()
            .filter_map(|m| {
                panels.iter().find(|p| p.id() == m.terminal_id).map(|p| {
                    let pos = p.position();
                    let size = p.size();
                    egui::pos2(pos.x + size.x / 2.0, pos.y + size.y / 2.0)
                })
            })
            .collect();

        if member_centers.len() < 2 {
            continue;
        }

        // Group color — hash the group name for consistency
        let hue = (group.name.bytes().map(|b| b as u32).sum::<u32>() % 360) as f32;
        let group_color = egui::ecolor::Hsva::new(hue / 360.0, 0.4, 0.7, 0.15);
        let line_color: Color32 = group_color.into();

        // Draw lines between all pairs (star topology from orchestrator, or mesh for peers)
        match &group.mode.as_str() {
            &"orchestrated" => {
                // Star: lines from orchestrator to each worker
                if let Some(orch_center) = group.orchestrator_id.and_then(|oid| {
                    member_centers.iter().copied().find(|_| true) // first is orchestrator
                }) {
                    for center in &member_centers[1..] {
                        let from = transform * orch_center;
                        let to = transform * *center;
                        painter.line_segment(
                            [from, to],
                            egui::Stroke::new(1.5, line_color),
                        );
                    }
                }
            }
            _ => {
                // Mesh: lines between all adjacent pairs
                for i in 0..member_centers.len() {
                    let next = (i + 1) % member_centers.len();
                    let from = transform * member_centers[i];
                    let to = transform * member_centers[next];
                    painter.line_segment(
                        [from, to],
                        egui::Stroke::new(1.0, line_color),
                    );
                }
            }
        }
    }
}
```

---

## 11. Shared Context Store

### 11.1 Design

The shared context is a key-value store with these features:

- **Global namespace**: Keys without a prefix are accessible to all terminals.
- **Group namespace**: Keys prefixed with `group_name:` are logically scoped to that group
  (though technically any terminal can read them — the prefix is a convention).
- **TTL support**: Entries can expire after a set duration.
- **Source tracking**: Each entry records which terminal wrote it and when.
- **Lazy cleanup**: Expired entries are removed on next access, not by a background thread.

### 11.2 Naming Conventions

| Pattern | Scope | Example |
|---------|-------|---------|
| `key` | Global | `test_results`, `build_status` |
| `group:key` | Group-scoped | `build:test_output`, `research:finding_1` |
| `_msg:from:to:ts` | System (messages) | `_msg:abc:def:1234567890` |
| `_meta:key` | System (metadata) | `_meta:created_at` |

### 11.3 Usage Patterns

**Pattern 1: Scatter-Gather**

Orchestrator sends commands to workers, each worker stores its result in context,
orchestrator reads all results:

```bash
# Orchestrator
void-ctl send --group build "cargo test --lib 2>&1 | void-ctl context set build:test_lib -"
void-ctl send --group build "cargo test --bins 2>&1 | void-ctl context set build:test_bins -"
void-ctl wait-idle --group build
LIB=$(void-ctl context get build:test_lib)
BINS=$(void-ctl context get build:test_bins)
```

**Pattern 2: Shared Knowledge Base**

Multiple Claude Code instances build up a shared understanding:

```bash
# Claude A discovers something
void-ctl context set auth_mechanism "JWT with RS256, tokens stored in HttpOnly cookies"

# Claude B reads it later, doesn't need to re-discover
AUTH=$(void-ctl context get auth_mechanism)
```

**Pattern 3: Status Board**

Workers report their progress via context:

```bash
# Worker 1
void-ctl context set build:worker1_status "compiling: 45/120 crates"
# Worker 2
void-ctl context set build:worker2_status "testing: 12/42 tests passed"

# Orchestrator reads dashboard
void-ctl context list --prefix build:
```

---

## 12. Event & Subscription System

### 12.1 Event Flow

```
Terminal Action (output, title change, exit)
        │
        ▼
  Terminal Bus detects change
        │
        ▼
  bus.emit(BusEvent::...)
        │
        ▼
  For each subscriber:
    if filter.matches(event):
      tx.send(event)  (non-blocking)
        │
        ▼
  Socket server forwards to subscribed clients as JSON-RPC notifications
```

### 12.2 Subscription from void-ctl

```bash
# Watch for status changes in a group
void-ctl subscribe --group build --events status.changed

# Output (streaming):
# {"type":"status.changed","terminal_id":"e5f6...","old":"idle","new":"running"}
# {"type":"status.changed","terminal_id":"e5f6...","old":"running","new":"done"}
# {"type":"status.changed","terminal_id":"c9d0...","old":"idle","new":"running"}
```

### 12.3 Subscription Filters

Clients can filter by:
- **Event type**: `status.changed`, `output.changed`, `terminal.exited`, etc.
- **Terminal ID**: Only events involving specific terminals.
- **Group ID**: Only events from terminals in a specific group.

Filters are AND-combined: all specified filters must match for an event to be delivered.

### 12.4 Output Change Coalescing

The `OutputChanged` event is special because terminal output can change thousands of
times per second during heavy output. To avoid flooding subscribers:

1. The reader thread sets an `output_dirty: AtomicBool` flag instead of emitting events.
2. The bus's `tick_statuses()` method (called per frame, ~60Hz) checks dirty flags
   and emits coalesced `OutputChanged` events at most once per 100ms per terminal.

```rust
// In the bus tick (called from app.rs::update, ~60fps)
pub fn tick_output_events(&mut self) {
    for (id, handle) in &self.terminals {
        // Check if output changed since last tick
        let last_output = handle.last_output_at.lock().ok();
        if let Some(last_output) = last_output {
            if last_output.elapsed() < Duration::from_millis(100) {
                self.emit(BusEvent::OutputChanged { terminal_id: *id });
            }
        }
    }
}
```

---

## 13. Integration with Existing Code

### 13.1 Changes to `src/terminal/pty.rs`

```rust
// Add this method to PtyHandle:

/// Create a TerminalHandle from this PtyHandle's Arc references.
///
/// The handle is a lightweight, cloneable view into the same terminal state.
/// It does not own anything — just holds Arc clones.
pub fn create_handle(&self, panel_id: Uuid, workspace_id: Uuid) -> TerminalHandle {
    TerminalHandle {
        id: panel_id,
        term: Arc::clone(&self.term),
        writer: Arc::clone(&self.writer),
        title: Arc::clone(&self.title),
        alive: Arc::clone(&self.alive),
        last_input_at: Arc::clone(&self.last_input_at),
        last_output_at: Arc::clone(&self.last_output_at),
        workspace_id,
    }
}

// Modify spawn() to accept additional environment variables:

pub fn spawn(
    ctx: &egui::Context,
    rows: u16,
    cols: u16,
    title: &str,
    cwd: Option<&std::path::Path>,
    extra_env: Option<&HashMap<String, String>>,  // NEW PARAMETER
) -> anyhow::Result<Self> {
    // ... existing code ...

    let mut cmd = CommandBuilder::new_default_prog();
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("VOID_TERMINAL", "1");

    // NEW: Set IPC environment variables
    if let Some(env) = extra_env {
        for (key, value) in env {
            cmd.env(key, value);
        }
    }

    // ... rest of existing code ...
}
```

### 13.2 Changes to `src/app.rs`

```rust
// Add to VoidApp struct:

pub struct VoidApp {
    // ... existing fields ...

    /// The terminal communication bus.
    bus: Arc<Mutex<TerminalBus>>,
}

// In VoidApp::new():

pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
    // ... existing initialization ...

    let bus = Arc::new(Mutex::new(TerminalBus::new()));

    Self {
        // ... existing fields ...
        bus,
    }
}

// In VoidApp::update() — add bus tick:

fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    // ... existing code ...

    // Tick bus statuses (auto-detect idle terminals)
    if let Ok(mut bus) = self.bus.lock() {
        bus.tick_statuses();
    }

    // ... rest of existing update code ...
}

// When spawning terminals, pass IPC env vars:

fn spawn_terminal_with_bus(&mut self, workspace_idx: usize) {
    let ws = &mut self.workspaces[workspace_idx];
    let panel_id = Uuid::new_v4();

    let mut extra_env = HashMap::new();
    extra_env.insert("VOID_TERMINAL_ID".into(), panel_id.to_string());
    extra_env.insert("VOID_WORKSPACE_ID".into(), ws.id.to_string());

    // Pass extra_env to terminal panel creation
    // ... (terminal creation code with extra_env) ...

    // Register with bus
    if let Some(pty) = &panel.pty() {
        let handle = pty.create_handle(panel_id, ws.id);
        if let Ok(mut bus) = self.bus.lock() {
            bus.register(handle);
        }
    }
}
```

### 13.3 Changes to `src/state/workspace.rs`

```rust
// Modify spawn_terminal to accept bus + IPC config:

pub fn spawn_terminal(
    &mut self,
    ctx: &egui::Context,
    bus: &Arc<Mutex<TerminalBus>>,
) {
    let panel_id = Uuid::new_v4();

    let mut extra_env = std::collections::HashMap::new();
    extra_env.insert("VOID_TERMINAL_ID".to_string(), panel_id.to_string());
    extra_env.insert("VOID_WORKSPACE_ID".to_string(), self.id.to_string());

    let panel = TerminalPanel::new_with_terminal(
        ctx,
        panel_id,
        &format!("Terminal {}", self.panels.len() + 1),
        self.next_color,
        DEFAULT_PANEL_WIDTH,
        DEFAULT_PANEL_HEIGHT,
        self.next_z,
        self.cwd.as_deref(),
        Some(&extra_env),
    );

    // Register with bus
    if let Some(ref panel) = panel.pty() {
        let handle = panel.create_handle(panel_id, self.id);
        if let Ok(mut bus) = bus.lock() {
            bus.register(handle);
        }
    }

    // ... existing placement code ...
}

// Modify close_panel to deregister:

pub fn close_panel(
    &mut self,
    index: usize,
    bus: &Arc<Mutex<TerminalBus>>,
) {
    let panel_id = self.panels[index].id();

    // Deregister from bus
    if let Ok(mut bus) = bus.lock() {
        bus.deregister(panel_id);
    }

    // ... existing close code ...
}
```

### 13.4 Changes to `Cargo.toml`

```toml
# Add void-ctl binary
[[bin]]
name = "void-ctl"
path = "src/bin/void-ctl.rs"

# Dependencies for void-ctl (already present: serde, serde_json, uuid)
# No new dependencies needed for the bus.
# For void-ctl arg parsing, clap is optional — the implementation above
# uses manual parsing to avoid the dependency. Add clap if desired:
# [dependencies]
# clap = { version = "4", features = ["derive"], optional = true }
```

### 13.5 New File Structure

```
src/
  bus/
    mod.rs          # TerminalBus implementation
    types.rs        # All type definitions (TerminalHandle, Group, Status, etc.)
  terminal/
    pty.rs          # APC interception code lives here (extract_void_commands,
                    # handle_bus_command, dispatch_bus_method) — added to the
                    # existing reader thread, no separate module needed
  bin/
    void-ctl.rs     # CLI binary
```

---

## 14. Security Model

### 14.1 Trust Boundary

The PTY pipe **is** the authentication. Only the child process of a terminal's shell
(and its descendants) can write to that terminal's PTY stdout. The OS enforces this —
no token needed, no socket to protect.

This is stronger than token-based auth:
- tmux relies on socket file permissions (can be misconfigured)
- VS Code terminal API uses random tokens (can be leaked via env)
- Jupyter notebooks use token-based auth (visible in process list)

With APC-over-PTY, there is nothing to leak and nothing to misconfigure.

### 14.2 Attack Surface

- **No network surface.** There is no listening socket. Nothing to connect to from
  outside the process. Port scanners find nothing. Firewalls are irrelevant.
- **No token to leak.** The only env var is `VOID_TERMINAL_ID`, which is a UUID that
  identifies the terminal but does not grant access. Access comes from being a child
  process of the terminal's shell.
- **No auth to bypass.** There is no authentication handshake to get wrong. If you can
  write to the PTY, you are already inside the trust boundary.

### 14.3 Process Isolation

Each terminal's APC interception runs in that terminal's reader thread. A command
received on terminal A's PTY can only identify itself as terminal A — the terminal ID
is set by the reader thread, not by the client. A malicious child process cannot
impersonate another terminal.

### 14.4 Permission Model

Within the bus (unchanged from the in-process layer):
- Standalone terminals (not in a group) can be controlled by any terminal via the bus.
- In orchestrated groups, only the orchestrator can inject commands into workers.
  Workers can send messages to the orchestrator but cannot inject into each other.
- In peer groups, any peer can inject into any other peer.
- Context is globally readable/writable (scoped by convention, not enforcement).

---

## 15. Usage Scenarios

### 15.1 Claude Code Multi-Agent Orchestration

The primary use case. One Claude Code instance manages a team of workers:

```bash
# Terminal A: Claude Code orchestrator
$ claude

User: Run the full test suite, lint, and type-check in parallel.
      Summarize all results.

# Claude Code internally does:
$ void-ctl group create pipeline
$ void-ctl spawn --group pipeline --count 3 --cwd /project

# Get the new terminal IDs
$ WORKERS=$(void-ctl list | grep pipeline | grep Worker | awk '{print $1}')
$ W1=$(echo "$WORKERS" | sed -n 1p)
$ W2=$(echo "$WORKERS" | sed -n 2p)
$ W3=$(echo "$WORKERS" | sed -n 3p)

# Dispatch work
$ void-ctl send $W1 "cargo test 2>&1; echo '---VOID-EXIT-CODE:'\$?"
$ void-ctl send $W2 "cargo clippy --all-targets 2>&1; echo '---VOID-EXIT-CODE:'\$?"
$ void-ctl send $W3 "cargo check 2>&1; echo '---VOID-EXIT-CODE:'\$?"

# Wait for all to finish
$ void-ctl wait-idle --group pipeline --timeout 300

# Gather results
$ TEST_OUT=$(void-ctl read $W1 --lines 100)
$ LINT_OUT=$(void-ctl read $W2 --lines 100)
$ CHECK_OUT=$(void-ctl read $W3 --lines 100)

# Store for other agents
$ void-ctl context set pipeline:tests "$TEST_OUT"
$ void-ctl context set pipeline:lint "$LINT_OUT"
$ void-ctl context set pipeline:check "$CHECK_OUT"

# Clean up
$ void-ctl group dissolve pipeline
```

The user sees all four terminals on the canvas, each showing live output.

### 15.2 Shared Research Session

Multiple Claude Code instances research different aspects of a codebase:

```bash
# Terminal A: Claude researches authentication
$ claude
User: Investigate the auth system and share what you find.

# Claude A stores findings:
$ void-ctl context set auth:summary "JWT RS256, 1h expiry, refresh via /api/refresh"
$ void-ctl context set auth:files "src/auth/jwt.rs, src/auth/middleware.rs, src/routes/refresh.rs"
$ void-ctl context set auth:issues "Token refresh has no rate limiting"

# Terminal B: Claude researches database layer
$ claude
User: Check shared context first, then investigate the database layer.

# Claude B reads Claude A's findings:
$ AUTH_SUMMARY=$(void-ctl context get auth:summary)
# "JWT RS256, 1h expiry, refresh via /api/refresh"
# Now Claude B knows about auth without re-investigating

$ void-ctl context set db:summary "PostgreSQL via sqlx, 42 migrations, connection pool max 20"
$ void-ctl context set db:issues "No index on users.email, full table scan on login"
```

### 15.3 Log Monitoring Pipeline

One terminal tails logs, another processes them:

```bash
# Terminal A (orchestrator): Monitor and dispatch
$ void-ctl group create monitor
$ void-ctl spawn --group monitor --title "log-watcher"
$ void-ctl spawn --group monitor --title "alert-handler"

$ WATCHER=$(void-ctl list | grep log-watcher | awk '{print $1}')
$ HANDLER=$(void-ctl list | grep alert-handler | awk '{print $1}')

$ void-ctl send $WATCHER "tail -f /var/log/app.log | grep ERROR"

# Periodically check for new errors
$ void-ctl read $WATCHER --lines 5
# If errors found, dispatch to handler
$ void-ctl send $HANDLER "investigate_error 'connection pool exhausted'"
```

### 15.4 Interactive Tutorial

A teaching terminal guides the student through exercises:

```bash
# Teacher terminal sets up the exercise
$ void-ctl context set tutorial:step "1"
$ void-ctl context set tutorial:instruction "Create a function that reverses a string"
$ void-ctl context set tutorial:hint "Use .chars().rev().collect()"

# Student terminal reads the current step
$ STEP=$(void-ctl context get tutorial:step)
$ INSTRUCTION=$(void-ctl context get tutorial:instruction)
echo "Step $STEP: $INSTRUCTION"

# Student completes the exercise, teacher advances
$ void-ctl context set tutorial:step "2"
$ void-ctl context set tutorial:instruction "Now write tests for your reverse function"
```

---

## 16. API Reference

### 16.1 Terminal Bus Methods (Rust API)

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `() -> Self` | Create empty bus |
| `register` | `(&mut self, TerminalHandle)` | Register a terminal |
| `deregister` | `(&mut self, Uuid)` | Remove a terminal |
| `list_terminals` | `(&self) -> Vec<TerminalInfo>` | List all terminals |
| `get_terminal` | `(&self, Uuid) -> Option<TerminalInfo>` | Get terminal info |
| `get_handle` | `(&self, Uuid) -> Option<TerminalHandle>` | Get cloneable handle |
| `is_alive` | `(&self, Uuid) -> Option<bool>` | Check liveness |
| `inject_bytes` | `(&mut self, Uuid, &[u8], Option<Uuid>) -> Result` | Write raw bytes |
| `send_command` | `(&mut self, Uuid, &str, Option<Uuid>) -> Result` | Send command + Enter |
| `send_interrupt` | `(&mut self, Uuid, Option<Uuid>) -> Result` | Send Ctrl+C |
| `read_screen` | `(&self, Uuid) -> Result<Vec<String>>` | Read visible screen |
| `read_output` | `(&self, Uuid, usize) -> Result<Vec<String>>` | Read N lines with scrollback |
| `read_screen_text` | `(&self, Uuid) -> Result<String>` | Screen as single string |
| `read_output_text` | `(&self, Uuid, usize) -> Result<String>` | Output as single string |
| `is_idle` | `(&self, Uuid) -> Result<bool>` | Check idle state |
| `wait_idle_handle` | `(handle, Duration, Duration) -> bool` | Block until idle |
| `get_status` | `(&self, Uuid) -> Option<&TerminalStatus>` | Get status |
| `set_status` | `(&mut self, Uuid, TerminalStatus, Option<Uuid>) -> Result` | Set status |
| `tick_statuses` | `(&mut self)` | Auto-update statuses |
| `create_orchestrated_group` | `(&mut self, &str, Uuid) -> Result<Uuid>` | Create orchestrated group |
| `create_peer_group` | `(&mut self, &str, Uuid) -> Result<Uuid>` | Create peer group |
| `join_group` | `(&mut self, Uuid, Uuid) -> Result` | Join group by ID |
| `join_group_by_name` | `(&mut self, Uuid, &str) -> Result` | Join group by name |
| `leave_group` | `(&mut self, Uuid) -> Result` | Leave group |
| `dissolve_group` | `(&mut self, Uuid)` | Dissolve group |
| `list_groups` | `(&self) -> Vec<GroupInfo>` | List all groups |
| `get_group` | `(&self, Uuid) -> Option<GroupInfo>` | Get group info |
| `get_group_by_name` | `(&self, &str) -> Option<GroupInfo>` | Get group by name |
| `broadcast_command` | `(&mut self, Uuid, &str, Uuid) -> Result<Vec<Uuid>>` | Send to all workers |
| `send_message` | `(&mut self, Uuid, Uuid, &str) -> Result` | Direct message |
| `context_set` | `(&mut self, &str, &str, Uuid, Option<Duration>) -> Result` | Set context |
| `context_get` | `(&mut self, &str) -> Option<String>` | Get context |
| `context_get_entry` | `(&mut self, &str) -> Option<ContextEntry>` | Get context + metadata |
| `context_list` | `(&mut self) -> Vec<(String, ContextEntry)>` | List all entries |
| `context_delete` | `(&mut self, &str) -> bool` | Delete entry |
| `list_messages` | `(&mut self, Uuid) -> Vec<(Uuid, String, SystemTime)>` | List messages |
| `subscribe` | `(&mut self, EventFilter) -> (Uuid, Receiver<BusEvent>)` | Subscribe to events |
| `unsubscribe` | `(&mut self, Uuid)` | Unsubscribe |

### 16.2 void-ctl Commands

```
void-ctl list
void-ctl send <id> <command>
void-ctl send --group <name> <command>
void-ctl read <id> [--lines N] [--screen]
void-ctl read --group <name> [--lines N]
void-ctl wait-idle <id> [--timeout N] [--quiet N]
void-ctl wait-idle --group <name> [--timeout N] [--quiet N]
void-ctl status <id> <idle|running|waiting|done|error> [message]
void-ctl spawn [--cwd PATH] [--title TITLE] [--group NAME] [--count N]
void-ctl close <id>
void-ctl group create <name> [--mode orchestrated|peer]
void-ctl group join <name>
void-ctl group leave
void-ctl group dissolve <name>
void-ctl group list
void-ctl group info <name>
void-ctl context set <key> <value> [--ttl SECS]
void-ctl context get <key>
void-ctl context list [--prefix PREFIX]
void-ctl context delete <key>
void-ctl message send <id> <payload>
void-ctl message list
```

### 16.3 Environment Variables

| Variable | Set By | Used By | Example |
|----------|--------|---------|---------|
| `VOID_TERMINAL` | PtyHandle::spawn | Shell scripts | `1` |
| `VOID_TERMINAL_ID` | PtyHandle::spawn | void-ctl | `550e8400-e29b-...` |
| `VOID_WORKSPACE_ID` | PtyHandle::spawn | void-ctl | `6ba7b810-9dad-...` |

---

## 17. Testing Strategy

### 17.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bus_register_and_list() {
        let mut bus = TerminalBus::new();
        // Create a mock TerminalHandle (would need mock Term)
        // bus.register(handle);
        // assert_eq!(bus.list_terminals().len(), 1);
    }

    #[test]
    fn test_group_lifecycle() {
        let mut bus = TerminalBus::new();
        // Register terminals, create group, join, leave, dissolve
        // Verify state at each step
    }

    #[test]
    fn test_context_set_get() {
        let mut bus = TerminalBus::new();
        let id = Uuid::new_v4();
        // Register terminal, set context, get context
        // Verify TTL expiration
    }

    #[test]
    fn test_orchestrator_permission() {
        // Verify workers cannot inject into other workers
        // Verify orchestrator can inject into any worker
    }

    #[test]
    fn test_event_filter() {
        let filter = EventFilter {
            event_types: vec!["status.changed".into()],
            terminal_ids: vec![],
            group_id: None,
        };
        // Verify filter matches status.changed but not output.changed
    }

    #[test]
    fn test_context_ttl() {
        // Set entry with short TTL, verify it expires
    }
}
```

### 17.2 Integration Tests

```rust
#[cfg(test)]
mod integration {
    // Test APC interception round-trip
    // 1. Start server with mock bus
    // 2. Connect client, authenticate
    // 3. Call methods, verify responses
    // 4. Subscribe to events, verify delivery
}
```

### 17.3 End-to-End Test

```bash
#!/bin/bash
# test_orchestration.sh — run inside Void

# 1. Create a group
void-ctl group create test-e2e
echo "Group created: $?"

# 2. Spawn workers
void-ctl spawn --group test-e2e --count 2
echo "Workers spawned: $?"

# 3. List and verify
void-ctl group info test-e2e

# 4. Send command to group
void-ctl send --group test-e2e "echo hello-from-worker"

# 5. Wait for idle
void-ctl wait-idle --group test-e2e --timeout 10

# 6. Read output
OUTPUT=$(void-ctl read --group test-e2e --lines 5)
echo "$OUTPUT" | grep "hello-from-worker" && echo "PASS" || echo "FAIL"

# 7. Test context
void-ctl context set test_key "test_value"
VALUE=$(void-ctl context get test_key)
[ "$VALUE" = "test_value" ] && echo "PASS" || echo "FAIL"

# 8. Cleanup
void-ctl group dissolve test-e2e
echo "Dissolved: $?"
```

---

## 18. Future Extensions

### 18.1 Terminal Linking (Visual)

Draw visible "pipes" between terminals on the canvas. The output of terminal A visually
flows into terminal B. Click a pipe to see the data flowing through it.

### 18.2 Replay & Recording

Record all bus events for a session. Replay them later to understand what happened
during an orchestration run. Useful for debugging complex multi-agent workflows.

### 18.3 Remote Orchestration

Add a socket API layer that accepts connections over the network (with TLS + proper auth).
This enables one Void instance to orchestrate terminals on another machine.

### 18.4 Workflow Templates

Save and replay orchestration patterns:
```yaml
# .void/workflows/test-pipeline.yml
name: test-pipeline
mode: orchestrated
workers: 3
steps:
  - broadcast: "cargo test --lib"
    wait: idle
  - broadcast: "cargo test --doc"
    wait: idle
  - gather:
      key: test_results
      format: summary
```

### 18.5 AI Agent Protocol

A standardized protocol for AI agents to discover and use Void's orchestration
capabilities. The agent detects `VOID_TERMINAL_ID` in its environment and automatically
knows it can spawn workers, share context, and coordinate with other agents.

### 18.6 Terminal Dependencies

Express that terminal B depends on terminal A finishing:
```bash
void-ctl dependency add $TERM_B --after $TERM_A
# Terminal B shows "waiting" until terminal A becomes idle
# Then automatically starts the queued command
```

### 18.7 Shared Scrollback View

A special panel type that shows a merged, chronological view of output from all
terminals in a group. Like a unified log view with color-coded source indicators.

### 18.8 Group Persistence

Save group configurations to disk so they survive application restart:
```json
{
    "groups": [
        {
            "name": "build",
            "mode": "orchestrated",
            "auto_spawn_workers": 2,
            "cwd": "/project"
        }
    ]
}
```

---

## Summary

This system transforms Void from a terminal emulator into an **agent workspace**.

The architecture is layered:
1. **Terminal Bus** — in-process, zero dependencies, pure `std::sync`
2. **APC Layer** — escape sequences through existing PTY pipe, zero infrastructure
3. **void-ctl** — shell-native CLI, reads env vars, simple to use
4. **Title bar badges** — visual feedback for group membership and status
5. **Shared context** — key-value store with TTL and namespacing

Every layer builds on the one below it. The bus is the foundation — fast, safe, and
invisible to terminals that don't use it. The APC layer makes the bus accessible to
child processes through the PTY pipe. The CLI makes the bus accessible to humans and
AI agents.

The total implementation is approximately:
- Bus + types: ~800 lines of Rust
- APC interception: ~200 lines of Rust
- void-ctl: ~400 lines of Rust
- Title bar rendering: ~100 lines of Rust
- Integration changes: ~100 lines across existing files
- **Total: ~1,600 lines of new Rust code**

Zero new external dependencies. The void-ctl binary reuses existing `serde_json` and
`uuid` crates.

Everything stays 100% Rust. Everything works on Windows, Linux, and macOS.
Everything is opt-in. A terminal that never touches `void-ctl` behaves exactly as
it does today.
