// void-ctl — CLI to control Void terminals via the Terminal Bus.
//
// Communicates with the bus via a local TCP connection.
// Requires VOID_TERMINAL_ID and VOID_BUS_PORT env vars (auto-set by Void).

use std::env;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
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

    let port = env::var("VOID_BUS_PORT").unwrap_or_else(|_| {
        eprintln!("error: VOID_BUS_PORT not set. Is the Void bus server running?");
        process::exit(1);
    });

    let mut client = VoidClient::new(&terminal_id, &port);

    match args[1].as_str() {
        "list" => cmd_list(&mut client, &args[2..]),
        "send" => cmd_send(&mut client, &args[2..]),
        "read" => cmd_read(&mut client, &args[2..]),
        "wait-idle" => cmd_wait_idle(&mut client, &args[2..]),
        "status" => cmd_status(&mut client, &args[2..]),
        "group" => cmd_group(&mut client, &args[2..]),
        "context" => cmd_context(&mut client, &args[2..]),
        "message" => cmd_message(&mut client, &args[2..]),
        "task" => cmd_task(&mut client, &args[2..]),
        "spawn" => cmd_spawn(&mut client, &args[2..]),
        "close" => cmd_close(&mut client, &args[2..]),
        "help" | "--help" | "-h" => print_usage(),
        _ => {
            eprintln!("unknown command: {}", args[1]);
            print_usage();
            process::exit(1);
        }
    }
}

struct VoidClient {
    terminal_id: String,
    stream: TcpStream,
    reader: BufReader<TcpStream>,
    next_id: u64,
}

impl VoidClient {
    fn new(terminal_id: &str, port: &str) -> Self {
        let addr = format!("127.0.0.1:{port}");
        let stream = TcpStream::connect(&addr).unwrap_or_else(|e| {
            eprintln!("error: cannot connect to bus at {addr}: {e}");
            process::exit(1);
        });
        let reader = BufReader::new(stream.try_clone().unwrap());
        Self {
            terminal_id: terminal_id.to_string(),
            stream,
            reader,
            next_id: 1,
        }
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;

        // Add caller terminal ID to params
        let mut full_params = params.clone();
        if let Value::Object(ref mut map) = full_params {
            map.insert("_caller".to_string(), json!(self.terminal_id));
        }

        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": full_params,
        });

        writeln!(self.stream, "{}", request).map_err(|e| format!("write: {e}"))?;

        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .map_err(|e| format!("read: {e}"))?;

        let resp: Value = serde_json::from_str(&line).map_err(|e| format!("parse: {e}"))?;

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

fn cmd_list(client: &mut VoidClient, _args: &[String]) {
    let result = client
        .call("list_terminals", json!({}))
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            process::exit(1);
        });

    let empty = vec![];
    let terminals = result["terminals"].as_array().unwrap_or(&empty);

    if terminals.is_empty() {
        println!("No terminals registered.");
        return;
    }

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
            t["role"].as_str().unwrap_or("standalone"),
            t["status"].as_str().unwrap_or("-"),
        );
    }
}

fn cmd_send(client: &mut VoidClient, args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: void-ctl send <target-id> <command>");
        process::exit(1);
    }
    if args.len() < 2 {
        eprintln!("usage: void-ctl send <target-id> <command>");
        process::exit(1);
    }
    let target = &args[0];
    let command = args[1..].join(" ");
    client
        .call("inject", json!({"target": target, "command": command}))
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            process::exit(1);
        });
    println!("Sent.");
}

fn cmd_read(client: &mut VoidClient, args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: void-ctl read <target-id> [--lines N]");
        process::exit(1);
    }
    let target = &args[0];
    let lines: u64 = args
        .iter()
        .position(|a| a == "--lines")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);

    let result = client
        .call("read_output", json!({"target": target, "lines": lines}))
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            process::exit(1);
        });

    if let Some(output_lines) = result["lines"].as_array() {
        for line in output_lines {
            println!("{}", line.as_str().unwrap_or(""));
        }
    }
}

fn cmd_wait_idle(client: &mut VoidClient, args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: void-ctl wait-idle <target-id> [--timeout N]");
        process::exit(1);
    }
    let target = &args[0];
    let timeout: u64 = args
        .iter()
        .position(|a| a == "--timeout")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    let result = client
        .call(
            "wait_idle",
            json!({"target": target, "timeout_secs": timeout}),
        )
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            process::exit(1);
        });

    if result["idle"].as_bool().unwrap_or(false) {
        println!("Terminal idle.");
    } else {
        println!("Timeout reached.");
        process::exit(2);
    }
}

fn cmd_status(client: &mut VoidClient, args: &[String]) {
    if args.len() < 2 {
        eprintln!("usage: void-ctl status <target-id> <idle|running|done|error>");
        process::exit(1);
    }
    client
        .call(
            "set_status",
            json!({"target": &args[0], "status": &args[1]}),
        )
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            process::exit(1);
        });
    println!("Status updated.");
}

fn cmd_group(client: &mut VoidClient, args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: void-ctl group <create|join|leave|dissolve|list>");
        process::exit(1);
    }
    match args[0].as_str() {
        "create" => {
            if args.len() < 2 {
                eprintln!("usage: void-ctl group create <name>");
                process::exit(1);
            }
            let result = client
                .call(
                    "group_create",
                    json!({"name": &args[1], "mode": "orchestrated"}),
                )
                .unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    process::exit(1);
                });
            println!("Created group \"{}\".", &args[1]);
            let _ = result;
        }
        "join" => {
            if args.len() < 2 {
                eprintln!("usage: void-ctl group join <name>");
                process::exit(1);
            }
            client
                .call("group_join", json!({"group": &args[1]}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    process::exit(1);
                });
            println!("Joined group \"{}\".", &args[1]);
        }
        "leave" => {
            client.call("group_leave", json!({})).unwrap_or_else(|e| {
                eprintln!("error: {e}");
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
                    eprintln!("error: {e}");
                    process::exit(1);
                });
            println!("Group dissolved.");
        }
        "list" => {
            let result = client.call("group_list", json!({})).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                process::exit(1);
            });
            let empty = vec![];
            let groups = result["groups"].as_array().unwrap_or(&empty);
            if groups.is_empty() {
                println!("No groups.");
            } else {
                for g in groups {
                    println!(
                        "  {} ({}, {} members)",
                        g["name"].as_str().unwrap_or("?"),
                        g["mode"].as_str().unwrap_or("?"),
                        g["member_count"].as_u64().unwrap_or(0),
                    );
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
        eprintln!("usage: void-ctl context <set|get|list|delete>");
        process::exit(1);
    }
    match args[0].as_str() {
        "set" => {
            if args.len() < 3 {
                eprintln!("usage: void-ctl context set <key> <value>");
                process::exit(1);
            }
            client
                .call("context_set", json!({"key": &args[1], "value": &args[2]}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {e}");
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
                    eprintln!("error: {e}");
                    process::exit(1);
                });
            if result["value"].is_null() {
                eprintln!("Key not found.");
                process::exit(1);
            }
            print!("{}", result["value"].as_str().unwrap_or(""));
        }
        "list" => {
            let result = client.call("context_list", json!({})).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                process::exit(1);
            });
            if let Some(entries) = result["entries"].as_array() {
                for entry in entries {
                    println!(
                        "{} = {}",
                        entry["key"].as_str().unwrap_or("?"),
                        entry["value"].as_str().unwrap_or("?")
                    );
                }
            }
        }
        "delete" => {
            if args.len() < 2 {
                eprintln!("usage: void-ctl context delete <key>");
                process::exit(1);
            }
            client
                .call("context_delete", json!({"key": &args[1]}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    process::exit(1);
                });
            println!("Deleted.");
        }
        _ => {
            eprintln!("unknown context command: {}", args[0]);
            process::exit(1);
        }
    }
}

fn cmd_message(client: &mut VoidClient, args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: void-ctl message <send|list>");
        process::exit(1);
    }
    match args[0].as_str() {
        "send" => {
            if args.len() < 3 {
                eprintln!("usage: void-ctl message send <target-id> <payload>");
                process::exit(1);
            }
            client
                .call(
                    "message_send",
                    json!({"to": &args[1], "payload": args[2..].join(" ")}),
                )
                .unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    process::exit(1);
                });
            println!("Sent.");
        }
        "list" => {
            let result = client.call("message_list", json!({})).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                process::exit(1);
            });
            if let Some(messages) = result["messages"].as_array() {
                if messages.is_empty() {
                    println!("No messages.");
                } else {
                    for msg in messages {
                        println!(
                            "[from {}] {}",
                            msg["from"].as_str().unwrap_or("?"),
                            msg["payload"].as_str().unwrap_or("?"),
                        );
                    }
                }
            }
        }
        _ => {
            eprintln!("unknown message command: {}", args[0]);
            process::exit(1);
        }
    }
}

fn cmd_task(client: &mut VoidClient, args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: void-ctl task <create|list|update|assign|unassign|delete|get>");
        process::exit(1);
    }
    match args[0].as_str() {
        "create" => {
            if args.len() < 2 {
                eprintln!("usage: void-ctl task create <subject> [options]");
                process::exit(1);
            }
            let subject = &args[1];
            let mut params = json!({"subject": subject});
            let map = params.as_object_mut().unwrap();

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--group" if i + 1 < args.len() => {
                        map.insert("group".into(), json!(&args[i + 1]));
                        i += 2;
                    }
                    "--blocked-by" if i + 1 < args.len() => {
                        map.insert("blocked_by".into(), json!(&args[i + 1]));
                        i += 2;
                    }
                    "--assign" if i + 1 < args.len() => {
                        map.insert("owner".into(), json!(&args[i + 1]));
                        i += 2;
                    }
                    "--assign-self" => {
                        let tid = env::var("VOID_TERMINAL_ID").unwrap_or_default();
                        map.insert("owner".into(), json!(tid));
                        i += 1;
                    }
                    "--priority" if i + 1 < args.len() => {
                        let p: u64 = args[i + 1].parse().unwrap_or(100);
                        map.insert("priority".into(), json!(p));
                        i += 2;
                    }
                    "--tag" if i + 1 < args.len() => {
                        map.insert("tags".into(), json!(&args[i + 1]));
                        i += 2;
                    }
                    "--description" if i + 1 < args.len() => {
                        map.insert("description".into(), json!(&args[i + 1]));
                        i += 2;
                    }
                    _ => {
                        i += 1;
                    }
                }
            }

            let result = client.call("task.create", params).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                process::exit(1);
            });
            println!(
                "Created task {}: {}",
                result["task_id"].as_str().unwrap_or("?"),
                subject
            );
        }
        "list" => {
            let mut params = json!({});
            let map = params.as_object_mut().unwrap();

            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--group" if i + 1 < args.len() => {
                        map.insert("group".into(), json!(&args[i + 1]));
                        i += 2;
                    }
                    "--status" if i + 1 < args.len() => {
                        map.insert("status".into(), json!(&args[i + 1]));
                        i += 2;
                    }
                    "--owner" if i + 1 < args.len() => {
                        map.insert("owner".into(), json!(&args[i + 1]));
                        i += 2;
                    }
                    "--json" => {
                        map.insert("_json".into(), json!(true));
                        i += 1;
                    }
                    _ => {
                        i += 1;
                    }
                }
            }

            let json_output = params.get("_json").is_some();
            let _ = params.as_object_mut().unwrap().remove("_json");

            let result = client.call("task.list", params).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                process::exit(1);
            });

            let empty = vec![];
            let tasks = result["tasks"].as_array().unwrap_or(&empty);

            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                );
                return;
            }

            if tasks.is_empty() {
                println!("No tasks.");
                return;
            }

            println!(
                "{:<10} {:<14} {:<30} {:<8}",
                "ID", "STATUS", "SUBJECT", "PRIORITY"
            );
            println!("{}", "-".repeat(65));
            for t in tasks {
                let id = t["id"].as_str().unwrap_or("?");
                let short_id = if id.len() > 8 { &id[..8] } else { id };
                println!(
                    "{:<10} {:<14} {:<30} {:<8}",
                    short_id,
                    t["status"].as_str().unwrap_or("-"),
                    truncate(t["subject"].as_str().unwrap_or("-"), 30),
                    t["priority"].as_u64().unwrap_or(0),
                );
            }
        }
        "update" => {
            if args.len() < 2 {
                eprintln!(
                    "usage: void-ctl task update <task_id> --status <status> [--result <text>]"
                );
                process::exit(1);
            }
            let task_id = &args[1];
            let mut params = json!({"task_id": task_id});
            let map = params.as_object_mut().unwrap();

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--status" if i + 1 < args.len() => {
                        map.insert("status".into(), json!(&args[i + 1]));
                        i += 2;
                    }
                    "--result" if i + 1 < args.len() => {
                        map.insert("result".into(), json!(&args[i + 1]));
                        i += 2;
                    }
                    _ => {
                        i += 1;
                    }
                }
            }

            client
                .call("task.update_status", params)
                .unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    process::exit(1);
                });
            println!("Task {} updated.", task_id);
        }
        "assign" => {
            if args.len() < 2 {
                eprintln!("usage: void-ctl task assign <task_id> [--to <terminal_id>]");
                process::exit(1);
            }
            let task_id = &args[1];
            let owner = args
                .iter()
                .position(|a| a == "--to")
                .and_then(|i| args.get(i + 1))
                .map(|s| s.as_str())
                .unwrap_or(&client.terminal_id);

            client
                .call("task.assign", json!({"task_id": task_id, "owner": owner}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    process::exit(1);
                });
            println!("Task {} assigned.", task_id);
        }
        "unassign" => {
            if args.len() < 2 {
                eprintln!("usage: void-ctl task unassign <task_id>");
                process::exit(1);
            }
            client
                .call("task.unassign", json!({"task_id": &args[1]}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    process::exit(1);
                });
            println!("Task {} unassigned.", &args[1]);
        }
        "delete" => {
            if args.len() < 2 {
                eprintln!("usage: void-ctl task delete <task_id>");
                process::exit(1);
            }
            client
                .call("task.delete", json!({"task_id": &args[1]}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    process::exit(1);
                });
            println!("Task {} deleted.", &args[1]);
        }
        "get" => {
            if args.len() < 2 {
                eprintln!("usage: void-ctl task get <task_id>");
                process::exit(1);
            }
            let result = client
                .call("task.get", json!({"task_id": &args[1]}))
                .unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    process::exit(1);
                });
            println!(
                "{}",
                serde_json::to_string_pretty(&result).unwrap_or_default()
            );
        }
        _ => {
            eprintln!("unknown task command: {}", args[0]);
            process::exit(1);
        }
    }
}

fn cmd_spawn(client: &mut VoidClient, _args: &[String]) {
    let result = client
        .call("spawn", json!({"count": 1}))
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            process::exit(1);
        });
    println!(
        "{}",
        serde_json::to_string_pretty(&result).unwrap_or_default()
    );
}

fn cmd_close(client: &mut VoidClient, args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: void-ctl close <target-id>");
        process::exit(1);
    }
    client
        .call("close", json!({"target": &args[0]}))
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            process::exit(1);
        });
    println!("Closed.");
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

fn print_usage() {
    println!("void-ctl — control Void terminals from the command line");
    println!();
    println!("USAGE: void-ctl <command> [args...]");
    println!();
    println!("COMMANDS:");
    println!("  list                          List all terminals");
    println!("  send <id> <command>           Send command to terminal");
    println!("  read <id> [--lines N]         Read terminal output");
    println!("  wait-idle <id> [--timeout N]  Wait for terminal idle");
    println!("  status <id> <status>          Set terminal status");
    println!("  group create|join|leave|list  Group management");
    println!("  task create|list|update|...   Task management");
    println!("  context set|get|list|delete   Shared key-value store");
    println!("  message send|list             Direct messaging");
    println!("  spawn                         Spawn new terminal");
    println!("  close <id>                    Close a terminal");
    println!();
    println!("ENVIRONMENT:");
    println!("  VOID_TERMINAL_ID  This terminal's UUID (auto-set)");
    println!("  VOID_BUS_PORT     Bus server port (auto-set)");
}
