// src/orchestration/template.rs — TOML template engine for orchestration teams

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OrcTemplate {
    pub team: TeamConfig,
    pub leader: AgentConfig,
    #[serde(default)]
    pub worker: Vec<AgentConfig>,
    #[serde(default)]
    pub layout: LayoutConfig,
    #[serde(default)]
    pub kanban: PanelConfig,
    #[serde(default)]
    pub network: PanelConfig,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TeamConfig {
    pub name: String,
    pub mode: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AgentConfig {
    #[serde(default)]
    pub name: String,
    pub title: String,
    #[serde(default = "default_command")]
    pub command: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct LayoutConfig {
    #[serde(default = "default_pattern")]
    pub pattern: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PanelConfig {
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_position")]
    pub position: String,
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            visible: true,
            position: "auto".to_string(),
        }
    }
}

#[allow(dead_code)]
fn default_command() -> String {
    "claude".to_string()
}
#[allow(dead_code)]
fn default_pattern() -> String {
    "star".to_string()
}
#[allow(dead_code)]
fn default_true() -> bool {
    true
}
#[allow(dead_code)]
fn default_position() -> String {
    "auto".to_string()
}

#[allow(dead_code)]
impl OrcTemplate {
    /// Load a template from a TOML file.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read template: {}", e))?;
        toml::from_str(&content).map_err(|e| format!("Failed to parse template: {}", e))
    }

    /// Load a built-in template by name.
    pub fn builtin(name: &str) -> Option<Self> {
        let toml_str = match name {
            "duo" => include_str!("../../templates/duo.toml"),
            "trio" => include_str!("../../templates/trio.toml"),
            "fullstack" => include_str!("../../templates/fullstack.toml"),
            "research" => include_str!("../../templates/research.toml"),
            "hedge-fund" => include_str!("../../templates/hedge-fund.toml"),
            _ => return None,
        };
        toml::from_str(toml_str).ok()
    }

    /// Apply variable substitution.
    pub fn substitute(&mut self, vars: &std::collections::HashMap<String, String>) {
        let sub = |s: &mut String| {
            for (key, val) in vars {
                *s = s.replace(&format!("{{{}}}", key), val);
            }
        };

        sub(&mut self.team.name);
        sub(&mut self.team.description);
        sub(&mut self.leader.prompt);
        for w in &mut self.worker {
            sub(&mut w.prompt);
            sub(&mut w.title);
        }
    }

    /// Total number of agents (leader + workers).
    pub fn agent_count(&self) -> usize {
        1 + self.worker.len()
    }
}
