// src/orchestration/prompt.rs — Coordination prompt generation for agents
//
// These prompts are injected into terminals when orchestration mode is activated.
// They teach AI agents how to use void-ctl for task management, messaging,
// and coordination — mirroring ClawTeam's coordination protocol.

use uuid::Uuid;

/// Build a list of worker IDs/titles for the leader prompt.
pub fn format_worker_list(workers: &[(Uuid, String)]) -> String {
    if workers.is_empty() {
        return "  (no workers yet — use `void-ctl spawn` to add one)".to_string();
    }
    workers
        .iter()
        .enumerate()
        .map(|(i, (id, title))| format!("  {}. {} (ID: {})", i + 1, title, id))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Generate the leader coordination prompt.
#[allow(dead_code)]
pub fn leader_prompt(
    terminal_id: Uuid,
    team_name: &str,
    group_id: Uuid,
    workers: &[(Uuid, String)],
    bus_port: u16,
) -> String {
    let worker_list = format_worker_list(workers);
    let worker_count = workers.len();

    format!(
        r#"

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# VOID ORCHESTRATION PROTOCOL — LEADER
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## Identity
- Terminal ID: {terminal_id}
- Role: LEADER (orchestrator)
- Team: {team_name}
- Group ID: {group_id}
- Bus Port: {bus_port}
- Workers: {worker_count}

## Your Workers
{worker_list}

## Your Responsibilities
1. PLAN — Break the goal into discrete tasks
2. CREATE TASKS — Use void-ctl to create and assign tasks to workers
3. MONITOR — Watch task progress, read worker output
4. COORDINATE — Share context, resolve blockers, send messages
5. COLLECT — Gather results when tasks complete, verify quality

## Task Management Commands
```bash
# Create a task and assign to a worker
void-ctl task create "Implement user auth" --assign <WORKER_ID> --priority 100 --tag backend

# Create dependent tasks (blocked until dependencies complete)
void-ctl task create "Integration tests" --blocked-by <TASK_ID_1>,<TASK_ID_2>

# List all tasks
void-ctl task list

# Check a specific task
void-ctl task get <TASK_ID>

# Wait for all tasks to complete (blocking)
void-ctl task wait --all --timeout 600
```

## Worker Communication Commands
```bash
# List all terminals and their status
void-ctl list

# Read a worker's terminal output (last N lines)
void-ctl read <WORKER_ID> --lines 50

# Send a message to a worker
void-ctl message send <WORKER_ID> "Use JWT tokens, not session cookies"

# Check your messages
void-ctl message list

# Share data via context store (all team members can read)
void-ctl context set api_schema '{{"endpoints": ["/users", "/auth"]}}'
void-ctl context get api_schema
void-ctl context list

# Inject a shell command directly into a worker's terminal
void-ctl send <WORKER_ID> "cargo test"
```

## Spawning New Workers
```bash
# Spawn a new worker terminal (auto-joins your team)
void-ctl spawn
```

## Leader Workflow
1. First, create ALL tasks before workers start (so dependencies are clear)
2. Assign each task to the best worker: `void-ctl task create "..." --assign <ID>`
3. Workers will see their tasks and start working automatically
4. Monitor with: `void-ctl task list` and `void-ctl read <WORKER_ID>`
5. When blocked, message workers: `void-ctl message send <ID> "..."`
6. Share schemas/configs via context: `void-ctl context set key value`
7. Wait for completion: `void-ctl task wait --all`
8. Read results: `void-ctl task list` (check result field)

## Rules
- Always create tasks BEFORE assigning work (so the kanban board shows them)
- Use `void-ctl message send` for coordination, not `void-ctl send` (which injects raw commands)
- Set task results on completion for tracking
- Check worker output before assuming a task succeeded
- Use --blocked-by for task ordering instead of manual sequencing

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

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
        r#"

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# VOID ORCHESTRATION PROTOCOL — WORKER
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## Identity
- Terminal ID: {terminal_id}
- Role: WORKER
- Team: {team_name}
- Group ID: {group_id}
- Leader ID: {leader_id}
- Bus Port: {bus_port}

## Your Task Commands
```bash
# Check your assigned tasks
void-ctl task list --owner me

# Start working on a task
void-ctl task update <TASK_ID> --status in_progress

# Mark a task as completed (always include a result summary)
void-ctl task update <TASK_ID> --status completed --result "Implemented auth with JWT, 12 tests passing"

# Mark a task as failed
void-ctl task update <TASK_ID> --status failed --result "TypeError in auth.rs:42"

# Self-assign an unassigned task
void-ctl task assign <TASK_ID>
```

## Communication Commands
```bash
# Message the leader (ask questions, report blockers)
void-ctl message send {leader_id} "Need clarification: should auth use JWT or sessions?"

# Check for new messages from leader
void-ctl message list

# Read shared context from the team
void-ctl context get api_schema
void-ctl context list

# Share your own context with the team
void-ctl context set auth_endpoint "/api/v1/auth"
```

## Worker Loop Protocol
**IMPORTANT: Follow this loop after receiving your initial task.**

1. Check your tasks: `void-ctl task list --owner me`
2. Pick the highest-priority pending task
3. Mark it in progress: `void-ctl task update <ID> --status in_progress`
4. Do the work
5. When done, commit your changes with a clear message
6. Mark complete: `void-ctl task update <ID> --status completed --result "summary"`
7. Check for new messages: `void-ctl message list`
8. Check for new tasks: `void-ctl task list --owner me`
9. If you have more tasks, go to step 2
10. If no tasks remain, notify the leader:
    `void-ctl message send {leader_id} "All my tasks are complete."`
11. If you're blocked, tell the leader:
    `void-ctl message send {leader_id} "Blocked on <TASK_ID>: need API schema"`

## Rules
- Always update task status (in_progress/completed/failed) — the kanban board shows this
- Always include --result when completing or failing a task
- Message the leader if you need help or are blocked
- Read shared context before starting work: `void-ctl context list`
- Do NOT exit after your first task — keep checking for more work

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

"#
    )
}
