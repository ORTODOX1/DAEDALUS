//! Data recording — captures live ECU parameters into time-series
//! sessions with CSV export support.

use forge_core::{DaedalusError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A single time-stamped sample containing values for all tracked parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveSample {
    /// Timestamp in microseconds since recording start.
    pub timestamp: u64,
    /// Parameter ID -> decoded value.
    pub values: HashMap<String, f64>,
}

/// A recording session that accumulates live samples over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSession {
    /// Timestamp (microseconds) when recording started.
    pub start_time: u64,
    /// Ordered list of parameter IDs being recorded.
    pub parameters: Vec<String>,
    /// Collected samples in chronological order.
    pub samples: Vec<LiveSample>,
    /// Target sample rate in Hz.
    pub sample_rate_hz: u32,
}

impl RecordingSession {
    /// Create a new recording session.
    ///
    /// # Arguments
    /// * `parameters` — parameter IDs to record (defines CSV column order).
    /// * `sample_rate` — target samples per second.
    pub fn new(parameters: Vec<String>, sample_rate: u32) -> Self {
        Self {
            start_time: 0,
            parameters,
            samples: Vec::new(),
            sample_rate_hz: sample_rate,
        }
    }

    /// Add a sample to the session.
    ///
    /// The timestamp is auto-assigned based on sample count and rate
    /// if the `values` map is non-empty.
    pub fn add_sample(&mut self, values: HashMap<String, f64>) {
        let timestamp = if self.samples.is_empty() {
            self.start_time
        } else {
            let interval_us = 1_000_000 / self.sample_rate_hz.max(1) as u64;
            self.samples.last().map_or(0, |s| s.timestamp) + interval_us
        };

        self.samples.push(LiveSample { timestamp, values });
    }

    /// Total recording duration in seconds.
    pub fn duration_secs(&self) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }
        let first = self.samples.first().map_or(0, |s| s.timestamp);
        let last = self.samples.last().map_or(0, |s| s.timestamp);
        (last - first) as f64 / 1_000_000.0
    }

    /// Total number of recorded samples.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Export the recording session to a CSV file.
    ///
    /// CSV format:
    /// ```text
    /// timestamp_us,param1,param2,...
    /// 0,1234.5,98.2,...
    /// 10000,1235.0,98.3,...
    /// ```
    ///
    /// Missing values for a given timestamp are written as empty fields.
    pub fn export_csv(&self, path: &Path) -> Result<()> {
        use std::io::Write;

        let file = std::fs::File::create(path).map_err(|e| DaedalusError::IoError {
            message: format!("failed to create CSV file: {e}"),
            path: Some(path.to_path_buf()),
            source: Some(e),
        })?;

        let mut writer = std::io::BufWriter::new(file);

        // Header row.
        write!(writer, "timestamp_us").map_err(|e| DaedalusError::IoError {
            message: e.to_string(),
            path: Some(path.to_path_buf()),
            source: Some(e),
        })?;

        for param in &self.parameters {
            write!(writer, ",{param}").map_err(|e| DaedalusError::IoError {
                message: e.to_string(),
                path: Some(path.to_path_buf()),
                source: Some(e),
            })?;
        }
        writeln!(writer).map_err(|e| DaedalusError::IoError {
            message: e.to_string(),
            path: Some(path.to_path_buf()),
            source: Some(e),
        })?;

        // Data rows.
        for sample in &self.samples {
            write!(writer, "{}", sample.timestamp).map_err(|e| DaedalusError::IoError {
                message: e.to_string(),
                path: Some(path.to_path_buf()),
                source: Some(e),
            })?;

            for param in &self.parameters {
                if let Some(value) = sample.values.get(param) {
                    write!(writer, ",{value}").map_err(|e| DaedalusError::IoError {
                        message: e.to_string(),
                        path: Some(path.to_path_buf()),
                        source: Some(e),
                    })?;
                } else {
                    write!(writer, ",").map_err(|e| DaedalusError::IoError {
                        message: e.to_string(),
                        path: Some(path.to_path_buf()),
                        source: Some(e),
                    })?;
                }
            }

            writeln!(writer).map_err(|e| DaedalusError::IoError {
                message: e.to_string(),
                path: Some(path.to_path_buf()),
                source: Some(e),
            })?;
        }

        writer.flush().map_err(|e| DaedalusError::IoError {
            message: e.to_string(),
            path: Some(path.to_path_buf()),
            source: Some(e),
        })?;

        tracing::info!(
            path = %path.display(),
            samples = self.samples.len(),
            "recording exported to CSV"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_test_session() -> RecordingSession {
        let params = vec!["rpm".to_string(), "boost".to_string(), "coolant".to_string()];
        let mut session = RecordingSession::new(params, 10); // 10 Hz

        for i in 0..5 {
            let mut values = HashMap::new();
            values.insert("rpm".into(), 800.0 + i as f64 * 100.0);
            values.insert("boost".into(), 100.0 + i as f64 * 5.0);
            values.insert("coolant".into(), 85.0 + i as f64 * 0.5);
            session.add_sample(values);
        }

        session
    }

    #[test]
    fn new_session_is_empty() {
        let session = RecordingSession::new(vec!["rpm".into()], 10);
        assert_eq!(session.sample_count(), 0);
        assert_eq!(session.duration_secs(), 0.0);
    }

    #[test]
    fn add_samples_increments_count() {
        let session = make_test_session();
        assert_eq!(session.sample_count(), 5);
    }

    #[test]
    fn duration_calculated_correctly() {
        let session = make_test_session();
        // 10 Hz = 100_000 us interval, 4 intervals for 5 samples
        let expected = 4.0 * 100_000.0 / 1_000_000.0; // 0.4 seconds
        assert!((session.duration_secs() - expected).abs() < 0.001);
    }

    #[test]
    fn timestamps_are_monotonic() {
        let session = make_test_session();
        for window in session.samples.windows(2) {
            assert!(window[1].timestamp > window[0].timestamp);
        }
    }

    #[test]
    fn export_csv_creates_file() {
        let session = make_test_session();
        let dir = std::env::temp_dir();
        let path = dir.join("daedalus_test_recording.csv");

        session.export_csv(&path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        // Header + 5 data rows
        assert_eq!(lines.len(), 6);
        assert!(lines[0].starts_with("timestamp_us,"));
        assert!(lines[0].contains("rpm"));
        assert!(lines[0].contains("boost"));
        assert!(lines[0].contains("coolant"));

        // First data row should have values
        let first_data: Vec<&str> = lines[1].split(',').collect();
        assert_eq!(first_data.len(), 4); // timestamp + 3 params

        // Clean up.
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn export_csv_handles_missing_values() {
        let params = vec!["a".to_string(), "b".to_string()];
        let mut session = RecordingSession::new(params, 10);

        // Only provide "a", not "b"
        let mut values = HashMap::new();
        values.insert("a".into(), 42.0);
        session.add_sample(values);

        let dir = std::env::temp_dir();
        let path = dir.join("daedalus_test_missing.csv");

        session.export_csv(&path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let data_line = content.lines().nth(1).unwrap();
        // Should have timestamp,42,  (empty for "b")
        assert!(data_line.contains("42"));
        let parts: Vec<&str> = data_line.split(',').collect();
        assert_eq!(parts.len(), 3); // timestamp + a + b
        assert!(parts[2].is_empty()); // "b" is missing

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn export_csv_invalid_path() {
        let session = make_test_session();
        let path = PathBuf::from("/nonexistent/directory/file.csv");
        assert!(session.export_csv(&path).is_err());
    }

    #[test]
    fn session_serialization_roundtrip() {
        let session = make_test_session();
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: RecordingSession = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.sample_count(), 5);
        assert_eq!(deserialized.parameters.len(), 3);
    }
}
