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
        std::io::stdout()
            .flush()
            .map_err(|e| format!("flush: {}", e))?;

        // Read APC response from stdin
        // The PTY master injects the response as an APC sequence
        let response_str = read_apc_response().map_err(|e| format!("read response: {}", e))?;

        let resp: Value =
            serde_json::from_str(&response_str).map_err(|e| format!("parse: {}", e))?;

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
    let result = client
        .call("list_terminals", json!({}))
        .unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            process::exit(1);
        });

    let empty = vec![];
    let terminals = result["terminals"].as_array().unwrap_or(&empty);

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
            if t["alive"].as_bool().unwrap_or(false) {
                "yes"
            } else {
                "no"
            },
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
            .call(
                "group_broadcast",
                json!({"group": group, "command": command}),
            )
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
            client.call("group_leave", json!({})).unwrap_or_else(|e| {
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
            let result = client.call("group_list", json!({})).unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                process::exit(1);
            });

            let empty_groups = vec![];
            let groups = result["groups"].as_array().unwrap_or(&empty_groups);
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
            let result = client.call("group_list", json!({})).unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                process::exit(1);
            });

            let empty_g = vec![];
            let groups = result["groups"].as_array().unwrap_or(&empty_g);
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
            let result = client.call("message_list", json!({})).unwrap_or_else(|e| {
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
            "--cwd" => {
                i += 1;
                cwd = Some(args[i].clone());
            }
            "--title" => {
                i += 1;
                title = Some(args[i].clone());
            }
            "--group" => {
                i += 1;
                group = Some(args[i].clone());
            }
            "--count" => {
                i += 1;
                count = args[i].parse().unwrap_or(1);
            }
            _ => {}
        }
        i += 1;
    }

    let mut params = json!({"count": count});
    if let Some(cwd) = cwd {
        params["cwd"] = json!(cwd);
    }
    if let Some(title) = title {
        params["title"] = json!(title);
    }
    if let Some(group) = group {
        params["group"] = json!(group);
    }

    let result = client.call("spawn", params).unwrap_or_else(|e| {
        eprintln!("error: {}", e);
        process::exit(1);
    });

    if let Some(terminals) = result["terminals"].as_array() {
        for t in terminals {
            println!(
                "Spawned: {} ({})",
                t["id"].as_str().unwrap_or("?"),
                t["title"].as_str().unwrap_or("?")
            );
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
