//! Checksum algorithms for ECU firmware validation and correction.
//!
//! Automotive ECU firmware contains checksum fields that must be correct for the
//! ECU to accept the image.  This module implements the most common algorithms
//! and provides a generic verify/correct interface via [`ChecksumRegion`].

use forge_core::error::DaedalusError;
use serde::{Deserialize, Serialize};

/// Supported checksum algorithm families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChecksumType {
    /// IEEE 802.3 CRC-32 (polynomial 0xEDB88320, reflected).
    CRC32,
    /// Simple additive 16-bit checksum (sum of all `u16` words, wrapping).
    Sum16,
    /// Simple additive 32-bit checksum (sum of all `u32` words, wrapping).
    Sum32,
    /// Bosch ME7-family multipoint checksum.
    BoschME7,
    /// Bosch MED17 / EDC17 multipoint checksum.
    BoschMED17,
    /// User-defined / plug-in checksum (future use).
    Custom,
}

/// Describes one checksummed region inside the firmware image.
///
/// The checksum covers `data[start..end]` and the result is stored at
/// `checksum_addr` (which is typically *inside* the covered range and must
/// be excluded from the computation, or is outside — depends on the type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecksumRegion {
    /// Start offset (inclusive) of the data region.
    pub start: u32,
    /// End offset (exclusive) of the data region.
    pub end: u32,
    /// Offset where the checksum value is stored.
    pub checksum_addr: u32,
    /// Algorithm to use.
    pub checksum_type: ChecksumType,
}

// ── CRC-32 (IEEE) ────────────────────────────────────────────────────

/// CRC-32 lookup table (reflected polynomial 0xEDB88320).
const CRC32_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0u32;
    while i < 256 {
        let mut crc = i;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i as usize] = crc;
        i += 1;
    }
    table
};

/// Compute the standard IEEE CRC-32 of `data`.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        let idx = ((crc ^ byte as u32) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC32_TABLE[idx];
    }
    crc ^ 0xFFFF_FFFF
}

// ── Simple additive checksums ────────────────────────────────────────

/// Compute a 16-bit additive checksum (sum of all big-endian `u16` words).
///
/// If `data.len()` is odd, the final byte is treated as the high byte of a
/// `u16` with a zero low byte.
pub fn sum16(data: &[u8]) -> u16 {
    let mut sum: u16 = 0;
    let mut i = 0;
    while i + 1 < data.len() {
        let word = u16::from_be_bytes([data[i], data[i + 1]]);
        sum = sum.wrapping_add(word);
        i += 2;
    }
    if i < data.len() {
        sum = sum.wrapping_add((data[i] as u16) << 8);
    }
    sum
}

/// Compute a 32-bit additive checksum (sum of all big-endian `u32` words).
///
/// Data length should be a multiple of 4; trailing bytes are zero-padded.
pub fn sum32(data: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i + 3 < data.len() {
        let word = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        sum = sum.wrapping_add(word);
        i += 4;
    }
    // Handle trailing bytes.
    if i < data.len() {
        let mut pad = [0u8; 4];
        for (j, &b) in data[i..].iter().enumerate() {
            pad[j] = b;
        }
        sum = sum.wrapping_add(u32::from_be_bytes(pad));
    }
    sum
}

// ── Bosch multipoint ─────────────────────────────────────────────────

/// Compute Bosch-style multipoint checksums for a set of regions.
///
/// Returns `(checksum_addr, computed_value)` pairs.  The caller is
/// responsible for writing the values back into the image.
///
/// The algorithm uses [`sum32`] over each region, skipping the 4 bytes at
/// `checksum_addr` (which hold the checksum itself).
pub fn bosch_multipoint(data: &[u8], regions: &[ChecksumRegion]) -> Vec<(u32, u32)> {
    let mut results = Vec::with_capacity(regions.len());

    for region in regions {
        let start = region.start as usize;
        let end = region.end as usize;
        let cs_addr = region.checksum_addr as usize;

        if end > data.len() || start >= end {
            continue;
        }

        let mut sum: u32 = 0;
        let mut i = start;

        while i + 3 < end {
            // Skip the 4 bytes where the checksum is stored.
            if i == cs_addr {
                i += 4;
                continue;
            }
            let word = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
            sum = sum.wrapping_add(word);
            i += 4;
        }

        results.push((region.checksum_addr, sum));
    }

    results
}

// ── Generic verify / correct ─────────────────────────────────────────

/// Verify that the checksum stored in the image matches the computed value.
pub fn verify_checksum(data: &[u8], region: &ChecksumRegion) -> bool {
    let start = region.start as usize;
    let end = region.end as usize;
    let cs_addr = region.checksum_addr as usize;

    if end > data.len() {
        return false;
    }

    let slice = &data[start..end];

    match region.checksum_type {
        ChecksumType::CRC32 => {
            if cs_addr + 3 >= data.len() {
                return false;
            }
            let stored = u32::from_be_bytes([
                data[cs_addr],
                data[cs_addr + 1],
                data[cs_addr + 2],
                data[cs_addr + 3],
            ]);
            let computed = crc32(slice);
            stored == computed
        }
        ChecksumType::Sum16 => {
            if cs_addr + 1 >= data.len() {
                return false;
            }
            let stored = u16::from_be_bytes([data[cs_addr], data[cs_addr + 1]]);
            let computed = sum16(slice);
            stored == computed
        }
        ChecksumType::Sum32 | ChecksumType::BoschME7 | ChecksumType::BoschMED17 => {
            if cs_addr + 3 >= data.len() {
                return false;
            }
            let stored = u32::from_be_bytes([
                data[cs_addr],
                data[cs_addr + 1],
                data[cs_addr + 2],
                data[cs_addr + 3],
            ]);
            // For Bosch types, use multipoint with a single region.
            let results = bosch_multipoint(data, std::slice::from_ref(region));
            if let Some(&(_, computed)) = results.first() {
                stored == computed
            } else {
                false
            }
        }
        ChecksumType::Custom => {
            // Custom checksums cannot be verified without a plug-in.
            false
        }
    }
}

/// Correct the checksum in-place and return the new value.
///
/// The computed checksum is written into `data` at `region.checksum_addr`.
pub fn correct_checksum(data: &mut [u8], region: &ChecksumRegion) -> Result<u32, DaedalusError> {
    let start = region.start as usize;
    let end = region.end as usize;
    let cs_addr = region.checksum_addr as usize;

    if end > data.len() {
        return Err(DaedalusError::ChecksumError {
            message: format!("Region end 0x{end:08X} exceeds data length 0x{:08X}", data.len()),
            address: region.start,
        });
    }

    match region.checksum_type {
        ChecksumType::CRC32 => {
            let computed = crc32(&data[start..end]);
            if cs_addr + 3 >= data.len() {
                return Err(DaedalusError::ChecksumError {
                    message: "Checksum address out of range".into(),
                    address: region.checksum_addr,
                });
            }
            let bytes = computed.to_be_bytes();
            data[cs_addr..cs_addr + 4].copy_from_slice(&bytes);
            Ok(computed)
        }
        ChecksumType::Sum16 => {
            let computed = sum16(&data[start..end]);
            if cs_addr + 1 >= data.len() {
                return Err(DaedalusError::ChecksumError {
                    message: "Checksum address out of range".into(),
                    address: region.checksum_addr,
                });
            }
            let bytes = computed.to_be_bytes();
            data[cs_addr..cs_addr + 2].copy_from_slice(&bytes);
            Ok(computed as u32)
        }
        ChecksumType::Sum32 | ChecksumType::BoschME7 | ChecksumType::BoschMED17 => {
            let results = bosch_multipoint(data, std::slice::from_ref(region));
            if let Some(&(addr, computed)) = results.first() {
                let a = addr as usize;
                if a + 3 >= data.len() {
                    return Err(DaedalusError::ChecksumError {
                        message: "Checksum address out of range".into(),
                        address: addr,
                    });
                }
                let bytes = computed.to_be_bytes();
                data[a..a + 4].copy_from_slice(&bytes);
                Ok(computed)
            } else {
                Err(DaedalusError::ChecksumError {
                    message: "Bosch multipoint returned no results".into(),
                    address: region.start,
                })
            }
        }
        ChecksumType::Custom => Err(DaedalusError::ChecksumError {
            message: "Cannot correct custom checksum — no algorithm defined".into(),
            address: region.start,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_empty() {
        assert_eq!(crc32(&[]), 0x0000_0000);
    }

    #[test]
    fn crc32_known_value() {
        // CRC-32 of "123456789" = 0xCBF43926
        let data = b"123456789";
        assert_eq!(crc32(data), 0xCBF4_3926);
    }

    #[test]
    fn sum16_basic() {
        // 0x0001 + 0x0002 = 0x0003
        let data = [0x00, 0x01, 0x00, 0x02];
        assert_eq!(sum16(&data), 0x0003);
    }

    #[test]
    fn sum16_wrapping() {
        // 0xFFFF + 0x0001 = 0x0000 (wrapping)
        let data = [0xFF, 0xFF, 0x00, 0x01];
        assert_eq!(sum16(&data), 0x0000);
    }

    #[test]
    fn sum32_basic() {
        let data = [0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x14];
        assert_eq!(sum32(&data), 30); // 10 + 20
    }

    #[test]
    fn bosch_multipoint_basic() {
        // 64 bytes of data, checksum stored at offset 60.
        let mut data = vec![0u8; 64];
        // Fill with a known pattern: each u32 word = 1.
        for i in (0..60).step_by(4) {
            data[i..i + 4].copy_from_slice(&1u32.to_be_bytes());
        }
        // Offset 60 holds the checksum — skip during calculation.

        let region = ChecksumRegion {
            start: 0,
            end: 64,
            checksum_addr: 60,
            checksum_type: ChecksumType::Sum32,
        };

        let results = bosch_multipoint(&data, &[region]);
        assert_eq!(results.len(), 1);
        let (addr, value) = results[0];
        assert_eq!(addr, 60);
        assert_eq!(value, 15); // 15 words of value 1 (indices 0..60, skip 60..64)
    }

    #[test]
    fn correct_and_verify_sum32() {
        let mut data = vec![0u8; 64];
        for i in (0..56).step_by(4) {
            data[i..i + 4].copy_from_slice(&100u32.to_be_bytes());
        }

        let region = ChecksumRegion {
            start: 0,
            end: 64,
            checksum_addr: 56,
            checksum_type: ChecksumType::Sum32,
        };

        // Initially checksum is wrong (zero).
        assert!(!verify_checksum(&data, &region));

        // Correct it.
        let new_val = correct_checksum(&mut data, &region).unwrap();
        assert!(new_val > 0);

        // Now verification should pass.
        assert!(verify_checksum(&data, &region));
    }

    #[test]
    fn crc32_correct_and_verify() {
        let mut data = vec![0xAA; 32];

        let region = ChecksumRegion {
            start: 0,
            end: 28,
            checksum_addr: 28,
            checksum_type: ChecksumType::CRC32,
        };

        let new_val = correct_checksum(&mut data, &region).unwrap();
        assert!(new_val != 0);
        assert!(verify_checksum(&data, &region));
    }
}
