use std::sync::{Arc, Mutex};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASES_URL: &str = "https://api.github.com/repos/190km/void/releases/latest";

#[derive(Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Checking,
    UpToDate,
    Available,
    Downloading,
    Ready,
    Error(String),
}

#[derive(Clone)]
pub struct UpdateState {
    pub latest_version: Option<String>,
    pub download_url: Option<String>,
    pub installer_path: Option<String>,
    pub status: UpdateStatus,
}

pub struct UpdateChecker {
    state: Arc<Mutex<UpdateState>>,
    ctx: egui::Context,
}

impl UpdateChecker {
    pub fn new(ctx: egui::Context) -> Self {
        let state = Arc::new(Mutex::new(UpdateState {
            latest_version: None,
            download_url: None,
            installer_path: None,
            status: UpdateStatus::Checking,
        }));

        let state_clone = Arc::clone(&state);
        let ctx_clone = ctx.clone();
        std::thread::spawn(move || match check_latest_release() {
            Ok(result) => {
                if let Ok(mut s) = state_clone.lock() {
                    *s = result;
                }
                ctx_clone.request_repaint();
            }
            Err(e) => {
                if let Ok(mut s) = state_clone.lock() {
                    s.status = UpdateStatus::Error(e);
                }
                ctx_clone.request_repaint();
            }
        });

        Self { state, ctx }
    }

    pub fn state(&self) -> UpdateState {
        self.state.lock().unwrap().clone()
    }

    /// Download the installer in background, then mark as Ready.
    pub fn download(&self) {
        let state = Arc::clone(&self.state);
        let ctx = self.ctx.clone();

        // Set status to downloading
        if let Ok(mut s) = state.lock() {
            s.status = UpdateStatus::Downloading;
        }
        ctx.request_repaint();

        std::thread::spawn(move || {
            let download_url = {
                let s = state.lock().unwrap();
                s.download_url.clone()
            };

            match download_url {
                Some(url) => match download_installer(&url) {
                    Ok(path) => {
                        if let Ok(mut s) = state.lock() {
                            s.installer_path = Some(path);
                            s.status = UpdateStatus::Ready;
                        }
                        ctx.request_repaint();
                    }
                    Err(e) => {
                        if let Ok(mut s) = state.lock() {
                            s.status = UpdateStatus::Error(e);
                        }
                        ctx.request_repaint();
                    }
                },
                None => {
                    if let Ok(mut s) = state.lock() {
                        s.status = UpdateStatus::Error("No download URL".to_string());
                    }
                    ctx.request_repaint();
                }
            }
        });
    }

    /// Silent install + relaunch: runs the NSIS installer with /S,
    /// waits for it to finish, then relaunches the updated void.exe.
    pub fn install_and_restart(&self) {
        let path = {
            let s = self.state.lock().unwrap();
            s.installer_path.clone()
        };
        if let Some(installer_path) = path {
            // Find where void.exe is currently installed
            let current_exe = std::env::current_exe().unwrap_or_default();

            std::thread::spawn(move || {
                // Run installer silently — /S = silent, no UI
                let _ = std::process::Command::new(&installer_path)
                    .arg("/S")
                    .status(); // blocks until installer finishes

                // Relaunch the app from the same path
                let _ = std::process::Command::new(&current_exe).spawn();

                std::process::exit(0);
            });
        }
    }
}

fn check_latest_release() -> Result<UpdateState, String> {
    let resp = minreq::get(RELEASES_URL)
        .with_header("User-Agent", "void-terminal")
        .with_header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    let json: serde_json::Value =
        serde_json::from_str(resp.as_str().map_err(|e| format!("UTF-8 error: {e}"))?)
            .map_err(|e| format!("JSON parse failed: {e}"))?;

    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("No tag_name in response: {json}"))?;

    let latest = tag.strip_prefix('v').unwrap_or(tag);
    let update_available = version_newer(latest, CURRENT_VERSION);

    // Find the right asset for this platform + architecture
    let platform_suffix = if cfg!(target_os = "windows") {
        if cfg!(target_arch = "aarch64") {
            "windows-arm64.exe"
        } else {
            "windows-x64.exe"
        }
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "macos-arm64.dmg"
        } else {
            "macos-x64.dmg"
        }
    } else if cfg!(target_arch = "aarch64") {
        "linux-arm64.tar.gz"
    } else {
        "linux-x64.tar.gz"
    };

    let download_url = json
        .get("assets")
        .and_then(|a| a.as_array())
        .and_then(|assets| {
            assets.iter().find_map(|asset| {
                let name = asset.get("name")?.as_str()?;
                if name.ends_with(platform_suffix) {
                    asset
                        .get("browser_download_url")
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                }
            })
        });

    Ok(UpdateState {
        latest_version: Some(latest.to_string()),
        download_url,
        installer_path: None,
        status: if update_available {
            UpdateStatus::Available
        } else {
            UpdateStatus::UpToDate
        },
    })
}

fn download_installer(url: &str) -> Result<String, String> {
    let resp = minreq::get(url)
        .with_header("User-Agent", "void-terminal")
        .send()
        .map_err(|e| format!("Download failed: {e}"))?;

    let temp_dir = std::env::temp_dir();
    let filename = url.rsplit('/').next().unwrap_or("void-update.exe");
    let path = temp_dir.join(filename);

    std::fs::write(&path, resp.as_bytes()).map_err(|e| format!("Write failed: {e}"))?;

    Ok(path.to_string_lossy().to_string())
}

/// Returns true if `latest` is strictly newer than `current`.
fn version_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> { v.split('.').filter_map(|s| s.parse().ok()).collect() };
    let l = parse(latest);
    let c = parse(current);
    l > c
}
