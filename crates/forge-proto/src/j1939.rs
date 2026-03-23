//! SAE J1939 protocol support for heavy-duty vehicle diagnostics.
//!
//! Decodes Parameter Group Numbers (PGNs) and Suspect Parameter
//! Numbers (SPNs) commonly used in truck and diesel ECU communication.

use serde::{Deserialize, Serialize};

// ── Well-known PGNs ────────────────────────────────────────────────────

/// DM1 — Active Diagnostic Trouble Codes.
pub const PGN_DM1: u32 = 0xFECA;
/// DM2 — Previously Active DTCs.
pub const PGN_DM2: u32 = 0xFECB;
/// Electronic Engine Controller 1 — RPM, torque.
pub const PGN_ENGINE_SPEED: u32 = 0xF004;
/// Fuel Economy — fuel rate, instantaneous economy.
pub const PGN_FUEL_CONSUMPTION: u32 = 0xFEF2;
/// Cruise Control / Vehicle Speed.
pub const PGN_VEHICLE_SPEED: u32 = 0xFEF1;
/// Engine Temperature 1 — coolant temp, fuel temp.
pub const PGN_COOLANT_TEMP: u32 = 0xFEEE;
/// Inlet / Exhaust Conditions 1 — boost pressure, intake temp.
pub const PGN_BOOST_PRESSURE: u32 = 0xFEF6;
/// Engine Fluid Level / Pressure 1 — oil pressure, coolant level.
pub const PGN_OIL_PRESSURE: u32 = 0xFEEF;
/// Vehicle Electrical Power — battery voltage.
pub const PGN_BATTERY_VOLTAGE: u32 = 0xFEF7;
/// Ambient Conditions — ambient air temp, barometric pressure.
pub const PGN_AMBIENT_CONDITIONS: u32 = 0xFEF5;

// ── PGN ────────────────────────────────────────────────────────────────

/// A J1939 Parameter Group Number, extracted from a 29-bit CAN ID.
///
/// CAN ID layout (29-bit extended):
/// ```text
/// [28..26] Priority (3 bits)
/// [25]     Reserved
/// [24]     Data Page
/// [23..16] PDU Format (PF)
/// [15..8]  PDU Specific (PS) — destination or group extension
/// [7..0]   Source Address
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PGN(pub u32);

impl PGN {
    /// Extract the PGN from a 29-bit extended CAN arbitration ID.
    ///
    /// For PDU1 (PF < 240), PS is the destination and not part of PGN.
    /// For PDU2 (PF >= 240), PS is the group extension and IS part of PGN.
    pub fn from_can_id(id: u32) -> Self {
        let pf = ((id >> 16) & 0xFF) as u8;
        let ps = ((id >> 8) & 0xFF) as u8;
        let dp = ((id >> 24) & 0x01) as u32;

        let pgn = if pf < 240 {
            // PDU1: peer-to-peer, PS = destination address (not in PGN)
            (dp << 16) | ((pf as u32) << 8)
        } else {
            // PDU2: broadcast, PS = group extension (part of PGN)
            (dp << 16) | ((pf as u32) << 8) | ps as u32
        };

        PGN(pgn)
    }

    /// Message priority (0 = highest, 7 = lowest).
    pub fn priority_from_id(id: u32) -> u8 {
        ((id >> 26) & 0x07) as u8
    }

    /// Source address from the CAN ID.
    pub fn source_from_id(id: u32) -> u8 {
        (id & 0xFF) as u8
    }

    /// Destination address (only meaningful for PDU1, PF < 240).
    pub fn destination_from_id(id: u32) -> Option<u8> {
        let pf = ((id >> 16) & 0xFF) as u8;
        if pf < 240 {
            Some(((id >> 8) & 0xFF) as u8)
        } else {
            None // broadcast
        }
    }

    /// Raw PGN value.
    pub fn value(self) -> u32 {
        self.0
    }
}

// ── J1939 Message ──────────────────────────────────────────────────────

/// A decoded J1939 message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct J1939Message {
    /// Parameter Group Number.
    pub pgn: PGN,
    /// Source address of the sender.
    pub source: u8,
    /// Destination address (None for broadcast/PDU2).
    pub destination: Option<u8>,
    /// Message payload (up to 8 bytes, or up to 1785 via J1939 transport).
    pub data: Vec<u8>,
    /// Timestamp in microseconds since connection start.
    pub timestamp: u64,
}

impl J1939Message {
    /// Construct a message from a raw 29-bit CAN ID and payload.
    pub fn from_can(id: u32, data: Vec<u8>, timestamp: u64) -> Self {
        Self {
            pgn: PGN::from_can_id(id),
            source: PGN::source_from_id(id),
            destination: PGN::destination_from_id(id),
            data,
            timestamp,
        }
    }
}

// ── SPN Value ──────────────────────────────────────────────────────────

/// A decoded Suspect Parameter Number value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SPNValue {
    /// SPN number.
    pub spn: u32,
    /// Decoded physical value.
    pub value: f64,
    /// Engineering unit (e.g. "rpm", "kPa", "degC").
    pub unit: String,
}

// ── Decoders ───────────────────────────────────────────────────────────

/// Decode engine speed from PGN 0xF004 (EEC1).
///
/// SPN 190, bytes 4-5 (little-endian), resolution 0.125 RPM, offset 0.
/// Returns `None` if the data is too short or the value indicates
/// "not available" (0xFFFF).
pub fn decode_engine_speed(data: &[u8]) -> Option<f64> {
    if data.len() < 6 {
        return None;
    }
    let raw = u16::from_le_bytes([data[3], data[4]]);
    if raw == 0xFFFF {
        return None; // not available
    }
    Some(raw as f64 * 0.125)
}

/// Decode vehicle speed from PGN 0xFEF1 (CCVS).
///
/// SPN 84, bytes 2-3 (little-endian), resolution 1/256 km/h, offset 0.
pub fn decode_vehicle_speed(data: &[u8]) -> Option<f64> {
    if data.len() < 4 {
        return None;
    }
    let raw = u16::from_le_bytes([data[1], data[2]]);
    if raw == 0xFFFF {
        return None;
    }
    Some(raw as f64 / 256.0)
}

/// Decode fuel rate from PGN 0xFEF2 (Fuel Economy).
///
/// SPN 183, bytes 1-2 (little-endian), resolution 0.05 L/h, offset 0.
pub fn decode_fuel_rate(data: &[u8]) -> Option<f64> {
    if data.len() < 3 {
        return None;
    }
    let raw = u16::from_le_bytes([data[0], data[1]]);
    if raw == 0xFFFF {
        return None;
    }
    Some(raw as f64 * 0.05)
}

/// Decode coolant temperature from PGN 0xFEEE (Engine Temp 1).
///
/// SPN 110, byte 1, resolution 1 degC, offset -40.
pub fn decode_coolant_temp(data: &[u8]) -> Option<f64> {
    if data.is_empty() {
        return None;
    }
    let raw = data[0];
    if raw == 0xFF {
        return None;
    }
    Some(raw as f64 - 40.0)
}

/// Decode boost pressure from PGN 0xFEF6 (Inlet/Exhaust Conditions 1).
///
/// SPN 102, byte 2, resolution 2 kPa, offset 0.
pub fn decode_boost_pressure(data: &[u8]) -> Option<f64> {
    if data.len() < 2 {
        return None;
    }
    let raw = data[1];
    if raw == 0xFF {
        return None;
    }
    Some(raw as f64 * 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pgn_from_can_id_pdu2_broadcast() {
        // Example: CAN ID 0x18FEF200
        // Priority = 6, PF = 0xFE (254 >= 240 → PDU2), PS = 0xF2, SA = 0x00
        let id: u32 = 0x18FEF200;
        let pgn = PGN::from_can_id(id);
        assert_eq!(pgn.value(), 0xFEF2);
        assert_eq!(PGN::priority_from_id(id), 6);
        assert_eq!(PGN::source_from_id(id), 0x00);
        assert!(PGN::destination_from_id(id).is_none());
    }

    #[test]
    fn pgn_from_can_id_pdu1_peer() {
        // CAN ID with PF < 240: e.g. 0x18DA00F1
        // Priority=6, PF=0xDA (218 < 240), PS=0x00 (destination), SA=0xF1
        let id: u32 = 0x18DA00F1;
        let pgn = PGN::from_can_id(id);
        assert_eq!(pgn.value(), 0xDA00);
        assert_eq!(PGN::destination_from_id(id), Some(0x00));
        assert_eq!(PGN::source_from_id(id), 0xF1);
    }

    #[test]
    fn decode_engine_speed_valid() {
        // 2000 RPM = 2000 / 0.125 = 16000 = 0x3E80
        let mut data = [0u8; 8];
        data[3] = 0x80;
        data[4] = 0x3E;
        let rpm = decode_engine_speed(&data).unwrap();
        assert!((rpm - 2000.0).abs() < 0.01);
    }

    #[test]
    fn decode_engine_speed_not_available() {
        let mut data = [0u8; 8];
        data[3] = 0xFF;
        data[4] = 0xFF;
        assert!(decode_engine_speed(&data).is_none());
    }

    #[test]
    fn decode_vehicle_speed_valid() {
        // 90 km/h = 90 * 256 = 23040 = 0x5A00
        let mut data = [0u8; 8];
        data[1] = 0x00;
        data[2] = 0x5A;
        let speed = decode_vehicle_speed(&data).unwrap();
        assert!((speed - 90.0).abs() < 0.01);
    }

    #[test]
    fn decode_fuel_rate_valid() {
        // 25.0 L/h = 25.0 / 0.05 = 500 = 0x01F4
        let mut data = [0u8; 8];
        data[0] = 0xF4;
        data[1] = 0x01;
        let rate = decode_fuel_rate(&data).unwrap();
        assert!((rate - 25.0).abs() < 0.01);
    }

    #[test]
    fn decode_coolant_temp_valid() {
        // 90 degC = raw 130 (130 - 40 = 90)
        let data = [130u8];
        let temp = decode_coolant_temp(&data).unwrap();
        assert!((temp - 90.0).abs() < 0.01);
    }

    #[test]
    fn decode_boost_pressure_valid() {
        // 200 kPa = 200 / 2 = 100
        let data = [0x00, 100u8];
        let pressure = decode_boost_pressure(&data).unwrap();
        assert!((pressure - 200.0).abs() < 0.01);
    }

    #[test]
    fn decode_too_short_data() {
        assert!(decode_engine_speed(&[0; 2]).is_none());
        assert!(decode_vehicle_speed(&[0]).is_none());
        assert!(decode_fuel_rate(&[0]).is_none());
        assert!(decode_coolant_temp(&[]).is_none());
        assert!(decode_boost_pressure(&[0]).is_none());
    }

    #[test]
    fn j1939_message_from_can() {
        let msg = J1939Message::from_can(0x18FEF200, vec![0x01, 0x02, 0x03], 12345);
        assert_eq!(msg.pgn.value(), 0xFEF2);
        assert_eq!(msg.source, 0x00);
        assert!(msg.destination.is_none());
        assert_eq!(msg.timestamp, 12345);
    }
}
