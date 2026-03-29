// Single-instance IPC via TCP loopback + lockfile discovery.
//
// The first Void instance binds a TCP listener on 127.0.0.1 (OS-assigned port)
// and writes the port to a lockfile. Subsequent launches read the lockfile,
// send the void:// URL to the running instance, and exit.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

/// Pending deep-link URL received from another instance.
pub type PendingUrl = Arc<Mutex<Option<String>>>;

/// IPC server that listens for incoming void:// URLs from secondary instances.
pub struct IpcServer {
    pending: PendingUrl,
    lock_path: PathBuf,
    // Keep the listener alive so the port stays bound.
    _listener: TcpListener,
}

impl IpcServer {
    /// Start the IPC server. Binds a random port and writes it to the lockfile.
    /// Returns `None` if the data directory cannot be determined.
    pub fn start(ctx: egui::Context) -> Option<Self> {
        let lock_path = lock_file_path()?;

        let listener = TcpListener::bind("127.0.0.1:0").ok()?;
        let port = listener.local_addr().ok()?.port();

        // Write port to lockfile
        if let Some(parent) = lock_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&lock_path, port.to_string());

        let pending: PendingUrl = Arc::new(Mutex::new(None));
        let pending_clone = Arc::clone(&pending);

        let accept_listener = listener.try_clone().expect("failed to clone TcpListener");
        thread::spawn(move || {
            for stream in accept_listener.incoming().flatten() {
                if let Some(url) = read_url(stream) {
                    if let Ok(mut guard) = pending_clone.lock() {
                        *guard = Some(url);
                    }
                    ctx.request_repaint();
                }
            }
        });

        Some(Self {
            pending,
            lock_path,
            _listener: listener,
        })
    }

    /// Take the pending URL if one has arrived.
    pub fn take_pending(&self) -> Option<String> {
        self.pending.lock().ok()?.take()
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

/// Try to send a URL to a running Void instance. Returns `true` if successful.
pub fn try_send_to_running(url: &str) -> bool {
    let Some(lock_path) = lock_file_path() else {
        return false;
    };
    let Ok(port_str) = std::fs::read_to_string(&lock_path) else {
        return false;
    };
    let Ok(port) = port_str.trim().parse::<u16>() else {
        // Corrupt lockfile — remove it
        let _ = std::fs::remove_file(&lock_path);
        return false;
    };

    // Try to connect with a short timeout
    let addr = format!("127.0.0.1:{port}");
    let Ok(mut stream) =
        TcpStream::connect_timeout(&addr.parse().unwrap(), std::time::Duration::from_secs(2))
    else {
        // Stale lockfile — server not running
        let _ = std::fs::remove_file(&lock_path);
        return false;
    };

    // Send the URL followed by a newline
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(2)));
    if writeln!(stream, "{url}").is_err() {
        return false;
    }
    let _ = stream.flush();
    true
}

fn read_url(stream: TcpStream) -> Option<String> {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let trimmed = line.trim().to_string();
    if trimmed.starts_with("void://") {
        Some(trimmed)
    } else {
        None
    }
}

fn lock_file_path() -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "void")?;
    Some(dirs.data_dir().join("void.lock"))
}
