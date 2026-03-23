//! DTC domain types for OBD-II and J1939 diagnostics.

use serde::{Deserialize, Serialize};

// ── Severity / status ────────────────────────────────────────────────

/// How critical a DTC is for vehicle operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DTCSeverity {
    Info,
    Warning,
    Critical,
}

/// Current lifecycle state of a stored DTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DTCStatus {
    /// Currently triggering.
    Active,
    /// Stored in ECU memory but condition no longer present.
    Stored,
    /// Detected once, waiting for confirmation cycle.
    Pending,
    /// Was present but has been cleared by a scan tool.
    Cleared,
}

// ── OBD-II ───────────────────────────────────────────────────────────

/// A standard OBD-II five-character DTC (e.g. "P0420").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OBD2Code {
    /// The alphanumeric code, e.g. `"P0420"`.
    pub code: String,
    /// Short human-readable title.
    pub name: String,
    /// Detailed description of the fault.
    pub description: String,
    /// Functional category, e.g. `"emissions"`, `"fuel_system"`.
    pub category: String,
    /// Criticality level.
    pub severity: DTCSeverity,
    /// Runtime status (populated when read from an ECU).
    pub status: DTCStatus,
}

// ── J1939 ────────────────────────────────────────────────────────────

/// A J1939 DTC identified by SPN + FMI pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct J1939Code {
    /// Suspect Parameter Number (19-bit in the standard).
    pub spn: u32,
    /// Failure Mode Indicator (5-bit, 0–31).
    pub fmi: u8,
    /// Short human-readable title.
    pub name: String,
    /// Detailed description.
    pub description: String,
    /// Functional category, e.g. `"fuel_system"`, `"aftertreatment"`.
    pub category: String,
    /// Criticality level.
    pub severity: DTCSeverity,
    /// ECU types where this code is commonly seen.
    pub ecu_types: Vec<String>,
}

// ── Freeze frame ─────────────────────────────────────────────────────

/// Snapshot of engine parameters captured at the moment a DTC was set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeFrame {
    /// Unix timestamp (seconds) when the snapshot was taken.
    pub timestamp: u64,
    /// Engine speed in RPM.
    pub rpm: u16,
    /// Coolant temperature in °C (signed — cold-start can be negative).
    pub coolant_temp: i16,
    /// Fuel rail pressure in bar.
    pub fuel_pressure: u16,
    /// Intake / boost pressure in kPa.
    pub boost_pressure: u16,
    /// Battery voltage in volts.
    pub battery_voltage: f32,
}

// ── Aggregated read result ───────────────────────────────────────────

/// Everything returned by a single DTC-read session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DTCSnapshot {
    /// OBD-II codes read via UDS service 0x19.
    pub codes: Vec<OBD2Code>,
    /// J1939 codes read via DM1 (PGN 0xFECA).
    pub j1939_codes: Vec<J1939Code>,
    /// Freeze frames associated with active DTCs.
    pub freeze_frames: Vec<FreezeFrame>,
    /// Unix timestamp when the read was performed.
    pub read_time: u64,
}

impl DTCSnapshot {
    /// Create an empty snapshot with the current time.
    pub fn empty(read_time: u64) -> Self {
        Self {
            codes: Vec::new(),
            j1939_codes: Vec::new(),
            freeze_frames: Vec::new(),
            read_time,
        }
    }

    /// Total number of DTCs across both protocols.
    pub fn total_count(&self) -> usize {
        self.codes.len() + self.j1939_codes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_serde_lowercase() {
        let json = serde_json::to_string(&DTCSeverity::Critical).unwrap();
        assert_eq!(json, "\"critical\"");
        let back: DTCSeverity = serde_json::from_str(&json).unwrap();
        assert_eq!(back, DTCSeverity::Critical);
    }

    #[test]
    fn snapshot_total_count() {
        let mut snap = DTCSnapshot::empty(1_000_000);
        assert_eq!(snap.total_count(), 0);

        snap.codes.push(OBD2Code {
            code: "P0420".into(),
            name: "Catalyst".into(),
            description: "Below threshold".into(),
            category: "emissions".into(),
            severity: DTCSeverity::Warning,
            status: DTCStatus::Active,
        });
        snap.j1939_codes.push(J1939Code {
            spn: 100,
            fmi: 3,
            name: "Oil Pressure".into(),
            description: "Voltage high".into(),
            category: "lubrication".into(),
            severity: DTCSeverity::Critical,
            ecu_types: vec!["EDC17".into()],
        });
        assert_eq!(snap.total_count(), 2);
    }
}
