// src/bus/mod.rs

pub mod apc;
pub mod server;
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
        let title = handle.title.lock().map(|t| t.clone()).unwrap_or_default();
        let alive = handle.alive.load(Ordering::Relaxed);
        let status = self.statuses.get(&handle.id).cloned().unwrap_or_default();
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
    fn check_injection_permission(&self, source: Uuid, target: Uuid) -> Result<(), BusError> {
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

        let grid = term.grid();
        let cols = grid.columns();
        let lines = grid.screen_lines();

        let mut result = Vec::with_capacity(lines);

        // Build lines from the grid
        for line_idx in 0..lines {
            let mut line_str = String::with_capacity(cols);
            for col in 0..cols {
                let point = alacritty_terminal::index::Point::new(
                    alacritty_terminal::index::Line(line_idx as i32),
                    alacritty_terminal::index::Column(col),
                );
                let cell = &grid[point];
                let c = cell.c;
                if c == '\0' {
                    line_str.push(' ');
                } else {
                    line_str.push(c);
                }
            }
            result.push(line_str.trim_end().to_string());
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
    pub fn read_output(&self, target: Uuid, lines: usize) -> Result<Vec<String>, BusError> {
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
        let cols = grid.columns();
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
                    if output_elapsed >= IDLE_THRESHOLD && started_at.elapsed() > IDLE_THRESHOLD {
                        transitions.push((
                            *id,
                            TerminalStatus::Done {
                                finished_at: Instant::now(),
                            },
                        ));
                    }
                }
            }
        }

        for (id, new_status) in transitions {
            let old_label = self
                .statuses
                .get(&id)
                .map(|s| s.label().to_string())
                .unwrap_or_default();
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
    pub fn create_peer_group(&mut self, name: &str, creator: Uuid) -> Result<Uuid, BusError> {
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
    pub fn join_group(&mut self, terminal_id: Uuid, group_id: Uuid) -> Result<(), BusError> {
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
        let did_remove;

        if let Some(group) = self.groups.get_mut(&group_id) {
            did_remove = group.remove_member(terminal_id);
            should_dissolve = group.is_empty() || group.is_orchestrator(terminal_id);
        } else {
            return;
        }

        if did_remove {
            self.emit(BusEvent::GroupMemberLeft {
                group_id,
                terminal_id,
            });
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
    pub fn send_message(&mut self, from: Uuid, to: Uuid, payload: &str) -> Result<(), BusError> {
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
        let prefix = "_msg:".to_string();
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
    pub fn subscribe(&mut self, filter: EventFilter) -> (Uuid, mpsc::Receiver<BusEvent>) {
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
