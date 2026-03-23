//! OBD-II (On-Board Diagnostics) PID encoding and decoding.
//!
//! Supports standard OBD-II modes and the most commonly used PIDs
//! for real-time vehicle data monitoring.

use serde::{Deserialize, Serialize};

// ── OBD-II Modes ───────────────────────────────────────────────────────

/// Standard OBD-II service modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum OBD2Mode {
    /// Mode 01 — current (live) data.
    CurrentData = 0x01,
    /// Mode 02 — freeze frame data (snapshot at DTC time).
    FreezeFrame = 0x02,
    /// Mode 03 — stored DTCs.
    StoredDTC = 0x03,
    /// Mode 04 — clear DTCs and freeze frames.
    ClearDTC = 0x04,
    /// Mode 09 — vehicle information (VIN, calibration IDs, etc.).
    LiveData = 0x09,
}

impl OBD2Mode {
    /// Raw mode byte.
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

// ── PID metadata ───────────────────────────────────────────────────────

/// Metadata for a single OBD-II PID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OBD2PID {
    /// PID number (e.g. 0x0C for RPM).
    pub pid: u8,
    /// Human-readable name.
    pub name: String,
    /// Engineering unit.
    pub unit: String,
    /// Minimum physical value.
    pub min: f64,
    /// Maximum physical value.
    pub max: f64,
}

/// Return metadata for all supported PIDs.
pub fn supported_pids() -> Vec<OBD2PID> {
    vec![
        OBD2PID {
            pid: 0x04,
            name: "Calculated engine load".into(),
            unit: "%".into(),
            min: 0.0,
            max: 100.0,
        },
        OBD2PID {
            pid: 0x05,
            name: "Engine coolant temperature".into(),
            unit: "\u{00B0}C".into(),
            min: -40.0,
            max: 215.0,
        },
        OBD2PID {
            pid: 0x0B,
            name: "Intake manifold absolute pressure".into(),
            unit: "kPa".into(),
            min: 0.0,
            max: 255.0,
        },
        OBD2PID {
            pid: 0x0C,
            name: "Engine RPM".into(),
            unit: "rpm".into(),
            min: 0.0,
            max: 16383.75,
        },
        OBD2PID {
            pid: 0x0D,
            name: "Vehicle speed".into(),
            unit: "km/h".into(),
            min: 0.0,
            max: 255.0,
        },
        OBD2PID {
            pid: 0x11,
            name: "Throttle position".into(),
            unit: "%".into(),
            min: 0.0,
            max: 100.0,
        },
    ]
}

// ── Request builder ────────────────────────────────────────────────────

/// Build a raw OBD-II request payload for a given mode and PID.
///
/// Result is suitable for wrapping in an ISO-TP Single Frame.
pub fn build_obd2_request(mode: OBD2Mode, pid: u8) -> Vec<u8> {
    vec![mode.as_u8(), pid]
}

// ── PID decoder ────────────────────────────────────────────────────────

/// Decode an OBD-II PID response into a physical value.
///
/// `pid` is the PID number, `data` contains the response bytes
/// (A, B, C, D) after the mode+PID echo.
///
/// Returns `None` for unsupported PIDs or insufficient data.
pub fn decode_pid(pid: u8, data: &[u8]) -> Option<f64> {
    match pid {
        // PID 0x04: Calculated engine load
        // Formula: A * 100 / 255
        0x04 => {
            let a = *data.first()? as f64;
            Some(a * 100.0 / 255.0)
        }

        // PID 0x05: Engine coolant temperature
        // Formula: A - 40
        0x05 => {
            let a = *data.first()? as f64;
            Some(a - 40.0)
        }

        // PID 0x0B: Intake manifold absolute pressure
        // Formula: A (kPa)
        0x0B => {
            let a = *data.first()? as f64;
            Some(a)
        }

        // PID 0x0C: Engine RPM
        // Formula: (256*A + B) / 4
        0x0C => {
            if data.len() < 2 {
                return None;
            }
            let a = data[0] as f64;
            let b = data[1] as f64;
            Some((a * 256.0 + b) / 4.0)
        }

        // PID 0x0D: Vehicle speed
        // Formula: A (km/h)
        0x0D => {
            let a = *data.first()? as f64;
            Some(a)
        }

        // PID 0x11: Throttle position
        // Formula: A * 100 / 255
        0x11 => {
            let a = *data.first()? as f64;
            Some(a * 100.0 / 255.0)
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_request_current_data() {
        let req = build_obd2_request(OBD2Mode::CurrentData, 0x0C);
        assert_eq!(req, vec![0x01, 0x0C]);
    }

    #[test]
    fn build_request_freeze_frame() {
        let req = build_obd2_request(OBD2Mode::FreezeFrame, 0x05);
        assert_eq!(req, vec![0x02, 0x05]);
    }

    #[test]
    fn decode_coolant_temp() {
        // A=180 → 180 - 40 = 140 degC
        assert_eq!(decode_pid(0x05, &[180]), Some(140.0));
        // A=40 → 0 degC
        assert_eq!(decode_pid(0x05, &[40]), Some(0.0));
    }

    #[test]
    fn decode_rpm() {
        // A=0x1A, B=0xF8 → (0x1A*256 + 0xF8) / 4 = (6656 + 248) / 4 = 1726.0
        let result = decode_pid(0x0C, &[0x1A, 0xF8]).unwrap();
        assert!((result - 1726.0).abs() < 0.01);
    }

    #[test]
    fn decode_rpm_zero() {
        assert_eq!(decode_pid(0x0C, &[0, 0]), Some(0.0));
    }

    #[test]
    fn decode_speed() {
        assert_eq!(decode_pid(0x0D, &[120]), Some(120.0));
    }

    #[test]
    fn decode_intake_pressure() {
        assert_eq!(decode_pid(0x0B, &[101]), Some(101.0));
    }

    #[test]
    fn decode_engine_load() {
        let result = decode_pid(0x04, &[255]).unwrap();
        assert!((result - 100.0).abs() < 0.01);
    }

    #[test]
    fn decode_throttle() {
        let result = decode_pid(0x11, &[128]).unwrap();
        // 128 * 100 / 255 ≈ 50.196
        assert!((result - 50.196).abs() < 0.01);
    }

    #[test]
    fn decode_unknown_pid() {
        assert!(decode_pid(0xAA, &[0x00]).is_none());
    }

    #[test]
    fn decode_insufficient_data() {
        assert!(decode_pid(0x0C, &[0x1A]).is_none()); // RPM needs 2 bytes
        assert!(decode_pid(0x05, &[]).is_none());
    }

    #[test]
    fn supported_pids_not_empty() {
        let pids = supported_pids();
        assert!(!pids.is_empty());
        assert!(pids.iter().any(|p| p.pid == 0x0C));
    }
}
