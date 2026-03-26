// src/orchestration/prompt.rs — Coordination prompt generation for agents

use uuid::Uuid;

/// Generate the leader coordination prompt.
#[allow(dead_code)]
pub fn leader_prompt(
    terminal_id: Uuid,
    team_name: &str,
    group_id: Uuid,
    worker_count: usize,
    bus_port: u16,
) -> String {
    format!(
        r#"# ─── VOID ORCHESTRATION PROTOCOL ────────────────────────────────
# You are running inside Void, an infinite canvas terminal emulator
# with built-in swarm intelligence. Your terminal ID is: {terminal_id}
# Your role: LEADER
# Your team: {team_name}
# Group ID: {group_id}
# Workers: {worker_count}
# Bus port: {bus_port}
#
# Available commands (use void-ctl):
#   void-ctl task create "subject" --assign $WORKER_ID
#   void-ctl task list
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
"#
    )
}

/// Generate the worker coordination prompt.
#[allow(dead_code)]
pub fn worker_prompt(
    terminal_id: Uuid,
    team_name: &str,
    group_id: Uuid,
    leader_id: Uuid,
    bus_port: u16,
) -> String {
    format!(
        r#"# ─── VOID ORCHESTRATION PROTOCOL ────────────────────────────────
# You are running inside Void, an infinite canvas terminal emulator
# with built-in swarm intelligence. Your terminal ID is: {terminal_id}
# Your role: WORKER
# Your team: {team_name}
# Group ID: {group_id}
# Leader: {leader_id}
# Bus port: {bus_port}
#
# Available commands (use void-ctl):
#   void-ctl task list --owner me
#   void-ctl task update $TASK_ID --status in_progress
#   void-ctl task update $TASK_ID --status completed --result "summary"
#   void-ctl message send {leader_id} "message text"
#   void-ctl message list
#   void-ctl context get key
#   void-ctl context set key value
# ─────────────────────────────────────────────────────────────────
"#
    )
}
