// Auto-register the void:// protocol handler on first launch.
// Checks if already registered before doing anything.

/// Register the void:// URL scheme handler if not already present.
pub fn ensure_registered() {
    #[cfg(target_os = "windows")]
    register_windows();

    #[cfg(target_os = "linux")]
    register_linux();

    #[cfg(target_os = "macos")]
    register_macos();
}

#[cfg(target_os = "windows")]
fn register_windows() {
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    // Check if already registered
    let check = Command::new("reg")
        .args([
            "query",
            "HKCU\\Software\\Classes\\void\\shell\\open\\command",
        ])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output();
    if check.is_ok_and(|o| o.status.success()) {
        return;
    }

    let exe = match std::env::current_exe() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return,
    };

    let command_value = format!("\"{}\" \"%1\"", exe);

    let entries: &[(&str, &str, &str)] = &[
        ("HKCU\\Software\\Classes\\void", "", "URL:Void Protocol"),
        ("HKCU\\Software\\Classes\\void", "URL Protocol", ""),
        (
            "HKCU\\Software\\Classes\\void\\shell\\open\\command",
            "",
            &command_value,
        ),
    ];

    for (key, name, value) in entries {
        let mut args = vec!["add", key];
        if name.is_empty() {
            args.extend(["/ve", "/d", value, "/f"]);
        } else {
            args.extend(["/v", name, "/d", value, "/f"]);
        }
        let _ = Command::new("reg")
            .args(&args)
            .creation_flags(0x08000000)
            .output();
    }

    log::info!("Registered void:// protocol handler (Windows)");
}

#[cfg(target_os = "linux")]
fn register_linux() {
    use std::process::Command;

    let Some(app_dir) = dirs_path("applications") else {
        return;
    };
    let desktop_path = app_dir.join("void-terminal.desktop");

    // Already registered
    if desktop_path.exists() {
        return;
    }

    let exe = match std::env::current_exe() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return,
    };

    let desktop_content = format!(
        "[Desktop Entry]\n\
         Name=Void Terminal\n\
         Comment=Infinite canvas terminal emulator\n\
         Exec={} %u\n\
         Icon=void\n\
         Terminal=false\n\
         Type=Application\n\
         Categories=System;TerminalEmulator;\n\
         MimeType=x-scheme-handler/void;\n",
        exe
    );

    let _ = std::fs::create_dir_all(&app_dir);
    if std::fs::write(&desktop_path, desktop_content).is_err() {
        return;
    }

    let _ = Command::new("xdg-mime")
        .args(["default", "void-terminal.desktop", "x-scheme-handler/void"])
        .output();

    let _ = Command::new("update-desktop-database")
        .arg(&app_dir)
        .output();

    log::info!("Registered void:// protocol handler (Linux)");
}

#[cfg(target_os = "linux")]
fn dirs_path(subdir: &str) -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let xdg = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| format!("{}/.local/share", home));
    Some(std::path::PathBuf::from(xdg).join(subdir))
}

#[cfg(target_os = "macos")]
fn register_macos() {
    use std::process::Command;

    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return,
    };

    // Walk up from the binary to find the .app bundle
    let mut app_bundle = None;
    let mut path = exe.as_path();
    for _ in 0..4 {
        if let Some(parent) = path.parent() {
            if parent.extension().is_some_and(|ext| ext == "app") {
                app_bundle = Some(parent.to_path_buf());
                break;
            }
            path = parent;
        }
    }

    let Some(bundle) = app_bundle else {
        return; // Not running from .app bundle (dev build)
    };

    // Check if already registered by querying LaunchServices
    let check = Command::new("defaults")
        .args([
            "read",
            "com.apple.LaunchServices/com.apple.launchservices.secure",
        ])
        .output();
    if check.is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains("x-scheme-handler/void")) {
        return;
    }

    let lsregister = "/System/Library/Frameworks/CoreServices.framework\
        /Frameworks/LaunchServices.framework/Support/lsregister";
    let _ = Command::new(lsregister)
        .args(["-R", "-f"])
        .arg(&bundle)
        .output();

    log::info!("Registered void:// protocol handler (macOS)");
}
