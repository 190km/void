/// Describes a known application that can be embedded in Void.
pub struct AppEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub exe_candidates: &'static [&'static str],
    #[allow(dead_code)]
    pub window_class: Option<&'static str>,
    pub window_title_contains: &'static str,
}

pub const APPS: &[AppEntry] = &[
    AppEntry {
        id: "brave",
        name: "Brave Browser",
        icon: "B",
        exe_candidates: &[
            "C:\\Program Files\\BraveSoftware\\Brave-Browser\\Application\\brave.exe",
            "brave",
        ],
        window_class: Some("Chrome_WidgetWin_1"),
        window_title_contains: "Brave",
    },
    AppEntry {
        id: "vscode",
        name: "Visual Studio Code",
        icon: "V",
        exe_candidates: &["code", "C:\\Program Files\\Microsoft VS Code\\Code.exe"],
        window_class: Some("Chrome_WidgetWin_1"),
        window_title_contains: "Visual Studio Code",
    },
];

/// Resolve an exe path, expanding %USERNAME% and checking existence.
pub fn resolve_exe(candidates: &[&str]) -> Option<String> {
    let username = std::env::var("USERNAME").unwrap_or_default();
    for candidate in candidates {
        let expanded = candidate.replace("%USERNAME%", &username);
        let path = std::path::Path::new(&expanded);
        if path.exists() {
            return Some(expanded);
        }
        // Check if it's in PATH
        if which_exists(&expanded) {
            return Some(expanded);
        }
    }
    None
}

fn which_exists(name: &str) -> bool {
    std::process::Command::new("where")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
