//! Backup management for ECU firmware images.
//!
//! Every ECU write operation **must** create a backup first.  This module
//! handles timestamped file creation, SHA-256 integrity hashing, listing,
//! restoring, and verifying backups.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use forge_core::error::DaedalusError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Metadata for a single backup file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    /// Full path to the backup file on disk.
    pub path: PathBuf,
    /// ECU type identifier (e.g. "EDC17C46", "MED17.5.2").
    pub ecu_type: String,
    /// Unix timestamp (seconds since epoch) when the backup was created.
    pub timestamp: u64,
    /// Hex-encoded SHA-256 hash of the file contents.
    pub sha256: String,
    /// File size in bytes.
    pub size: usize,
}

/// Create a backup of the firmware data.
///
/// The file is written to `backup_dir` with a timestamped name:
/// `backup_<ecu_type>_<YYYYMMDD_HHMMSS>.bin`
///
/// Returns a [`Backup`] record with the computed SHA-256 hash.
pub fn create_backup(
    data: &[u8],
    ecu_type: &str,
    backup_dir: &Path,
) -> Result<Backup, DaedalusError> {
    // Ensure the backup directory exists.
    if !backup_dir.exists() {
        std::fs::create_dir_all(backup_dir).map_err(|e| DaedalusError::IoError {
            message: format!("Failed to create backup directory: {e}"),
            path: Some(backup_dir.to_path_buf()),
            source: Some(e),
        })?;
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let timestamp = now.as_secs();

    // Format timestamp as YYYYMMDD_HHMMSS (UTC-like from raw seconds).
    let datetime_str = format_timestamp(timestamp);

    // Sanitise ecu_type for use in filenames (replace spaces/slashes).
    let safe_ecu: String = ecu_type
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
        .collect();

    let filename = format!("backup_{safe_ecu}_{datetime_str}.bin");
    let path = backup_dir.join(&filename);

    // Compute SHA-256 before writing.
    let sha256 = compute_sha256(data);

    // Write the file.
    std::fs::write(&path, data).map_err(|e| DaedalusError::IoError {
        message: format!("Failed to write backup file: {e}"),
        path: Some(path.clone()),
        source: Some(e),
    })?;

    tracing::info!(
        ecu_type,
        path = %path.display(),
        size = data.len(),
        sha256 = %sha256,
        "Backup created"
    );

    Ok(Backup {
        path,
        ecu_type: ecu_type.to_string(),
        timestamp,
        sha256,
        size: data.len(),
    })
}

/// List all backup files in the given directory.
///
/// Files are identified by the `backup_` prefix and `.bin` extension.
/// Returns them sorted newest-first.
pub fn list_backups(backup_dir: &Path) -> Vec<Backup> {
    let mut backups = Vec::new();

    let entries = match std::fs::read_dir(backup_dir) {
        Ok(e) => e,
        Err(_) => return backups,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        if !name.starts_with("backup_") || !name.ends_with(".bin") {
            continue;
        }

        // Parse metadata from filename: backup_<ecu>_<YYYYMMDD_HHMMSS>.bin
        let stem = &name["backup_".len()..name.len() - ".bin".len()];
        let (ecu_type, timestamp) = parse_backup_stem(stem);

        let size = entry.metadata().map(|m| m.len() as usize).unwrap_or(0);

        // Compute SHA-256 of the file on disk.
        let sha256 = match std::fs::read(&path) {
            Ok(data) => compute_sha256(&data),
            Err(_) => String::from("error"),
        };

        backups.push(Backup {
            path,
            ecu_type,
            timestamp,
            sha256,
            size,
        });
    }

    // Sort newest first.
    backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    backups
}

/// Read a backup file and return its contents.
pub fn restore_backup(backup: &Backup) -> Result<Vec<u8>, DaedalusError> {
    std::fs::read(&backup.path).map_err(|e| DaedalusError::IoError {
        message: format!("Failed to read backup file: {e}"),
        path: Some(backup.path.clone()),
        source: Some(e),
    })
}

/// Verify that the SHA-256 hash of the backup file matches the stored value.
pub fn verify_backup(backup: &Backup) -> Result<bool, DaedalusError> {
    let data = restore_backup(backup)?;
    let actual = compute_sha256(&data);
    Ok(actual == backup.sha256)
}

// ── Internal helpers ─────────────────────────────────────────────────

/// Compute the hex-encoded SHA-256 digest of `data`.
fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    // Format as lowercase hex.
    result
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

/// Format a Unix timestamp as `YYYYMMDD_HHMMSS`.
///
/// Uses a simple manual calculation to avoid pulling in `chrono` just for this.
fn format_timestamp(secs: u64) -> String {
    // Days since epoch, accounting for leap years.
    let secs_per_day: u64 = 86400;
    let mut remaining = secs;

    let time_of_day = remaining % secs_per_day;
    remaining /= secs_per_day;

    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Calculate year/month/day from days since 1970-01-01.
    let mut year: u32 = 1970;
    loop {
        let days_in_year: u64 = if is_leap(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }

    let days_in_months: [u64; 12] = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month: u32 = 1;
    for &days in &days_in_months {
        if remaining < days {
            break;
        }
        remaining -= days;
        month += 1;
    }

    let day = remaining as u32 + 1;

    format!("{year:04}{month:02}{day:02}_{hours:02}{minutes:02}{seconds:02}")
}

/// Check if a year is a leap year.
fn is_leap(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Parse a backup filename stem like `EDC17C46_20260323_143022` into
/// `(ecu_type, timestamp_secs)`.
fn parse_backup_stem(stem: &str) -> (String, u64) {
    // Find the last two `_`-separated segments that look like a date+time.
    // Pattern: <ecu>_YYYYMMDD_HHMMSS
    let parts: Vec<&str> = stem.rsplitn(3, '_').collect();
    if parts.len() >= 3 {
        let time_str = parts[0]; // HHMMSS
        let date_str = parts[1]; // YYYYMMDD
        let ecu = parts[2].to_string();

        if date_str.len() == 8 && time_str.len() == 6 {
            // We don't reparse to epoch — just store 0 and rely on filesystem.
            // In production, we'd parse properly; for listing, sort by filename.
            let combined = format!("{date_str}{time_str}");
            let ts = combined.parse::<u64>().unwrap_or(0);
            return (ecu, ts);
        }
    }

    (stem.to_string(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn sha256_known() {
        // SHA-256 of empty string = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let hash = compute_sha256(&[]);
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn timestamp_formatting() {
        // 2026-03-23 14:30:22 UTC ≈ 1774291822 (approximate).
        // Use a known epoch instead.
        // 2020-01-01 00:00:00 UTC = 1577836800
        let ts = format_timestamp(1_577_836_800);
        assert_eq!(ts, "20200101_000000");
    }

    #[test]
    fn create_and_verify_backup() {
        let dir = std::env::temp_dir().join("daedalus_test_backup");
        let _ = std::fs::remove_dir_all(&dir);

        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let backup = create_backup(&data, "EDC17C46", &dir).unwrap();

        assert_eq!(backup.size, 4);
        assert_eq!(backup.ecu_type, "EDC17C46");
        assert!(backup.path.exists());

        // Verify.
        let ok = verify_backup(&backup).unwrap();
        assert!(ok);

        // Restore.
        let restored = restore_backup(&backup).unwrap();
        assert_eq!(restored, data);

        // List.
        let list = list_backups(&dir);
        assert!(!list.is_empty());

        // Cleanup.
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_stem() {
        let (ecu, _ts) = parse_backup_stem("EDC17C46_20260323_143022");
        assert_eq!(ecu, "EDC17C46");
    }

    #[test]
    fn sanitize_ecu_type() {
        let dir = std::env::temp_dir().join("daedalus_test_sanitize");
        let _ = std::fs::remove_dir_all(&dir);

        let data = vec![0x00];
        let backup = create_backup(&data, "MED17/5.2 (test)", &dir).unwrap();
        let name = backup.path.file_name().unwrap().to_str().unwrap();
        assert!(!name.contains('/'));
        assert!(!name.contains('('));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
