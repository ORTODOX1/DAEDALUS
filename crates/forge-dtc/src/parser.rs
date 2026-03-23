//! Parse raw DTC bytes from ECU diagnostic responses.
//!
//! Two protocols are supported:
//! - **OBD-II / UDS**: service 0x19 `ReadDTCInformation` sub-functions.
//! - **J1939 DM1**: PGN 0xFECA active diagnostic trouble codes.
//!
//! The parsers produce domain types from [`crate::types`] ready for
//! display or storage.

use crate::types::{DTCSeverity, DTCStatus, FreezeFrame, J1939Code, OBD2Code};

// ── OBD-II (UDS service 0x19) ────────────────────────────────────────

/// First byte of the OBD-II DTC code: system letter + high nibble.
///
/// Bits 7-6 → letter (P/C/B/U), bits 5-4 → first digit.
fn decode_obd2_prefix(high: u8) -> (char, u8) {
    let letter = match (high >> 6) & 0x03 {
        0 => 'P', // Powertrain
        1 => 'C', // Chassis
        2 => 'B', // Body
        _ => 'U', // Network / undefined
    };
    let digit = (high >> 4) & 0x03;
    (letter, digit)
}

/// Build the 5-character OBD-II code from a two-byte DTC value.
///
/// Layout (ISO 15031-6):
/// ```text
///  Byte 0:  [L1 L0 D1 D0  D2₃ D2₂ D2₁ D2₀]
///  Byte 1:  [D3₃ D3₂ D3₁ D3₀  D4₃ D4₂ D4₁ D4₀]
/// ```
fn format_obd2_code(high: u8, low: u8) -> String {
    let (letter, d1) = decode_obd2_prefix(high);
    let d2 = high & 0x0F;
    let d3 = (low >> 4) & 0x0F;
    let d4 = low & 0x0F;
    format!("{letter}{d1}{d2:X}{d3:X}{d4:X}")
}

/// Map UDS status-byte bits to our [`DTCStatus`].
///
/// Bit 0 = testFailed (active), bit 3 = confirmedDTC, bit 4 = testNotCompletedSinceLastClear.
fn decode_dtc_status(status_byte: u8) -> DTCStatus {
    if status_byte & 0x01 != 0 {
        DTCStatus::Active
    } else if status_byte & 0x08 != 0 {
        DTCStatus::Stored
    } else if status_byte & 0x10 != 0 {
        DTCStatus::Pending
    } else {
        DTCStatus::Cleared
    }
}

/// Parse a UDS `ReadDTCInformation` positive response (service 0x59).
///
/// Expected format for sub-function 0x02 (`reportDTCByStatusMask`):
/// ```text
///  [0x59] [sub] [availability_mask] [DTC_high DTC_mid DTC_low status] ...
/// ```
///
/// Each DTC record is 4 bytes: 3-byte DTC number + 1-byte status.
/// The first 3 bytes of the response (SID echo + sub + mask) are skipped.
///
/// This function works with the raw payload **after** ISO-TP reassembly.
pub fn parse_obd2_response(data: &[u8]) -> Vec<OBD2Code> {
    // Minimum: SID(1) + sub(1) + mask(1) + at least one 4-byte record.
    if data.len() < 7 {
        return Vec::new();
    }

    // Verify positive response SID.
    if data[0] != 0x59 {
        tracing::warn!(sid = data[0], "Unexpected SID in DTC response");
        return Vec::new();
    }

    let records = &data[3..];
    let mut codes = Vec::new();

    for chunk in records.chunks_exact(4) {
        let dtc_high = chunk[0];
        let dtc_low = chunk[1];
        // chunk[2] is the third DTC byte (sub-component / extended info),
        // not used for the standard 5-char code but part of the 3-byte number.
        let status_byte = chunk[3];

        let code_str = format_obd2_code(dtc_high, dtc_low);
        let status = decode_dtc_status(status_byte);

        codes.push(OBD2Code {
            code: code_str,
            name: String::new(),
            description: String::new(),
            category: String::new(),
            severity: DTCSeverity::Info,
            status,
        });
    }

    tracing::debug!(count = codes.len(), "Parsed OBD2 DTCs from UDS response");
    codes
}

// ── J1939 DM1 (PGN 0xFECA) ──────────────────────────────────────────

/// Extract the 19-bit SPN from a 4-byte J1939 DTC record.
///
/// J1939-73 DTC layout (4 bytes):
/// ```text
///  Byte 0: SPN bits 18..11
///  Byte 1: SPN bits 10..3
///  Byte 2: [SPN 2..0 | FMI 4..0]   (bits 7-5 = SPN low, bits 4-0 = FMI)
///  Byte 3: [OC 6..0 | CM]          (occurrence count + conversion method)
/// ```
fn extract_spn(b0: u8, b1: u8, b2: u8) -> u32 {
    let spn_high = (b0 as u32) << 11;
    let spn_mid = (b1 as u32) << 3;
    let spn_low = ((b2 >> 5) & 0x07) as u32;
    spn_high | spn_mid | spn_low
}

/// Extract the 5-bit FMI from byte 2 of a J1939 DTC record.
fn extract_fmi(b2: u8) -> u8 {
    b2 & 0x1F
}

/// Parse a J1939 DM1 message payload (PGN 0xFECA — Active Diagnostic Trouble Codes).
///
/// DM1 format:
/// ```text
///  Byte 0-1:  Lamp status (malfunction indicator, red/amber/protect)
///  Byte 2..N: 4-byte DTC records
/// ```
///
/// If the payload is exactly `[0xFF, 0xFF]` or shorter, no active codes exist.
pub fn parse_j1939_dm1(data: &[u8]) -> Vec<J1939Code> {
    // 2 bytes lamp status + at least one 4-byte DTC record.
    if data.len() < 6 {
        return Vec::new();
    }

    // Check for "no active DTCs" sentinel (all 0xFF).
    if data.iter().all(|&b| b == 0xFF) {
        return Vec::new();
    }

    let records = &data[2..];
    let mut codes = Vec::new();

    for chunk in records.chunks_exact(4) {
        let spn = extract_spn(chunk[0], chunk[1], chunk[2]);
        let fmi = extract_fmi(chunk[2]);

        // SPN 0 + FMI 0 can appear as padding — skip it.
        if spn == 0 && fmi == 0 {
            continue;
        }

        codes.push(J1939Code {
            spn,
            fmi,
            name: String::new(),
            description: String::new(),
            category: String::new(),
            severity: DTCSeverity::Info,
            ecu_types: Vec::new(),
        });
    }

    tracing::debug!(count = codes.len(), "Parsed J1939 DM1 DTCs");
    codes
}

/// Parse a freeze-frame block (UDS service 0x19 sub 0x04 or proprietary).
///
/// This is a simplified parser for the common Bosch EDC17 layout:
/// ```text
///  Bytes 0-1: RPM (big-endian, raw / 4)
///  Bytes 2-3: Coolant temp (signed, °C + 40 offset)
///  Bytes 4-5: Fuel pressure (bar, big-endian)
///  Bytes 6-7: Boost pressure (kPa, big-endian)
///  Bytes 8-9: Battery voltage (raw / 100.0)
///  Bytes 10-13: Timestamp (u32 big-endian, seconds since ECU power-up)
/// ```
pub fn parse_freeze_frame(data: &[u8]) -> Option<FreezeFrame> {
    if data.len() < 14 {
        return None;
    }

    let rpm = u16::from_be_bytes([data[0], data[1]]) / 4;
    let coolant_raw = i16::from_be_bytes([data[2], data[3]]);
    let coolant_temp = coolant_raw - 40;
    let fuel_pressure = u16::from_be_bytes([data[4], data[5]]);
    let boost_pressure = u16::from_be_bytes([data[6], data[7]]);
    let battery_raw = u16::from_be_bytes([data[8], data[9]]);
    let battery_voltage = battery_raw as f32 / 100.0;
    let timestamp = u32::from_be_bytes([data[10], data[11], data[12], data[13]]) as u64;

    Some(FreezeFrame {
        timestamp,
        rpm,
        coolant_temp,
        fuel_pressure,
        boost_pressure,
        battery_voltage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── OBD-II tests ─────────────────────────────────────────────────

    #[test]
    fn format_code_p0420() {
        // P0420 → letter P (00), first digit 0 (00), then 4, 2, 0
        // high byte: 00_00_0100 = 0x04, low byte: 0010_0000 = 0x20
        let code = format_obd2_code(0x04, 0x20);
        assert_eq!(code, "P0420");
    }

    #[test]
    fn format_code_c0300() {
        // C = 01, digit 0 = 00 → high bits = 01_00 = 0x40
        // 3, 0, 0 → high low nibble = 3 → 0x43, low = 0x00
        let code = format_obd2_code(0x43, 0x00);
        assert_eq!(code, "C0300");
    }

    #[test]
    fn parse_obd2_empty() {
        assert!(parse_obd2_response(&[]).is_empty());
        assert!(parse_obd2_response(&[0x59, 0x02, 0xFF]).is_empty());
    }

    #[test]
    fn parse_obd2_single_dtc() {
        // Positive response: SID=0x59, sub=0x02, mask=0xFF, then P0420 active
        let data = [0x59, 0x02, 0xFF, 0x04, 0x20, 0x00, 0x01];
        let codes = parse_obd2_response(&data);
        assert_eq!(codes.len(), 1);
        assert_eq!(codes[0].code, "P0420");
        assert_eq!(codes[0].status, DTCStatus::Active);
    }

    #[test]
    fn parse_obd2_multiple_dtcs() {
        let data = [
            0x59, 0x02, 0xFF, // header
            0x04, 0x20, 0x00, 0x01, // P0420 active
            0x04, 0x01, 0x00, 0x08, // P0401 stored (confirmed)
        ];
        let codes = parse_obd2_response(&data);
        assert_eq!(codes.len(), 2);
        assert_eq!(codes[0].code, "P0420");
        assert_eq!(codes[0].status, DTCStatus::Active);
        assert_eq!(codes[1].code, "P0401");
        assert_eq!(codes[1].status, DTCStatus::Stored);
    }

    #[test]
    fn bad_sid_returns_empty() {
        let data = [0x7F, 0x19, 0x12, 0x00, 0x00, 0x00, 0x00];
        assert!(parse_obd2_response(&data).is_empty());
    }

    // ── J1939 tests ──────────────────────────────────────────────────

    #[test]
    fn extract_spn_fmi() {
        // SPN 91, FMI 3
        // SPN 91 = 0b000_0000_0101_1011
        // b0 = SPN[18:11] = 0x00
        // b1 = SPN[10:3]  = 0b0000_1011 = 0x0B
        // b2 = SPN[2:0]|FMI = 0b011_00011 = 0x63
        let spn = extract_spn(0x00, 0x0B, 0x63);
        let fmi = extract_fmi(0x63);
        assert_eq!(spn, 91);
        assert_eq!(fmi, 3);
    }

    #[test]
    fn extract_large_spn() {
        // SPN 520192 = 0x7F000
        // 0b0_0111_1111_0000_0000_0000_00
        // b0 = [18:11] = 0b11111110 = 0xFE
        // b1 = [10:3]  = 0b00000000 = 0x00
        // b2 = [2:0]|FMI = 0b000_00000 = 0x00  → SPN low = 0, FMI = 0
        let spn = extract_spn(0xFE, 0x00, 0x00);
        assert_eq!(spn, 520192);
    }

    #[test]
    fn parse_dm1_empty() {
        assert!(parse_j1939_dm1(&[]).is_empty());
        assert!(parse_j1939_dm1(&[0xFF, 0xFF]).is_empty());
        assert!(parse_j1939_dm1(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]).is_empty());
    }

    #[test]
    fn parse_dm1_single_code() {
        // Lamp status (2 bytes) + SPN 91 FMI 3 + OC byte
        let mut data = vec![0x00, 0x00]; // lamps off
        data.push(0x00); // SPN[18:11]
        data.push(0x0B); // SPN[10:3]
        data.push(0x63); // SPN[2:0] | FMI
        data.push(0x01); // OC + CM
        let codes = parse_j1939_dm1(&data);
        assert_eq!(codes.len(), 1);
        assert_eq!(codes[0].spn, 91);
        assert_eq!(codes[0].fmi, 3);
    }

    #[test]
    fn parse_dm1_skips_zero_padding() {
        let data = vec![
            0x00, 0x00, // lamps
            0x00, 0x00, 0x00, 0x00, // SPN=0, FMI=0 → padding, skip
        ];
        let codes = parse_j1939_dm1(&data);
        assert!(codes.is_empty());
    }

    // ── Freeze frame tests ───────────────────────────────────────────

    #[test]
    fn parse_freeze_frame_valid() {
        let mut data = Vec::new();
        data.extend_from_slice(&3200u16.to_be_bytes()); // RPM raw = 3200 → /4 = 800
        data.extend_from_slice(&130i16.to_be_bytes()); // coolant = 130 - 40 = 90°C
        data.extend_from_slice(&1800u16.to_be_bytes()); // fuel pressure = 1800 bar
        data.extend_from_slice(&250u16.to_be_bytes()); // boost = 250 kPa
        data.extend_from_slice(&1380u16.to_be_bytes()); // battery = 13.80 V
        data.extend_from_slice(&12345u32.to_be_bytes()); // timestamp

        let ff = parse_freeze_frame(&data).unwrap();
        assert_eq!(ff.rpm, 800);
        assert_eq!(ff.coolant_temp, 90);
        assert_eq!(ff.fuel_pressure, 1800);
        assert_eq!(ff.boost_pressure, 250);
        assert!((ff.battery_voltage - 13.80).abs() < 0.01);
        assert_eq!(ff.timestamp, 12345);
    }

    #[test]
    fn parse_freeze_frame_too_short() {
        assert!(parse_freeze_frame(&[0u8; 13]).is_none());
        assert!(parse_freeze_frame(&[]).is_none());
    }
}
