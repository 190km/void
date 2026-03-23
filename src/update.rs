use std::sync::{Arc, Mutex};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASES_URL: &str = "https://api.github.com/repos/190km/void/releases/latest";

#[derive(Clone)]
pub struct UpdateState {
    pub latest_version: Option<String>,
    pub release_url: Option<String>,
    pub update_available: bool,
}

pub struct UpdateChecker {
    state: Arc<Mutex<UpdateState>>,
}

impl UpdateChecker {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(UpdateState {
            latest_version: None,
            release_url: None,
            update_available: false,
        }));

        // Check in background thread
        let state_clone = Arc::clone(&state);
        std::thread::spawn(move || {
            if let Some(result) = check_latest_release() {
                if let Ok(mut s) = state_clone.lock() {
                    *s = result;
                }
            }
        });

        Self { state }
    }

    pub fn state(&self) -> UpdateState {
        self.state.lock().unwrap().clone()
    }
}

fn check_latest_release() -> Option<UpdateState> {
    let resp = ureq::get(RELEASES_URL)
        .set("User-Agent", "void-terminal")
        .call()
        .ok()?;

    let json: serde_json::Value = resp.into_json().ok()?;
    let tag = json.get("tag_name")?.as_str()?;
    let url = json.get("html_url")?.as_str()?;

    let latest = tag.strip_prefix('v').unwrap_or(tag);
    let update_available = version_newer(latest, CURRENT_VERSION);

    Some(UpdateState {
        latest_version: Some(latest.to_string()),
        release_url: Some(url.to_string()),
        update_available,
    })
}

/// Returns true if `latest` is strictly newer than `current`.
fn version_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> { v.split('.').filter_map(|s| s.parse().ok()).collect() };
    let l = parse(latest);
    let c = parse(current);
    l > c
}
