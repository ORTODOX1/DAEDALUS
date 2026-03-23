//! Unified error types for the Daedalus platform.
//!
//! All crates re-export [`DaedalusError`] as their primary error type,
//! keeping error handling consistent across the workspace.

use std::path::PathBuf;

/// Convenience alias used throughout the codebase.
pub type Result<T> = std::result::Result<T, DaedalusError>;

/// Top-level error enum covering every failure domain in Daedalus.
#[derive(Debug, thiserror::Error)]
pub enum DaedalusError {
    // ── I/O ──────────────────────────────────────────────────────────
    /// File-system or generic I/O failure.
    #[error("I/O error at {path:?}: {message}")]
    IoError {
        message: String,
        path: Option<PathBuf>,
        #[source]
        source: Option<std::io::Error>,
    },

    // ── Parsing / binary ─────────────────────────────────────────────
    /// Failed to parse a binary, configuration file, or data structure.
    #[error("Parse error: {message}")]
    ParseError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    // ── Protocols (UDS / KWP / J1939) ────────────────────────────────
    /// A diagnostic-protocol-level error (negative response, timeout, etc.).
    #[error("Protocol error: {message} (service 0x{service:02X}, NRC 0x{nrc:02X})")]
    ProtocolError {
        message: String,
        /// UDS/KWP service ID that triggered the error.
        service: u8,
        /// Negative Response Code (0x00 = not applicable).
        nrc: u8,
    },

    // ── Connection / hardware ────────────────────────────────────────
    /// CAN adapter or serial port connection failure.
    #[error("Connection error: {message}")]
    ConnectionError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    // ── Checksum / integrity ─────────────────────────────────────────
    /// Checksum mismatch or correction failure.
    #[error("Checksum error at region 0x{address:08X}: {message}")]
    ChecksumError { message: String, address: u32 },

    // ── AI provider ──────────────────────────────────────────────────
    /// Cloud or local AI provider returned an error.
    #[error("AI provider error ({provider}): {message}")]
    AIError { provider: String, message: String },

    // ── Project management ───────────────────────────────────────────
    /// Project open / save / validation failure.
    #[error("Project error: {message}")]
    ProjectError { message: String },

    // ── Safety ───────────────────────────────────────────────────────
    /// A hard safety limit was violated — the operation MUST be blocked.
    #[error("Safety violation: {message}")]
    SafetyViolation { message: String },

    // ── Serialization ────────────────────────────────────────────────
    /// JSON (de)serialization failure — thin wrapper around `serde_json`.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

// ── Convenient `From` impls ──────────────────────────────────────────

impl From<std::io::Error> for DaedalusError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError {
            message: err.to_string(),
            path: None,
            source: Some(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_displays_path() {
        let err = DaedalusError::IoError {
            message: "file not found".into(),
            path: Some(PathBuf::from("/tmp/test.bin")),
            source: None,
        };
        let msg = err.to_string();
        assert!(msg.contains("/tmp/test.bin"));
        assert!(msg.contains("file not found"));
    }

    #[test]
    fn protocol_error_formats_hex() {
        let err = DaedalusError::ProtocolError {
            message: "security access denied".into(),
            service: 0x27,
            nrc: 0x35,
        };
        let msg = err.to_string();
        assert!(msg.contains("0x27"));
        assert!(msg.contains("0x35"));
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let err: DaedalusError = io_err.into();
        assert!(matches!(err, DaedalusError::IoError { .. }));
    }
}
