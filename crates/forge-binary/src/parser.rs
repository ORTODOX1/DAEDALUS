//! Binary file parsing for Raw, Intel HEX, and Motorola S-Record formats.
//!
//! Entry point: [`BinaryImage::from_file`] auto-detects format.

use std::path::Path;

use forge_core::error::DaedalusError;
use serde::{Deserialize, Serialize};

/// Recognised binary file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryFormat {
    /// Raw flat binary (no address metadata).
    Raw,
    /// Intel HEX (`:LLAAAATT…CC`).
    IntelHex,
    /// Motorola S-Record (`S0`–`S9`).
    MotorolaSrec,
    /// Auto-detect from file content / extension.
    Auto,
}

/// An in-memory firmware image with an optional base address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryImage {
    /// Raw byte content of the image.
    data: Vec<u8>,
    /// Logical start address (0 for raw files).
    base_address: u32,
    /// Format the image was loaded from.
    format: BinaryFormat,
    /// Convenience: `data.len()`.
    size: usize,
}

impl BinaryImage {
    // ── Construction ─────────────────────────────────────────────────

    /// Load a binary image from disk, auto-detecting format by extension
    /// and content when `BinaryFormat::Auto` would be chosen.
    pub fn from_file(path: &Path) -> Result<Self, DaedalusError> {
        let raw = std::fs::read(path).map_err(|e| DaedalusError::IoError {
            message: e.to_string(),
            path: Some(path.to_path_buf()),
            source: Some(e),
        })?;

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        match ext.as_str() {
            "hex" | "ihex" | "ihx" => {
                let text = String::from_utf8(raw).map_err(|e| DaedalusError::ParseError {
                    message: format!("Intel HEX file is not valid UTF-8: {e}"),
                    source: None,
                })?;
                parse_intel_hex(&text)
            }
            "srec" | "s19" | "s28" | "s37" | "mot" => {
                let text = String::from_utf8(raw).map_err(|e| DaedalusError::ParseError {
                    message: format!("S-Record file is not valid UTF-8: {e}"),
                    source: None,
                })?;
                parse_srec(&text)
            }
            _ => {
                // Try to detect by first bytes.
                if raw.first() == Some(&b':') {
                    let text =
                        String::from_utf8(raw).map_err(|e| DaedalusError::ParseError {
                            message: format!("Intel HEX file is not valid UTF-8: {e}"),
                            source: None,
                        })?;
                    parse_intel_hex(&text)
                } else if raw.starts_with(b"S0") || raw.starts_with(b"S1") {
                    let text =
                        String::from_utf8(raw).map_err(|e| DaedalusError::ParseError {
                            message: format!("S-Record file is not valid UTF-8: {e}"),
                            source: None,
                        })?;
                    parse_srec(&text)
                } else {
                    Ok(Self::from_raw(raw))
                }
            }
        }
    }

    /// Wrap a raw byte buffer as a flat binary image (base address 0).
    pub fn from_raw(data: Vec<u8>) -> Self {
        let size = data.len();
        Self {
            data,
            base_address: 0,
            format: BinaryFormat::Raw,
            size,
        }
    }

    // ── Accessors ────────────────────────────────────────────────────

    /// Base (start) address of the image.
    pub fn base_address(&self) -> u32 {
        self.base_address
    }

    /// Detected or declared binary format.
    pub fn format(&self) -> BinaryFormat {
        self.format
    }

    /// Total size in bytes.
    pub fn len(&self) -> usize {
        self.size
    }

    /// Whether the image is empty.
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Direct access to the underlying byte buffer.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Mutable access to the underlying byte buffer.
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    // ── Read helpers ─────────────────────────────────────────────────

    /// Convert a logical address to an index into `self.data`.
    fn addr_to_index(&self, addr: u32) -> Option<usize> {
        if addr < self.base_address {
            return None;
        }
        let idx = (addr - self.base_address) as usize;
        if idx < self.size {
            Some(idx)
        } else {
            None
        }
    }

    /// Read a single byte at a logical address.
    pub fn read_u8(&self, addr: u32) -> Option<u8> {
        self.addr_to_index(addr).map(|i| self.data[i])
    }

    /// Read a big-endian `u16` at a logical address.
    pub fn read_u16_be(&self, addr: u32) -> Option<u16> {
        let i = self.addr_to_index(addr)?;
        if i + 1 >= self.size {
            return None;
        }
        Some(u16::from_be_bytes([self.data[i], self.data[i + 1]]))
    }

    /// Read a little-endian `u16` at a logical address.
    pub fn read_u16_le(&self, addr: u32) -> Option<u16> {
        let i = self.addr_to_index(addr)?;
        if i + 1 >= self.size {
            return None;
        }
        Some(u16::from_le_bytes([self.data[i], self.data[i + 1]]))
    }

    // ── Write helpers ────────────────────────────────────────────────

    /// Write a single byte at a logical address.
    pub fn write_u8(&mut self, addr: u32, value: u8) -> Result<(), DaedalusError> {
        let i = self
            .addr_to_index(addr)
            .ok_or(DaedalusError::ParseError {
                message: format!("Address 0x{addr:08X} out of range"),
                source: None,
            })?;
        self.data[i] = value;
        Ok(())
    }

    /// Write a big-endian `u16` at a logical address.
    pub fn write_u16_be(&mut self, addr: u32, value: u16) -> Result<(), DaedalusError> {
        let i = self
            .addr_to_index(addr)
            .ok_or(DaedalusError::ParseError {
                message: format!("Address 0x{addr:08X} out of range"),
                source: None,
            })?;
        if i + 1 >= self.size {
            return Err(DaedalusError::ParseError {
                message: format!("Address 0x{:08X} + 1 out of range", addr),
                source: None,
            });
        }
        let bytes = value.to_be_bytes();
        self.data[i] = bytes[0];
        self.data[i + 1] = bytes[1];
        Ok(())
    }

    /// Return a byte slice for the logical address range `[start, end)`.
    ///
    /// # Panics
    /// Panics if the range is out of bounds.
    pub fn region(&self, start: u32, end: u32) -> &[u8] {
        let s = (start - self.base_address) as usize;
        let e = (end - self.base_address) as usize;
        &self.data[s..e]
    }
}

// ── Intel HEX parser ─────────────────────────────────────────────────

/// Parse an Intel HEX formatted string into a [`BinaryImage`].
///
/// Supports record types 00 (data), 01 (EOF), 02 (extended segment address),
/// 04 (extended linear address), and 05 (start linear address / ignored).
pub fn parse_intel_hex(content: &str) -> Result<BinaryImage, DaedalusError> {
    let mut segments: Vec<(u32, Vec<u8>)> = Vec::new();
    let mut base: u32 = 0;

    for (line_no, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with(':') {
            return Err(DaedalusError::ParseError {
                message: format!("Line {}: expected ':', got {:?}", line_no + 1, &line[..1.min(line.len())]),
                source: None,
            });
        }

        let hex_str = &line[1..];
        let bytes = hex_decode(hex_str).map_err(|e| DaedalusError::ParseError {
            message: format!("Line {}: {e}", line_no + 1),
            source: None,
        })?;

        if bytes.len() < 5 {
            return Err(DaedalusError::ParseError {
                message: format!("Line {}: record too short", line_no + 1),
                source: None,
            });
        }

        // Verify checksum: sum of all bytes (including checksum) should be 0 mod 256.
        let checksum: u8 = bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        if checksum != 0 {
            return Err(DaedalusError::ParseError {
                message: format!("Line {}: checksum mismatch", line_no + 1),
                source: None,
            });
        }

        let byte_count = bytes[0] as usize;
        let address = u16::from_be_bytes([bytes[1], bytes[2]]) as u32;
        let record_type = bytes[3];
        let data = &bytes[4..4 + byte_count];

        match record_type {
            0x00 => {
                // Data record.
                let full_addr = base + address;
                segments.push((full_addr, data.to_vec()));
            }
            0x01 => {
                // End-of-file.
                break;
            }
            0x02 => {
                // Extended segment address.
                if data.len() >= 2 {
                    base = (u16::from_be_bytes([data[0], data[1]]) as u32) << 4;
                }
            }
            0x04 => {
                // Extended linear address.
                if data.len() >= 2 {
                    base = (u16::from_be_bytes([data[0], data[1]]) as u32) << 16;
                }
            }
            0x03 | 0x05 => {
                // Start segment / start linear address — informational, skip.
            }
            _ => {
                return Err(DaedalusError::ParseError {
                    message: format!("Line {}: unknown record type 0x{record_type:02X}", line_no + 1),
                    source: None,
                });
            }
        }
    }

    if segments.is_empty() {
        return Err(DaedalusError::ParseError {
            message: "Intel HEX: no data records found".into(),
            source: None,
        });
    }

    // Determine the overall range and fill a contiguous buffer.
    let min_addr = segments.iter().map(|(a, _)| *a).min().unwrap();
    let max_end = segments
        .iter()
        .map(|(a, d)| *a + d.len() as u32)
        .max()
        .unwrap();

    let total = (max_end - min_addr) as usize;
    let mut data = vec![0xFFu8; total]; // 0xFF = erased flash default

    for (addr, seg) in &segments {
        let offset = (*addr - min_addr) as usize;
        data[offset..offset + seg.len()].copy_from_slice(seg);
    }

    let size = data.len();
    Ok(BinaryImage {
        data,
        base_address: min_addr,
        format: BinaryFormat::IntelHex,
        size,
    })
}

// ── Motorola S-Record parser ─────────────────────────────────────────

/// Parse a Motorola S-Record formatted string into a [`BinaryImage`].
///
/// Supports S0 (header), S1 (16-bit addr data), S2 (24-bit), S3 (32-bit),
/// S7/S8/S9 (terminators).
pub fn parse_srec(content: &str) -> Result<BinaryImage, DaedalusError> {
    let mut segments: Vec<(u32, Vec<u8>)> = Vec::new();

    for (line_no, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.len() < 2 || !line.starts_with('S') {
            return Err(DaedalusError::ParseError {
                message: format!("Line {}: invalid S-Record prefix", line_no + 1),
                source: None,
            });
        }

        let record_type = line.as_bytes()[1];
        let hex_str = &line[2..];
        let bytes = hex_decode(hex_str).map_err(|e| DaedalusError::ParseError {
            message: format!("Line {}: {e}", line_no + 1),
            source: None,
        })?;

        if bytes.is_empty() {
            continue;
        }

        // Verify checksum: one's complement of sum of all bytes except the checksum.
        let sum: u8 = bytes[..bytes.len() - 1]
            .iter()
            .fold(0u8, |acc, &b| acc.wrapping_add(b));
        let expected_checksum = !sum;
        if expected_checksum != bytes[bytes.len() - 1] {
            return Err(DaedalusError::ParseError {
                message: format!("Line {}: checksum mismatch", line_no + 1),
                source: None,
            });
        }

        let byte_count = bytes[0] as usize;
        // byte_count includes address + data + checksum.
        let payload = &bytes[1..1 + byte_count - 1]; // Exclude checksum byte.

        match record_type {
            b'0' => {
                // Header — skip.
            }
            b'1' => {
                // 16-bit address data.
                if payload.len() < 2 {
                    continue;
                }
                let addr = u16::from_be_bytes([payload[0], payload[1]]) as u32;
                let data = &payload[2..];
                segments.push((addr, data.to_vec()));
            }
            b'2' => {
                // 24-bit address data.
                if payload.len() < 3 {
                    continue;
                }
                let addr = u32::from_be_bytes([0, payload[0], payload[1], payload[2]]);
                let data = &payload[3..];
                segments.push((addr, data.to_vec()));
            }
            b'3' => {
                // 32-bit address data.
                if payload.len() < 4 {
                    continue;
                }
                let addr = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                let data = &payload[4..];
                segments.push((addr, data.to_vec()));
            }
            b'5' | b'6' => {
                // Record count — informational, skip.
            }
            b'7' | b'8' | b'9' => {
                // Terminator — stop.
                break;
            }
            _ => {
                return Err(DaedalusError::ParseError {
                    message: format!("Line {}: unknown S-Record type S{}", line_no + 1, record_type as char),
                    source: None,
                });
            }
        }
    }

    if segments.is_empty() {
        return Err(DaedalusError::ParseError {
            message: "S-Record: no data records found".into(),
            source: None,
        });
    }

    let min_addr = segments.iter().map(|(a, _)| *a).min().unwrap();
    let max_end = segments
        .iter()
        .map(|(a, d)| *a + d.len() as u32)
        .max()
        .unwrap();

    let total = (max_end - min_addr) as usize;
    let mut data = vec![0xFFu8; total];

    for (addr, seg) in &segments {
        let offset = (*addr - min_addr) as usize;
        data[offset..offset + seg.len()].copy_from_slice(seg);
    }

    let size = data.len();
    Ok(BinaryImage {
        data,
        base_address: min_addr,
        format: BinaryFormat::MotorolaSrec,
        size,
    })
}

// ── Hex string helper ────────────────────────────────────────────────

/// Decode a hex string (e.g. `"0A1B2C"`) into bytes.
fn hex_decode(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("Hex string has odd length".into());
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    for i in (0..hex.len()).step_by(2) {
        let byte = u8::from_str_radix(&hex[i..i + 2], 16)
            .map_err(|_| format!("Invalid hex byte at offset {i}: {:?}", &hex[i..i + 2]))?;
        out.push(byte);
    }
    Ok(out)
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_image_basics() {
        let img = BinaryImage::from_raw(vec![0x00, 0x11, 0x22, 0x33]);
        assert_eq!(img.len(), 4);
        assert_eq!(img.base_address(), 0);
        assert_eq!(img.read_u8(0), Some(0x00));
        assert_eq!(img.read_u8(3), Some(0x33));
        assert_eq!(img.read_u8(4), None);
    }

    #[test]
    fn read_u16() {
        let img = BinaryImage::from_raw(vec![0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(img.read_u16_be(0), Some(0xDEAD));
        assert_eq!(img.read_u16_le(0), Some(0xADDE));
        assert_eq!(img.read_u16_be(2), Some(0xBEEF));
        assert_eq!(img.read_u16_be(3), None); // would overflow
    }

    #[test]
    fn write_operations() {
        let mut img = BinaryImage::from_raw(vec![0; 8]);
        img.write_u8(0, 0xAA).unwrap();
        assert_eq!(img.read_u8(0), Some(0xAA));

        img.write_u16_be(2, 0x1234).unwrap();
        assert_eq!(img.read_u16_be(2), Some(0x1234));

        assert!(img.write_u8(100, 0xFF).is_err());
    }

    #[test]
    fn region_slice() {
        let img = BinaryImage::from_raw(vec![10, 20, 30, 40, 50]);
        assert_eq!(img.region(1, 4), &[20, 30, 40]);
    }

    #[test]
    fn parse_intel_hex_simple() {
        // Minimal Intel HEX: 4 bytes at address 0x0000, then EOF.
        let hex = ":04000000DEADBEEF43\n:00000001FF\n";
        let img = parse_intel_hex(hex).unwrap();
        assert_eq!(img.len(), 4);
        assert_eq!(img.base_address(), 0);
        assert_eq!(img.read_u8(0), Some(0xDE));
        assert_eq!(img.read_u8(3), Some(0xEF));
        assert_eq!(img.format(), BinaryFormat::IntelHex);
    }

    #[test]
    fn parse_intel_hex_extended_linear() {
        // Extended linear address 0x0800_0000, then 2 data bytes.
        let hex = ":020000040800F2\n:0200000041427B\n:00000001FF\n";
        let img = parse_intel_hex(hex).unwrap();
        assert_eq!(img.base_address(), 0x0800_0000);
        assert_eq!(img.read_u8(0x0800_0000), Some(0x41));
        assert_eq!(img.read_u8(0x0800_0001), Some(0x42));
    }

    #[test]
    fn parse_intel_hex_bad_checksum() {
        let hex = ":04000000DEADBEEF00\n:00000001FF\n";
        assert!(parse_intel_hex(hex).is_err());
    }

    #[test]
    fn parse_srec_s1() {
        // S0 header + S1 data (2-byte addr, 4 data bytes) + S9 terminator.
        // S1: byte_count=07, addr=0000, data=DEADBEEF, checksum
        // sum of 07+00+00+DE+AD+BE+EF = 07+DE+AD+BE+EF = 0x279, low byte 0x79
        // checksum = !0x79 = 0x86
        let srec = "S0030000FC\nS10700000102030468\nS9030000FC\n";
        let img = parse_srec(srec).unwrap();
        assert_eq!(img.base_address(), 0);
        assert_eq!(img.read_u8(0), Some(0x01));
        assert_eq!(img.read_u8(3), Some(0x04));
        assert_eq!(img.format(), BinaryFormat::MotorolaSrec);
    }

    #[test]
    fn hex_decode_valid() {
        assert_eq!(hex_decode("DEADBEEF").unwrap(), vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn hex_decode_invalid() {
        assert!(hex_decode("ZZZZ").is_err());
        assert!(hex_decode("ABC").is_err()); // odd length
    }
}
