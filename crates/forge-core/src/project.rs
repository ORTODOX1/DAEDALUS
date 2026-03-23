//! Project management: create, open, save, undo/redo.
//!
//! Every tuning session is wrapped in a [`Project`] that tracks the ECU
//! binary, extracted maps, backups, and the full edit history (for undo).

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{DaedalusError, Result};
use crate::types::ECUInfo;

// ── File classification ──────────────────────────────────────────────

/// Semantic role of a file inside the project directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileType {
    /// Raw ECU dump / flash image.
    Binary,
    /// Exported or imported calibration map.
    Map,
    /// JSON / YAML configuration (e.g. checksum definitions).
    Config,
    /// Automatic pre-write backup.
    Backup,
}

/// A single file registered in the project manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    /// Path relative to the project root.
    pub path: PathBuf,
    /// Display name.
    pub name: String,
    /// File size in bytes.
    pub size: u64,
    /// Semantic role.
    pub file_type: FileType,
}

// ── Project ──────────────────────────────────────────────────────────

/// Top-level container for a tuning session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// User-chosen project name.
    pub name: String,
    /// Root directory on disk.
    pub path: PathBuf,
    /// ECU identification (filled after connecting or loading a dump).
    pub ecu_info: Option<ECUInfo>,
    /// Registered project files.
    pub files: Vec<ProjectFile>,
    /// Timestamp of project creation.
    pub created_at: DateTime<Utc>,
    /// Timestamp of the last modification.
    pub modified_at: DateTime<Utc>,
}

impl Project {
    /// Create a new empty project rooted at `path`.
    pub fn new(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        let now = Utc::now();
        Self {
            name: name.into(),
            path: path.into(),
            ecu_info: None,
            files: Vec::new(),
            created_at: now,
            modified_at: now,
        }
    }

    /// Register a file in the project manifest.
    pub fn add_file(&mut self, file: ProjectFile) {
        self.modified_at = Utc::now();
        self.files.push(file);
    }

    /// Remove a file entry by relative path (does **not** delete from disk).
    pub fn remove_file(&mut self, path: &Path) -> Option<ProjectFile> {
        self.modified_at = Utc::now();
        if let Some(idx) = self.files.iter().position(|f| f.path == path) {
            Some(self.files.remove(idx))
        } else {
            None
        }
    }

    /// Persist the project manifest as `project.json` in the project root.
    pub fn save(&self) -> Result<()> {
        let manifest_path = self.path.join("project.json");
        let json = serde_json::to_string_pretty(self)?;

        std::fs::create_dir_all(&self.path).map_err(|e| DaedalusError::IoError {
            message: format!("Cannot create project directory: {e}"),
            path: Some(self.path.clone()),
            source: Some(e),
        })?;

        std::fs::write(&manifest_path, json).map_err(|e| DaedalusError::IoError {
            message: format!("Cannot write project manifest: {e}"),
            path: Some(manifest_path),
            source: Some(e),
        })?;

        tracing::info!(project = %self.name, "Project saved");
        Ok(())
    }

    /// Load a project from its `project.json` manifest.
    pub fn load(project_dir: &Path) -> Result<Self> {
        let manifest_path = project_dir.join("project.json");
        let data = std::fs::read_to_string(&manifest_path).map_err(|e| {
            DaedalusError::ProjectError {
                message: format!("Cannot read project manifest at {}: {e}", manifest_path.display()),
            }
        })?;
        let project: Self = serde_json::from_str(&data)?;
        tracing::info!(project = %project.name, "Project loaded");
        Ok(project)
    }
}

// ── Undo / Redo (Command pattern) ────────────────────────────────────

/// A reversible edit operation on the binary or map data.
///
/// Implementors capture the old state on `execute()` so that `undo()`
/// can restore it.  The trait is object-safe to allow heterogeneous
/// command storage.
pub trait EditCommand: Send + Sync {
    /// Apply the edit, returning an error if the operation is invalid.
    fn execute(&mut self) -> Result<()>;
    /// Reverse the edit, restoring the previous state.
    fn undo(&mut self) -> Result<()>;
    /// Short human-readable label for the undo/redo UI.
    fn description(&self) -> &str;
}

/// Linear undo/redo stack using the command pattern.
///
/// When a new command is pushed after one or more undos, the "future"
/// commands are discarded (standard behaviour).
pub struct CommandHistory {
    commands: Vec<Box<dyn EditCommand>>,
    /// Points to the command that would be *undone* next.
    /// Equal to `commands.len()` when nothing has been undone.
    current: usize,
}

impl CommandHistory {
    /// Create an empty history.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            current: 0,
        }
    }

    /// Execute `cmd` and push it onto the history stack.
    ///
    /// Any previously-undone commands beyond the cursor are discarded.
    pub fn push(&mut self, mut cmd: Box<dyn EditCommand>) -> Result<()> {
        cmd.execute()?;
        // Discard redo tail.
        self.commands.truncate(self.current);
        self.commands.push(cmd);
        self.current = self.commands.len();
        Ok(())
    }

    /// Undo the most recent command.  Returns `false` if there is
    /// nothing to undo.
    pub fn undo(&mut self) -> Result<bool> {
        if self.current == 0 {
            return Ok(false);
        }
        self.current -= 1;
        self.commands[self.current].undo()?;
        Ok(true)
    }

    /// Redo the last undone command.  Returns `false` if there is
    /// nothing to redo.
    pub fn redo(&mut self) -> Result<bool> {
        if self.current >= self.commands.len() {
            return Ok(false);
        }
        self.commands[self.current].execute()?;
        self.current += 1;
        Ok(true)
    }

    /// `true` when at least one command can be undone.
    pub fn can_undo(&self) -> bool {
        self.current > 0
    }

    /// `true` when at least one command can be redone.
    pub fn can_redo(&self) -> bool {
        self.current < self.commands.len()
    }

    /// Number of executed (non-undone) commands.
    pub fn len(&self) -> usize {
        self.current
    }

    /// `true` when the history is empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Description of the command that would be undone next, if any.
    pub fn undo_description(&self) -> Option<&str> {
        if self.current > 0 {
            Some(self.commands[self.current - 1].description())
        } else {
            None
        }
    }

    /// Description of the command that would be redone next, if any.
    pub fn redo_description(&self) -> Option<&str> {
        if self.current < self.commands.len() {
            Some(self.commands[self.current].description())
        } else {
            None
        }
    }
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trivial command for testing: adds / removes a value from a Vec.
    struct PushCommand {
        buf: *mut Vec<u8>,
        value: u8,
    }

    // SAFETY: only used in single-threaded tests.
    unsafe impl Send for PushCommand {}
    unsafe impl Sync for PushCommand {}

    impl EditCommand for PushCommand {
        fn execute(&mut self) -> Result<()> {
            // SAFETY: exclusive test access.
            unsafe { (*self.buf).push(self.value) };
            Ok(())
        }
        fn undo(&mut self) -> Result<()> {
            unsafe { (*self.buf).pop() };
            Ok(())
        }
        fn description(&self) -> &str {
            "push value"
        }
    }

    #[test]
    fn undo_redo_cycle() {
        let mut buf: Vec<u8> = Vec::new();
        let mut history = CommandHistory::new();

        assert!(!history.can_undo());
        assert!(!history.can_redo());

        // Push two values.
        history
            .push(Box::new(PushCommand {
                buf: &mut buf,
                value: 10,
            }))
            .unwrap();
        history
            .push(Box::new(PushCommand {
                buf: &mut buf,
                value: 20,
            }))
            .unwrap();
        assert_eq!(buf, vec![10, 20]);
        assert!(history.can_undo());
        assert!(!history.can_redo());

        // Undo one.
        assert!(history.undo().unwrap());
        assert_eq!(buf, vec![10]);
        assert!(history.can_redo());

        // Redo.
        assert!(history.redo().unwrap());
        assert_eq!(buf, vec![10, 20]);

        // Undo two.
        assert!(history.undo().unwrap());
        assert!(history.undo().unwrap());
        assert!(buf.is_empty());
        assert!(!history.undo().unwrap()); // nothing left

        // Push new command discards redo tail.
        history
            .push(Box::new(PushCommand {
                buf: &mut buf,
                value: 99,
            }))
            .unwrap();
        assert_eq!(buf, vec![99]);
        assert!(!history.can_redo());
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn descriptions() {
        let mut buf: Vec<u8> = Vec::new();
        let mut history = CommandHistory::new();

        assert!(history.undo_description().is_none());
        assert!(history.redo_description().is_none());

        history
            .push(Box::new(PushCommand {
                buf: &mut buf,
                value: 1,
            }))
            .unwrap();
        assert_eq!(history.undo_description(), Some("push value"));
        history.undo().unwrap();
        assert_eq!(history.redo_description(), Some("push value"));
    }

    #[test]
    fn project_new_and_add_file() {
        let mut project = Project::new("test_project", "/tmp/test_project");
        assert_eq!(project.files.len(), 0);
        assert!(project.ecu_info.is_none());

        project.add_file(ProjectFile {
            path: PathBuf::from("dump.bin"),
            name: "dump.bin".into(),
            size: 2_097_152,
            file_type: FileType::Binary,
        });

        assert_eq!(project.files.len(), 1);
        assert_eq!(project.files[0].name, "dump.bin");

        let removed = project.remove_file(Path::new("dump.bin"));
        assert!(removed.is_some());
        assert_eq!(project.files.len(), 0);
    }

    #[test]
    fn project_save_and_load() {
        let dir = std::env::temp_dir().join("daedalus_test_project");
        let _ = std::fs::remove_dir_all(&dir);

        let mut project = Project::new("Test ECU", &dir);
        project.add_file(ProjectFile {
            path: PathBuf::from("flash.bin"),
            name: "flash.bin".into(),
            size: 1024,
            file_type: FileType::Binary,
        });
        project.save().unwrap();

        let loaded = Project::load(&dir).unwrap();
        assert_eq!(loaded.name, "Test ECU");
        assert_eq!(loaded.files.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
