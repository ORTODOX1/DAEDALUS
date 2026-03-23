//! Application-level configuration: AI provider, CAN bus, UI theme, etc.
//!
//! Persisted as `config.json` next to the project (or in the app data dir).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{DaedalusError, Result};

// ── Operating profile ────────────────────────────────────────────────

/// Determines whether the app favours local GPU inference or cloud APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProfileConfig {
    /// Ollama LLM + ONNX map finder.  Zero API calls, full offline.
    LocalFirst,
    /// Claude / OpenAI / Gemini APIs.  No GPU required, needs internet.
    CloudFirst,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self::CloudFirst
    }
}

// ── Main config ──────────────────────────────────────────────────────

/// Root configuration object serialized to / from `config.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Active AI provider name: `"claude"`, `"openai"`, `"gemini"`, `"ollama"`.
    pub ai_provider: String,

    /// Model identifier, e.g. `"claude-sonnet-4-20250514"`, `"gpt-4o"`.
    pub ai_model: String,

    /// UI language code (`"ru"`, `"en"`, `"de"`, …).
    pub language: String,

    /// UI theme (`"dark"`, `"light"`).
    pub theme: String,

    /// CAN bus baud rate in bits/s (typically 250 000 or 500 000).
    pub can_baud_rate: u32,

    /// Create a backup automatically before every binary modification.
    pub auto_backup: bool,

    /// When `true`, hard safety limits are enforced before every write.
    pub safety_checks_enabled: bool,

    /// Operating profile (local-first vs cloud-first).
    pub profile: ProfileConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ai_provider: "claude".into(),
            ai_model: "claude-sonnet-4-20250514".into(),
            language: "ru".into(),
            theme: "dark".into(),
            can_baud_rate: 500_000,
            auto_backup: true,
            safety_checks_enabled: true,
            profile: ProfileConfig::default(),
        }
    }
}

impl AppConfig {
    /// Load configuration from a JSON file on disk.
    ///
    /// Returns [`Default`] values if the file does not exist yet.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            tracing::info!(?path, "Config file not found — using defaults");
            return Ok(Self::default());
        }

        let data = std::fs::read_to_string(path).map_err(|e| DaedalusError::IoError {
            message: format!("Failed to read config: {e}"),
            path: Some(path.to_path_buf()),
            source: Some(e),
        })?;

        let config: Self = serde_json::from_str(&data)?;
        tracing::debug!(?path, "Loaded config");
        Ok(config)
    }

    /// Persist the current configuration to a JSON file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let json =
            serde_json::to_string_pretty(self).map_err(|e| DaedalusError::ParseError {
                message: format!("Failed to serialize config: {e}"),
                source: Some(Box::new(e)),
            })?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DaedalusError::IoError {
                message: format!("Failed to create config directory: {e}"),
                path: Some(parent.to_path_buf()),
                source: Some(e),
            })?;
        }

        std::fs::write(path, json).map_err(|e| DaedalusError::IoError {
            message: format!("Failed to write config: {e}"),
            path: Some(path.to_path_buf()),
            source: Some(e),
        })?;

        tracing::debug!(?path, "Saved config");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn default_values() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.ai_provider, "claude");
        assert_eq!(cfg.can_baud_rate, 500_000);
        assert!(cfg.auto_backup);
        assert!(cfg.safety_checks_enabled);
        assert_eq!(cfg.language, "ru");
        assert_eq!(cfg.theme, "dark");
    }

    #[test]
    fn round_trip_json() {
        let cfg = AppConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.ai_provider, cfg.ai_provider);
        assert_eq!(back.can_baud_rate, cfg.can_baud_rate);
    }

    #[test]
    fn save_and_load() {
        let dir = std::env::temp_dir().join("daedalus_test_config");
        let path = dir.join("config.json");

        let mut cfg = AppConfig::default();
        cfg.language = "en".into();
        cfg.can_baud_rate = 250_000;
        cfg.save(&path).unwrap();

        let loaded = AppConfig::load(&path).unwrap();
        assert_eq!(loaded.language, "en");
        assert_eq!(loaded.can_baud_rate, 250_000);

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_missing_returns_default() {
        let path = PathBuf::from("/nonexistent/config.json");
        let cfg = AppConfig::load(&path).unwrap();
        assert_eq!(cfg.ai_provider, "claude");
    }
}
