use std::sync::{Arc, Mutex};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASES_URL: &str = "https://api.github.com/repos/190km/void/releases/latest";
const REQUEST_TIMEOUT: u64 = 15; // seconds

#[derive(Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Checking,
    UpToDate,
    Available,
    Downloading,
    Ready,
    Installing,
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

    /// Download the update asset in background, then mark as Ready.
    pub fn download(&self) {
        let state = Arc::clone(&self.state);
        let ctx = self.ctx.clone();

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
                Some(url) => match download_asset(&url) {
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

    /// Install the update and restart the app. Cross-platform.
    pub fn install_and_restart(&self) {
        let installer_path = {
            let s = self.state.lock().unwrap();
            s.installer_path.clone()
        };

        let Some(installer_path) = installer_path else {
            return;
        };

        if let Ok(mut s) = self.state.lock() {
            s.status = UpdateStatus::Installing;
        }
        self.ctx.request_repaint();

        let current_exe = std::env::current_exe().unwrap_or_default();
        let pid = std::process::id();

        #[cfg(target_os = "windows")]
        {
            install_windows(pid, &installer_path, &current_exe);
        }
        #[cfg(target_os = "macos")]
        {
            install_macos(pid, &installer_path, &current_exe);
        }
        #[cfg(target_os = "linux")]
        {
            install_linux(pid, &installer_path, &current_exe);
        }
    }
}

// ── Platform-specific installers ─────────────────────────────────────────

#[cfg(target_os = "windows")]
fn install_windows(pid: u32, installer: &str, exe: &std::path::Path) {
    let script_path = std::env::temp_dir().join("void_update.cmd");
    let script = format!(
        r#"@echo off
:wait
tasklist /FI "PID eq {pid}" 2>NUL | find "{pid}" >NUL
if %ERRORLEVEL%==0 (
    timeout /t 1 /nobreak >NUL
    goto wait
)
start /wait "" "{installer}" /S
start "" "{exe}"
del "%~f0"
"#,
        pid = pid,
        installer = installer.replace('/', "\\"),
        exe = exe.display(),
    );

    if std::fs::write(&script_path, &script).is_ok() {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "/min", "", &script_path.to_string_lossy()])
            .spawn();
        std::process::exit(0);
    }
}

#[cfg(target_os = "macos")]
fn install_macos(pid: u32, dmg_path: &str, exe: &std::path::Path) {
    // Resolve the .app bundle path from the current executable
    // e.g. /Applications/Void.app/Contents/MacOS/void → /Applications/Void.app
    let app_bundle = exe
        .ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .map(|p| p.to_path_buf());

    let install_dir = app_bundle
        .as_ref()
        .and_then(|b| b.parent())
        .unwrap_or(std::path::Path::new("/Applications"));

    let script_path = std::env::temp_dir().join("void_update.sh");
    let script = format!(
        r#"#!/bin/bash
# Wait for the app to exit
while kill -0 {pid} 2>/dev/null; do sleep 0.5; done

# Mount DMG
MOUNT_DIR=$(hdiutil attach -nobrowse -noautoopen "{dmg}" 2>/dev/null | tail -1 | awk '{{print $NF}}')
if [ -z "$MOUNT_DIR" ]; then
    echo "Failed to mount DMG"
    exit 1
fi

# Find and copy the .app
APP_NAME=$(ls "$MOUNT_DIR"/*.app 2>/dev/null | head -1)
if [ -n "$APP_NAME" ]; then
    rm -rf "{install_dir}/$(basename "$APP_NAME")"
    cp -R "$APP_NAME" "{install_dir}/"
    # Relaunch
    open "{install_dir}/$(basename "$APP_NAME")"
fi

# Cleanup
hdiutil detach "$MOUNT_DIR" -quiet
rm -f "{dmg}"
rm -f "$0"
"#,
        pid = pid,
        dmg = dmg_path,
        install_dir = install_dir.display(),
    );

    if std::fs::write(&script_path, &script).is_ok() {
        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755));
        }
        let _ = std::process::Command::new("bash").arg(&script_path).spawn();
        std::process::exit(0);
    }
}

#[cfg(target_os = "linux")]
fn install_linux(pid: u32, archive_path: &str, exe: &std::path::Path) {
    // Install to the same directory as the current binary
    let install_dir = exe
        .parent()
        .unwrap_or(std::path::Path::new("/usr/local/bin"));
    let exe_name = exe
        .file_name()
        .unwrap_or(std::ffi::OsStr::new("void"))
        .to_string_lossy();

    let script_path = std::env::temp_dir().join("void_update.sh");
    let script = format!(
        r#"#!/bin/bash
# Wait for the app to exit
while kill -0 {pid} 2>/dev/null; do sleep 0.5; done

# Extract tar.gz to a temp dir
TEMP_EXTRACT=$(mktemp -d)
tar -xzf "{archive}" -C "$TEMP_EXTRACT" 2>/dev/null

# Find the binary (might be in a subdirectory)
NEW_BIN=$(find "$TEMP_EXTRACT" -name "{exe_name}" -type f | head -1)
if [ -z "$NEW_BIN" ]; then
    # Fallback: take first executable
    NEW_BIN=$(find "$TEMP_EXTRACT" -type f -executable | head -1)
fi

if [ -n "$NEW_BIN" ]; then
    cp "$NEW_BIN" "{install_dir}/{exe_name}"
    chmod +x "{install_dir}/{exe_name}"
    # Relaunch
    nohup "{install_dir}/{exe_name}" >/dev/null 2>&1 &
fi

# Cleanup
rm -rf "$TEMP_EXTRACT"
rm -f "{archive}"
rm -f "$0"
"#,
        pid = pid,
        archive = archive_path,
        exe_name = exe_name,
        install_dir = install_dir.display(),
    );

    if std::fs::write(&script_path, &script).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755));
        }
        let _ = std::process::Command::new("bash").arg(&script_path).spawn();
        std::process::exit(0);
    }
}

// ── Shared logic ─────────────────────────────────────────────────────────

fn check_latest_release() -> Result<UpdateState, String> {
    let resp = minreq::get(RELEASES_URL)
        .with_header("User-Agent", "void-terminal")
        .with_header("Accept", "application/vnd.github+json")
        .with_timeout(REQUEST_TIMEOUT)
        .send()
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    if resp.status_code != 200 {
        return Err(format!("GitHub API returned {}", resp.status_code));
    }

    let json: serde_json::Value =
        serde_json::from_str(resp.as_str().map_err(|e| format!("UTF-8 error: {e}"))?)
            .map_err(|e| format!("JSON parse failed: {e}"))?;

    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "No tag_name in response".to_string())?;

    let latest = tag.strip_prefix('v').unwrap_or(tag);
    let update_available = version_newer(latest, CURRENT_VERSION);

    let download_url = find_platform_asset(&json);

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

/// Find the right asset for the current platform + architecture.
/// Asset naming: void-VERSION-ARCH-setup.ext
/// e.g. void-1.0.0-x86_64-setup.exe, void-1.0.0-aarch64-apple-darwin-setup.dmg
fn find_platform_asset(json: &serde_json::Value) -> Option<String> {
    let assets = json.get("assets")?.as_array()?;

    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x86_64"
    };

    assets.iter().find_map(|asset| {
        let name = asset.get("name")?.as_str()?.to_lowercase();
        let url = asset
            .get("browser_download_url")
            .and_then(|u| u.as_str())
            .map(|s| s.to_string());

        // Must contain our architecture and "setup"
        if !name.contains(arch) || !name.contains("setup") {
            return None;
        }

        #[cfg(target_os = "windows")]
        {
            if name.ends_with(".exe") {
                return url;
            }
        }

        #[cfg(target_os = "macos")]
        {
            if name.ends_with(".dmg") && (name.contains("darwin") || name.contains("apple")) {
                return url;
            }
        }

        #[cfg(target_os = "linux")]
        {
            if name.contains("linux") && (name.ends_with(".tar.gz") || name.ends_with(".deb")) {
                // Prefer .tar.gz over .deb for auto-update
                if name.ends_with(".tar.gz") {
                    return url;
                }
            }
        }

        None
    })
}

fn download_asset(url: &str) -> Result<String, String> {
    let resp = minreq::get(url)
        .with_header("User-Agent", "void-terminal")
        .with_timeout(120) // large files need more time
        .send()
        .map_err(|e| format!("Download failed: {e}"))?;

    if resp.status_code != 200 {
        return Err(format!("Download returned HTTP {}", resp.status_code));
    }

    let temp_dir = std::env::temp_dir();
    let filename = url.rsplit('/').next().unwrap_or("void-update");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_comparison() {
        assert!(version_newer("0.0.2", "0.0.1"));
        assert!(version_newer("0.1.0", "0.0.9"));
        assert!(version_newer("1.0.0", "0.9.9"));
        assert!(!version_newer("0.0.1", "0.0.1"));
        assert!(!version_newer("0.0.1", "0.0.2"));
    }
}
