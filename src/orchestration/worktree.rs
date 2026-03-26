// src/orchestration/worktree.rs — Git worktree isolation for agents

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use uuid::Uuid;

#[allow(dead_code)]
pub struct WorktreeManager {
    /// Base directory for worktrees.
    base_dir: PathBuf,

    /// Mapping: terminal_id -> worktree path.
    worktrees: HashMap<Uuid, PathBuf>,
}

#[allow(dead_code)]
impl WorktreeManager {
    pub fn new() -> Self {
        let base_dir = std::env::temp_dir().join("void-worktrees");
        std::fs::create_dir_all(&base_dir).ok();
        Self {
            base_dir,
            worktrees: HashMap::new(),
        }
    }

    /// Create a worktree for a terminal. Returns the worktree path.
    pub fn create(
        &mut self,
        terminal_id: Uuid,
        team_name: &str,
        agent_name: &str,
        repo_root: &Path,
    ) -> Result<PathBuf, String> {
        let branch_name = format!("void/{}/{}", team_name, agent_name);
        let wt_path = self.base_dir.join(team_name).join(agent_name);

        let output = Command::new("git")
            .current_dir(repo_root)
            .args([
                "worktree",
                "add",
                &wt_path.to_string_lossy(),
                "-b",
                &branch_name,
            ])
            .output()
            .map_err(|e| format!("git worktree add failed: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "git worktree add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        self.worktrees.insert(terminal_id, wt_path.clone());
        Ok(wt_path)
    }

    /// Get the worktree path for a terminal.
    pub fn get(&self, terminal_id: Uuid) -> Option<&PathBuf> {
        self.worktrees.get(&terminal_id)
    }

    /// Remove a worktree.
    pub fn remove(&mut self, terminal_id: Uuid, repo_root: &Path) -> Result<(), String> {
        if let Some(wt_path) = self.worktrees.remove(&terminal_id) {
            Command::new("git")
                .current_dir(repo_root)
                .args(["worktree", "remove", &wt_path.to_string_lossy(), "--force"])
                .output()
                .map_err(|e| format!("git worktree remove failed: {}", e))?;
        }
        Ok(())
    }

    /// Merge a worker's branch back to the current branch.
    pub fn merge(
        &self,
        _terminal_id: Uuid,
        repo_root: &Path,
        team_name: &str,
        agent_name: &str,
    ) -> Result<(), String> {
        let branch_name = format!("void/{}/{}", team_name, agent_name);

        let output = Command::new("git")
            .current_dir(repo_root)
            .args(["merge", &branch_name, "--no-edit"])
            .output()
            .map_err(|e| format!("git merge failed: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Merge conflict: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }

    /// Clean up all worktrees for a team.
    pub fn cleanup_team(&mut self, team_name: &str, repo_root: &Path) {
        let team_dir = self.base_dir.join(team_name);
        let ids_to_remove: Vec<Uuid> = self
            .worktrees
            .iter()
            .filter(|(_, path)| path.starts_with(&team_dir))
            .map(|(id, _)| *id)
            .collect();

        for id in ids_to_remove {
            self.remove(id, repo_root).ok();
        }

        std::fs::remove_dir_all(&team_dir).ok();
    }
}
