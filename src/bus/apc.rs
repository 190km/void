// src/bus/apc.rs
//
// APC escape sequence interception and command handling for the Terminal Bus.
//
// Protocol:
//   Request:  \x1b_VOID;{json_request}\x1b\\
//   Response: \x1b_VOID;{json_response}\x1b\\

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use uuid::Uuid;

use super::TerminalBus;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const APC_START: &[u8] = b"\x1b_VOID;";
const APC_END: u8 = 0x9C; // ST (String Terminator)
const APC_END_ALT: &[u8] = b"\x1b\\"; // ESC \ (alternative ST)

// ---------------------------------------------------------------------------
// APC Command Extraction
// ---------------------------------------------------------------------------

/// Scan a byte buffer for `\x1b_VOID;...ST` sequences.
///
/// Returns (passthrough_bytes, extracted_command_payloads).
/// Handles partial sequences across read boundaries using the accumulator.
pub fn extract_void_commands(data: &[u8], accum: &mut Vec<u8>) -> (Vec<u8>, Vec<String>) {
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

// ---------------------------------------------------------------------------
// APC Command Handling
// ---------------------------------------------------------------------------

/// Parse an APC payload, dispatch to the bus, return the JSON response.
///
/// Payload format: `{"jsonrpc":"2.0","id":1,"method":"list_terminals","params":{}}`
/// Response format: `\x1b_VOID;{"jsonrpc":"2.0","id":1,"result":{...}}\x1b\\`
pub fn handle_bus_command(
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

    let response = dispatch_bus_method(method, params, Some(caller_terminal), bus);

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
pub fn dispatch_bus_method(
    method: &str,
    params: &Value,
    caller_terminal: Option<Uuid>,
    bus: &Arc<Mutex<TerminalBus>>,
) -> Result<Value, (i32, String)> {
    match method {
        "list_terminals" => {
            let bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            let all_terminals = bus.list_terminals();
            // Filter by caller's workspace — only show terminals in the same workspace
            let caller_workspace = caller_terminal
                .and_then(|id| all_terminals.iter().find(|t| t.id == id))
                .map(|t| t.workspace_id);
            let terminals: Vec<_> = if let Some(ws_id) = caller_workspace {
                all_terminals
                    .iter()
                    .filter(|t| t.workspace_id == ws_id)
                    .collect()
            } else {
                all_terminals.iter().collect()
            };
            let list: Vec<Value> = terminals
                .iter()
                .map(|t| {
                    json!({
                        "id": t.id.to_string(),
                        "title": t.title,
                        "alive": t.alive,
                        "workspace_id": t.workspace_id.to_string(),
                        "group_id": t.group_id.map(|g| g.to_string()),
                        "group_name": t.group_name,
                        "role": format!("{:?}", t.role),
                        "status": t.status.label(),
                        "last_output_ms": t.last_output_elapsed_ms,
                        "last_input_ms": t.last_input_elapsed_ms,
                    })
                })
                .collect();
            Ok(json!({ "terminals": list }))
        }

        "get_terminal" => {
            let id_str = params["id"]
                .as_str()
                .ok_or((-32602, "missing 'id' param".to_string()))?;
            let id = Uuid::parse_str(id_str).map_err(|_| (-32602, "invalid UUID".to_string()))?;
            let bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            let info = bus
                .get_terminal(id)
                .ok_or((-32000, format!("terminal not found: {}", id)))?;
            Ok(json!({
                "id": info.id.to_string(),
                "title": info.title,
                "alive": info.alive,
                "workspace_id": info.workspace_id.to_string(),
                "group_id": info.group_id.map(|g| g.to_string()),
                "group_name": info.group_name,
                "role": format!("{:?}", info.role),
                "status": info.status.label(),
                "last_output_ms": info.last_output_elapsed_ms,
                "last_input_ms": info.last_input_elapsed_ms,
            }))
        }

        "inject" => {
            let target_str = params["target"]
                .as_str()
                .ok_or((-32602, "missing 'target' param".to_string()))?;
            let target = Uuid::parse_str(target_str)
                .map_err(|_| (-32602, "invalid target UUID".to_string()))?;
            let command = params["command"]
                .as_str()
                .ok_or((-32602, "missing 'command' param".to_string()))?;
            let raw = params["raw"].as_bool().unwrap_or(false);

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;

            if raw {
                bus.inject_bytes(target, command.as_bytes(), caller_terminal)
                    .map_err(|e| (-32000, e.to_string()))?;
            } else {
                bus.send_command(target, command, caller_terminal)
                    .map_err(|e| (-32000, e.to_string()))?;
            }

            Ok(json!({ "ok": true }))
        }

        "read_output" => {
            let target_str = params["target"]
                .as_str()
                .ok_or((-32602, "missing 'target' param".to_string()))?;
            let target = Uuid::parse_str(target_str)
                .map_err(|_| (-32602, "invalid target UUID".to_string()))?;
            let line_count = params["lines"].as_u64().unwrap_or(50) as usize;
            let source = params["source"].as_str().unwrap_or("scrollback");

            let bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;

            let lines = if source == "screen" {
                bus.read_screen(target)
                    .map_err(|e| (-32000, e.to_string()))?
            } else {
                bus.read_output(target, line_count)
                    .map_err(|e| (-32000, e.to_string()))?
            };

            Ok(json!({
                "lines": lines,
                "total_lines": lines.len(),
            }))
        }

        "wait_idle" => {
            let target_str = params["target"]
                .as_str()
                .ok_or((-32602, "missing 'target' param".to_string()))?;
            let target = Uuid::parse_str(target_str)
                .map_err(|_| (-32602, "invalid target UUID".to_string()))?;
            let timeout_secs = params["timeout_secs"].as_f64().unwrap_or(120.0);
            let quiet_secs = params["quiet_secs"].as_f64().unwrap_or(2.0);

            // Get handle outside the bus lock so we don't hold it during polling
            let handle = {
                let bus = bus
                    .lock()
                    .map_err(|_| (-32007, "lock failed".to_string()))?;
                bus.get_handle(target)
                    .ok_or((-32000, format!("terminal not found: {}", target)))?
            };

            let start = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs_f64(timeout_secs);
            let quiet = std::time::Duration::from_secs_f64(quiet_secs);

            let idle = TerminalBus::wait_idle_handle(&handle, timeout, quiet);
            let elapsed = start.elapsed().as_secs_f64();

            Ok(json!({
                "idle": idle,
                "elapsed_secs": elapsed,
            }))
        }

        "set_status" => {
            let target_str = params["target"]
                .as_str()
                .ok_or((-32602, "missing 'target' param".to_string()))?;
            let target = Uuid::parse_str(target_str)
                .map_err(|_| (-32602, "invalid target UUID".to_string()))?;
            let status_str = params["status"]
                .as_str()
                .ok_or((-32602, "missing 'status' param".to_string()))?;

            let status = match status_str {
                "idle" => super::types::TerminalStatus::Idle,
                "running" => super::types::TerminalStatus::Running {
                    command: params["command"].as_str().map(|s| s.to_string()),
                    started_at: std::time::Instant::now(),
                },
                "waiting" => super::types::TerminalStatus::Waiting {
                    reason: params["reason"].as_str().map(|s| s.to_string()),
                },
                "done" => super::types::TerminalStatus::Done {
                    finished_at: std::time::Instant::now(),
                },
                "error" => super::types::TerminalStatus::Error {
                    message: params["message"]
                        .as_str()
                        .unwrap_or("unknown error")
                        .to_string(),
                    occurred_at: std::time::Instant::now(),
                },
                _ => return Err((-32602, format!("invalid status: {}", status_str))),
            };

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            bus.set_status(target, status, caller_terminal)
                .map_err(|e| (-32006, e.to_string()))?;

            Ok(json!({ "ok": true }))
        }

        "group_create" => {
            let name = params["name"]
                .as_str()
                .ok_or((-32602, "missing 'name' param".to_string()))?;
            let mode = params["mode"].as_str().unwrap_or("orchestrated");
            let caller = caller_terminal.ok_or((-32602, "no caller terminal".to_string()))?;

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;

            let group_id = if mode == "peer" {
                bus.create_peer_group(name, caller)
                    .map_err(|e| (-32000, e.to_string()))?
            } else {
                bus.create_orchestrated_group(name, caller)
                    .map_err(|e| (-32000, e.to_string()))?
            };

            Ok(json!({
                "group_id": group_id.to_string(),
                "name": name,
                "mode": mode,
            }))
        }

        "group_join" => {
            let group_name = params["group"]
                .as_str()
                .ok_or((-32602, "missing 'group' param".to_string()))?;
            let caller = caller_terminal.ok_or((-32602, "no caller terminal".to_string()))?;

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            bus.join_group_by_name(caller, group_name)
                .map_err(|e| (-32000, e.to_string()))?;

            Ok(json!({ "ok": true }))
        }

        "group_leave" => {
            let caller = caller_terminal.ok_or((-32602, "no caller terminal".to_string()))?;

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            bus.leave_group(caller)
                .map_err(|e| (-32000, e.to_string()))?;

            Ok(json!({ "ok": true }))
        }

        "group_dissolve" => {
            let group_name = params["group"]
                .as_str()
                .ok_or((-32602, "missing 'group' param".to_string()))?;

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            let group_id = bus
                .get_group_by_name(group_name)
                .map(|g| g.id)
                .ok_or((-32002, format!("group not found: {}", group_name)))?;
            bus.dissolve_group(group_id);

            Ok(json!({ "ok": true }))
        }

        "group_list" => {
            let bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            let groups = bus.list_groups();
            let list: Vec<Value> = groups
                .iter()
                .map(|g| {
                    let members: Vec<Value> = g
                        .members
                        .iter()
                        .map(|m| {
                            json!({
                                "id": m.terminal_id.to_string(),
                                "title": m.title,
                                "role": format!("{:?}", m.role),
                                "status": m.status.label(),
                                "alive": m.alive,
                            })
                        })
                        .collect();
                    json!({
                        "id": g.id.to_string(),
                        "name": g.name,
                        "mode": g.mode,
                        "orchestrator_id": g.orchestrator_id.map(|o| o.to_string()),
                        "member_count": g.member_count,
                        "members": members,
                    })
                })
                .collect();
            Ok(json!({ "groups": list }))
        }

        "group_broadcast" => {
            let group_name = params["group"]
                .as_str()
                .ok_or((-32602, "missing 'group' param".to_string()))?;
            let command = params["command"]
                .as_str()
                .ok_or((-32602, "missing 'command' param".to_string()))?;
            let caller = caller_terminal.ok_or((-32602, "no caller terminal".to_string()))?;

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            let group_id = bus
                .get_group_by_name(group_name)
                .map(|g| g.id)
                .ok_or((-32002, format!("group not found: {}", group_name)))?;
            let targets = bus
                .broadcast_command(group_id, command, caller)
                .map_err(|e| (-32000, e.to_string()))?;

            Ok(json!({
                "ok": true,
                "targets": targets.iter().map(|t| t.to_string()).collect::<Vec<_>>(),
            }))
        }

        "context_set" => {
            let key = params["key"]
                .as_str()
                .ok_or((-32602, "missing 'key' param".to_string()))?;
            let value = params["value"]
                .as_str()
                .ok_or((-32602, "missing 'value' param".to_string()))?;
            let ttl = params["ttl_secs"]
                .as_f64()
                .map(std::time::Duration::from_secs_f64);
            let caller = caller_terminal.ok_or((-32602, "no caller terminal".to_string()))?;

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            bus.context_set(key, value, caller, ttl)
                .map_err(|e| (-32000, e.to_string()))?;

            Ok(json!({ "ok": true }))
        }

        "context_get" => {
            let key = params["key"]
                .as_str()
                .ok_or((-32602, "missing 'key' param".to_string()))?;

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            let entry = bus.context_get_entry(key);

            match entry {
                Some(e) => Ok(json!({
                    "key": key,
                    "value": e.value,
                    "source": e.source.to_string(),
                    "updated_at": format!("{:?}", e.updated_at),
                })),
                None => Ok(json!({ "key": key, "value": null })),
            }
        }

        "context_list" => {
            let prefix = params["prefix"].as_str().unwrap_or("");

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            let entries = bus.context_list();

            let filtered: Vec<Value> = entries
                .iter()
                .filter(|(k, _)| prefix.is_empty() || k.starts_with(prefix))
                .map(|(k, v)| {
                    json!({
                        "key": k,
                        "value": v.value,
                        "source": v.source.to_string(),
                    })
                })
                .collect();

            Ok(json!({ "entries": filtered }))
        }

        "context_delete" => {
            let key = params["key"]
                .as_str()
                .ok_or((-32602, "missing 'key' param".to_string()))?;

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            let deleted = bus.context_delete(key);

            Ok(json!({ "deleted": deleted }))
        }

        "message_send" => {
            let to_str = params["to"]
                .as_str()
                .ok_or((-32602, "missing 'to' param".to_string()))?;
            let to =
                Uuid::parse_str(to_str).map_err(|_| (-32602, "invalid 'to' UUID".to_string()))?;
            let payload = params["payload"]
                .as_str()
                .ok_or((-32602, "missing 'payload' param".to_string()))?;
            let caller = caller_terminal.ok_or((-32602, "no caller terminal".to_string()))?;

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            bus.send_message(caller, to, payload)
                .map_err(|e| (-32000, e.to_string()))?;

            Ok(json!({ "ok": true }))
        }

        "message_list" => {
            let caller = caller_terminal.ok_or((-32602, "no caller terminal".to_string()))?;

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            let messages = bus.list_messages(caller);

            let list: Vec<Value> = messages
                .iter()
                .map(|(from, payload, time)| {
                    json!({
                        "from": from.to_string(),
                        "payload": payload,
                        "time": format!("{:?}", time),
                    })
                })
                .collect();

            Ok(json!({ "messages": list }))
        }

        "spawn" => {
            let cwd = params["cwd"].as_str().map(|s| s.to_string());
            let title = params["title"].as_str().map(|s| s.to_string());
            let group = params["group"].as_str().map(|s| s.to_string());
            let command = params["command"].as_str().map(|s| s.to_string());

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            bus.pending_spawns.push(super::PendingSpawn {
                group_name: group,
                cwd,
                title,
                command,
            });

            Ok(json!({ "queued": true }))
        }

        "close" => {
            let target_str = params["target"]
                .as_str()
                .ok_or((-32602, "missing 'target' param".to_string()))?;
            let target = Uuid::parse_str(target_str)
                .map_err(|_| (-32602, "invalid 'target' UUID".to_string()))?;

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            bus.pending_closes.push(target);

            Ok(json!({ "queued": true }))
        }

        // ── Task Methods ─────────────────────────────────────────
        "task.create" => {
            let subject = params["subject"]
                .as_str()
                .ok_or((-32602, "missing 'subject' param".to_string()))?;
            let group_id_str = params["group_id"].as_str();
            let caller = caller_terminal.ok_or((-32602, "no caller terminal".to_string()))?;

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;

            // Resolve group_id: from param, or from caller's group
            let group_id = if let Some(gid) = group_id_str {
                Uuid::parse_str(gid).map_err(|_| (-32602, "invalid group_id UUID".to_string()))?
            } else if let Some(gn) = params["group"].as_str() {
                bus.get_group_by_name(gn)
                    .map(|g| g.id)
                    .ok_or((-32002, format!("group not found: {}", gn)))?
            } else {
                // Use caller's group
                bus.list_groups()
                    .iter()
                    .find(|g| g.members.iter().any(|m| m.terminal_id == caller))
                    .map(|g| g.id)
                    .ok_or((-32002, "caller is not in any group".to_string()))?
            };

            let blocked_by: Vec<Uuid> = params["blocked_by"]
                .as_str()
                .map(|s| {
                    s.split(',')
                        .filter_map(|id| Uuid::parse_str(id.trim()).ok())
                        .collect()
                })
                .or_else(|| {
                    params["blocked_by"].as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .filter_map(|s| Uuid::parse_str(s).ok())
                            .collect()
                    })
                })
                .unwrap_or_default();

            let owner = params["owner"]
                .as_str()
                .and_then(|s| Uuid::parse_str(s).ok());
            let priority = params["priority"].as_u64().unwrap_or(100) as u8;
            let tags: Vec<String> = params["tags"]
                .as_str()
                .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
                .or_else(|| {
                    params["tags"].as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect()
                    })
                })
                .unwrap_or_default();
            let description = params["description"].as_str().unwrap_or("");

            let task_id = bus
                .task_create(
                    subject,
                    group_id,
                    caller,
                    blocked_by,
                    owner,
                    priority,
                    tags,
                    description,
                )
                .map_err(|e| (-32000, e.to_string()))?;

            Ok(json!({
                "task_id": task_id.to_string(),
                "subject": subject,
            }))
        }

        "task.update_status" => {
            let task_id_str = params["task_id"]
                .as_str()
                .ok_or((-32602, "missing 'task_id' param".to_string()))?;
            let task_id = Uuid::parse_str(task_id_str)
                .map_err(|_| (-32602, "invalid task_id UUID".to_string()))?;
            let status_str = params["status"]
                .as_str()
                .ok_or((-32602, "missing 'status' param".to_string()))?;
            let status = super::task::TaskStatus::from_str(status_str)
                .ok_or((-32602, format!("invalid status: {}", status_str)))?;
            let result = params["result"].as_str().map(|s| s.to_string());
            let caller = caller_terminal.ok_or((-32602, "no caller terminal".to_string()))?;

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            bus.task_update_status(task_id, status, caller, result)
                .map_err(|e| (-32000, e.to_string()))?;

            Ok(json!({ "ok": true }))
        }

        "task.assign" => {
            let task_id_str = params["task_id"]
                .as_str()
                .ok_or((-32602, "missing 'task_id' param".to_string()))?;
            let task_id = Uuid::parse_str(task_id_str)
                .map_err(|_| (-32602, "invalid task_id UUID".to_string()))?;
            let owner_str = params["owner"]
                .as_str()
                .ok_or((-32602, "missing 'owner' param".to_string()))?;
            let owner = Uuid::parse_str(owner_str)
                .map_err(|_| (-32602, "invalid owner UUID".to_string()))?;
            let caller = caller_terminal.ok_or((-32602, "no caller terminal".to_string()))?;

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            bus.task_assign(task_id, owner, caller)
                .map_err(|e| (-32000, e.to_string()))?;

            Ok(json!({ "ok": true }))
        }

        "task.unassign" => {
            let task_id_str = params["task_id"]
                .as_str()
                .ok_or((-32602, "missing 'task_id' param".to_string()))?;
            let task_id = Uuid::parse_str(task_id_str)
                .map_err(|_| (-32602, "invalid task_id UUID".to_string()))?;
            let caller = caller_terminal.ok_or((-32602, "no caller terminal".to_string()))?;

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            bus.task_unassign(task_id, caller)
                .map_err(|e| (-32000, e.to_string()))?;

            Ok(json!({ "ok": true }))
        }

        "task.delete" => {
            let task_id_str = params["task_id"]
                .as_str()
                .ok_or((-32602, "missing 'task_id' param".to_string()))?;
            let task_id = Uuid::parse_str(task_id_str)
                .map_err(|_| (-32602, "invalid task_id UUID".to_string()))?;
            let caller = caller_terminal.ok_or((-32602, "no caller terminal".to_string()))?;

            let mut bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            bus.task_delete(task_id, caller)
                .map_err(|e| (-32000, e.to_string()))?;

            Ok(json!({ "ok": true }))
        }

        "task.list" => {
            let bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;

            // Resolve group_id
            let group_id = if let Some(gid) = params["group_id"].as_str() {
                Uuid::parse_str(gid).map_err(|_| (-32602, "invalid group_id UUID".to_string()))?
            } else if let Some(gn) = params["group"].as_str() {
                bus.get_group_by_name(gn)
                    .map(|g| g.id)
                    .ok_or((-32002, format!("group not found: {}", gn)))?
            } else {
                let caller = caller_terminal.ok_or((-32602, "no caller terminal".to_string()))?;
                bus.list_groups()
                    .iter()
                    .find(|g| g.members.iter().any(|m| m.terminal_id == caller))
                    .map(|g| g.id)
                    .ok_or((-32002, "caller is not in any group".to_string()))?
            };

            let status_filter = params["status"]
                .as_str()
                .and_then(super::task::TaskStatus::from_str);
            let owner_filter = params["owner"].as_str().and_then(|s| {
                if s == "me" {
                    caller_terminal
                } else {
                    Uuid::parse_str(s).ok()
                }
            });

            let tasks = bus.task_list(group_id, status_filter, owner_filter);
            let list: Vec<Value> = tasks
                .iter()
                .map(|t| {
                    json!({
                        "id": t.id.to_string(),
                        "subject": t.subject,
                        "status": t.status,
                        "owner": t.owner.map(|o| o.to_string()),
                        "owner_title": t.owner_title,
                        "priority": t.priority,
                        "tags": t.tags,
                        "blocked_by": t.blocked_by.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
                        "blocking": t.blocking.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
                        "result": t.result,
                        "elapsed_ms": t.elapsed_ms,
                    })
                })
                .collect();

            Ok(json!({ "tasks": list }))
        }

        "task.get" => {
            let task_id_str = params["task_id"]
                .as_str()
                .ok_or((-32602, "missing 'task_id' param".to_string()))?;
            let task_id = Uuid::parse_str(task_id_str)
                .map_err(|_| (-32602, "invalid task_id UUID".to_string()))?;

            let bus = bus
                .lock()
                .map_err(|_| (-32007, "lock failed".to_string()))?;
            let info = bus
                .task_get(task_id)
                .ok_or((-32000, format!("task not found: {}", task_id)))?;

            Ok(json!({
                "id": info.id.to_string(),
                "subject": info.subject,
                "description": info.description,
                "status": info.status,
                "owner": info.owner.map(|o| o.to_string()),
                "owner_title": info.owner_title,
                "group_id": info.group_id.to_string(),
                "group_name": info.group_name,
                "created_by": info.created_by.to_string(),
                "blocked_by": info.blocked_by.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
                "blocking": info.blocking.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
                "priority": info.priority,
                "tags": info.tags,
                "result": info.result,
                "elapsed_ms": info.elapsed_ms,
            }))
        }

        _ => Err((-32601, format!("method not found: {}", method))),
    }
}
