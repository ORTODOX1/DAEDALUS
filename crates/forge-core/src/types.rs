//! Core domain types shared across all Daedalus crates.
//!
//! Every struct here derives [`serde::Serialize`] + [`serde::Deserialize`]
//! so it can cross the Tauri IPC boundary without boilerplate.

use serde::{Deserialize, Serialize};

// ── ECU identification ───────────────────────────────────────────────

/// Supported ECU platform families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ECUType {
    /// Bosch EDC17 — widespread diesel ECU (Euro 5/6 trucks & cars).
    EDC17,
    /// Bosch MD1 — next-gen diesel/gas ECU (Euro 6d).
    MD1,
    /// Cummins CM2350 — heavy-duty diesel (ISX15, ISB6.7).
    CM2350,
    /// Continental (Siemens) DCM3.7 — PSA / Ford diesel.
    DCM37,
    /// Continental SID309 — light-commercial diesel.
    SID309,
    /// Fallback for unrecognized ECUs.
    Generic,
}

/// High-level vehicle category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VehicleType {
    Truck,
    Car,
    Bus,
    Agriculture,
}

/// Physical / logical protocol used to communicate with the ECU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Protocol {
    /// Background Debug Mode (Motorola/NXP).
    BDM,
    /// JTAG boundary-scan debug.
    JTAG,
    /// ARM Debug Access Port (SWD / coresight).
    DAP,
    /// ISO 9141 / ISO 14230 serial line.
    KLine,
    /// ISO 11898 CAN bus (OBD-II / UDS).
    CAN,
    /// SAE J1939 CAN for heavy-duty vehicles.
    J1939,
}

/// ECU chipset manufacturer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Manufacturer {
    Bosch,
    Delphi,
    Denso,
    Cummins,
    Siemens,
}

/// Full identification block for a connected or loaded ECU.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ECUInfo {
    /// Human-readable ECU label, e.g. "EDC17C46 P_1037".
    pub name: String,
    pub manufacturer: Manufacturer,
    /// Main processor, e.g. "TC1797", "MPC5674F".
    pub processor: String,
    /// Hardware version string read from the ECU.
    pub hw_version: String,
    /// Software / calibration version string.
    pub sw_version: String,
    /// Primary communication protocol.
    pub protocol: Protocol,
    /// Target vehicle category.
    pub vehicle_type: VehicleType,
}

// ── Map / calibration types ──────────────────────────────────────────

/// Semantic classification of a calibration map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MapType {
    RailPressure,
    InjectionTiming,
    PilotQuantity,
    BoostPressure,
    TorqueLimiter,
    EGRRate,
    SpeedLimiter,
    FuelTempComp,
    DPFRegenThreshold,
    AdBlueDosing,
    Generic,
}

/// A located calibration map inside the binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapRegion {
    /// Byte offset (start) in the binary image.
    pub start_addr: u32,
    /// Byte offset (exclusive end).
    pub end_addr: u32,
    /// Semantic type, if classified.
    pub map_type: MapType,
    /// Number of rows (Y-axis breakpoints).
    pub rows: u16,
    /// Number of columns (X-axis breakpoints).
    pub cols: u16,
    /// X-axis physical unit, e.g. "RPM".
    pub x_axis_unit: String,
    /// Y-axis physical unit, e.g. "mg/stroke".
    pub y_axis_unit: String,
    /// Cell value physical unit, e.g. "bar".
    pub value_unit: String,
}

// ── Safety ───────────────────────────────────────────────────────────

/// A hard safety constraint that **blocks** a write when violated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyLimit {
    /// Parameter being constrained, e.g. "rail_pressure".
    pub parameter: String,
    /// Lower acceptable bound.
    pub min_value: f64,
    /// Upper acceptable bound.
    pub max_value: f64,
    /// Physical unit, e.g. "bar", "°BTDC".
    pub unit: String,
    /// Human-readable explanation shown in the UI.
    pub description: String,
}

impl SafetyLimit {
    /// Returns `true` when `value` is within the allowed range (inclusive).
    pub fn is_within(&self, value: f64) -> bool {
        value >= self.min_value && value <= self.max_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safety_limit_boundary() {
        let limit = SafetyLimit {
            parameter: "rail_pressure".into(),
            min_value: 200.0,
            max_value: 2200.0,
            unit: "bar".into(),
            description: "Common-rail max pressure".into(),
        };
        assert!(limit.is_within(200.0));
        assert!(limit.is_within(2200.0));
        assert!(limit.is_within(1500.0));
        assert!(!limit.is_within(199.9));
        assert!(!limit.is_within(2200.1));
    }

    #[test]
    fn ecu_info_round_trip() {
        let info = ECUInfo {
            name: "EDC17C46".into(),
            manufacturer: Manufacturer::Bosch,
            processor: "TC1797".into(),
            hw_version: "H04".into(),
            sw_version: "1037.40.61".into(),
            protocol: Protocol::CAN,
            vehicle_type: VehicleType::Truck,
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: ECUInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "EDC17C46");
        assert!(matches!(back.manufacturer, Manufacturer::Bosch));
    }

    #[test]
    fn map_region_serde() {
        let region = MapRegion {
            start_addr: 0x8_0000,
            end_addr: 0x8_0200,
            map_type: MapType::RailPressure,
            rows: 16,
            cols: 16,
            x_axis_unit: "RPM".into(),
            y_axis_unit: "mg/stroke".into(),
            value_unit: "bar".into(),
        };
        let json = serde_json::to_string(&region).unwrap();
        assert!(json.contains("RailPressure"));
    }
}
