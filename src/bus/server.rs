// TCP server for void-ctl communication.
//
// Windows conpty strips APC escape sequences, so we use a local TCP socket
// as a fallback. The server listens on 127.0.0.1 with an OS-assigned port.
// The port is exposed via VOID_BUS_PORT env var on spawned terminals.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use super::TerminalBus;

/// Start the bus TCP server on localhost with an OS-assigned port.
/// Returns the port number for setting VOID_BUS_PORT env var.
pub fn start_bus_server(bus: Arc<Mutex<TerminalBus>>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind bus server");
    let port = listener.local_addr().unwrap().port();

    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let bus = bus.clone();
                    thread::spawn(move || handle_client(stream, bus));
                }
                Err(e) => {
                    log::debug!("Bus server accept error: {e}");
                }
            }
        }
    });

    log::info!("Bus server listening on 127.0.0.1:{port}");
    port
}

fn handle_client(mut stream: TcpStream, bus: Arc<Mutex<TerminalBus>>) {
    let peer = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_default();
    log::debug!("Bus client connected: {peer}");

    let reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        // Parse the request — expect JSON-RPC format
        let request: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => {
                let err = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {"code": -32700, "message": "parse error"}
                });
                let _ = writeln!(stream, "{}", err);
                continue;
            }
        };

        let id = request["id"].clone();
        let method = request["method"].as_str().unwrap_or("");
        let params = &request["params"];

        // Extract caller terminal ID from params (void-ctl sends it)
        let caller_id = params["_caller"]
            .as_str()
            .and_then(|s| uuid::Uuid::parse_str(s).ok());

        let result = {
            let bus_ref = &bus;
            super::apc::dispatch_bus_method(method, params, caller_id, bus_ref)
        };

        let response = match result {
            Ok(result) => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result,
            }),
            Err((code, message)) => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": code, "message": message},
            }),
        };

        if writeln!(stream, "{}", response).is_err() {
            break;
        }
    }
}
