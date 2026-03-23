//! Binary comparison (stock vs. modified firmware).
//!
//! Produces a compact list of [`DiffRegion`]s and a human-readable summary
//! that can be shown in the frontend diff view before an ECU write.

use serde::Serialize;

use crate::parser::BinaryImage;

/// A contiguous region where two binaries differ.
#[derive(Debug, Clone, Serialize)]
pub struct DiffRegion {
    /// Start address of the differing region.
    pub start_addr: u32,
    /// Length in bytes.
    pub length: usize,
    /// Bytes from the original (stock) image.
    pub old_bytes: Vec<u8>,
    /// Bytes from the modified image.
    pub new_bytes: Vec<u8>,
}

/// Result of comparing two binary images.
#[derive(Debug, Clone, Serialize)]
pub struct DiffResult {
    /// Contiguous regions that differ between original and modified.
    pub regions: Vec<DiffRegion>,
    /// Total number of discrete change regions.
    pub total_changes: usize,
    /// Total number of individual bytes that differ.
    pub bytes_changed: usize,
}

/// Compare two binary images and return all differing regions.
///
/// Both images must have the same base address and length.
/// Adjacent differing bytes are merged into a single [`DiffRegion`].
pub fn diff_binaries(original: &BinaryImage, modified: &BinaryImage) -> DiffResult {
    let base = original.base_address();
    let orig_data = original.data();
    let mod_data = modified.data();

    // Use the smaller length to avoid out-of-bounds.
    let len = orig_data.len().min(mod_data.len());

    let mut regions: Vec<DiffRegion> = Vec::new();
    let mut bytes_changed: usize = 0;

    let mut i = 0;
    while i < len {
        if orig_data[i] != mod_data[i] {
            // Start of a differing region.
            let start = i;
            while i < len && orig_data[i] != mod_data[i] {
                i += 1;
            }
            let region_len = i - start;
            bytes_changed += region_len;

            regions.push(DiffRegion {
                start_addr: base + start as u32,
                length: region_len,
                old_bytes: orig_data[start..i].to_vec(),
                new_bytes: mod_data[start..i].to_vec(),
            });
        } else {
            i += 1;
        }
    }

    // If images differ in length, count the tail as a change.
    if orig_data.len() != mod_data.len() {
        let longer = orig_data.len().max(mod_data.len());
        let shorter = len;
        let tail_len = longer - shorter;
        bytes_changed += tail_len;

        let (old_tail, new_tail) = if orig_data.len() > mod_data.len() {
            (orig_data[shorter..].to_vec(), Vec::new())
        } else {
            (Vec::new(), mod_data[shorter..].to_vec())
        };

        regions.push(DiffRegion {
            start_addr: base + shorter as u32,
            length: tail_len,
            old_bytes: old_tail,
            new_bytes: new_tail,
        });
    }

    let total_changes = regions.len();

    DiffResult {
        regions,
        total_changes,
        bytes_changed,
    }
}

/// Produce a human-readable summary of a [`DiffResult`].
pub fn diff_summary(result: &DiffResult) -> String {
    if result.total_changes == 0 {
        return "Binaries are identical.".to_string();
    }

    let mut lines = Vec::new();
    lines.push(format!(
        "Found {} change region(s), {} byte(s) modified.",
        result.total_changes, result.bytes_changed,
    ));

    for (idx, region) in result.regions.iter().enumerate() {
        lines.push(format!(
            "  Region {}: 0x{:08X}..0x{:08X} ({} byte{})",
            idx + 1,
            region.start_addr,
            region.start_addr + region.length as u32,
            region.length,
            if region.length == 1 { "" } else { "s" },
        ));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::BinaryImage;

    #[test]
    fn identical_binaries() {
        let a = BinaryImage::from_raw(vec![0x01, 0x02, 0x03]);
        let b = BinaryImage::from_raw(vec![0x01, 0x02, 0x03]);
        let result = diff_binaries(&a, &b);
        assert_eq!(result.total_changes, 0);
        assert_eq!(result.bytes_changed, 0);
        assert!(result.regions.is_empty());

        let summary = diff_summary(&result);
        assert!(summary.contains("identical"));
    }

    #[test]
    fn single_byte_change() {
        let a = BinaryImage::from_raw(vec![0x00, 0x11, 0x22, 0x33]);
        let b = BinaryImage::from_raw(vec![0x00, 0xFF, 0x22, 0x33]);
        let result = diff_binaries(&a, &b);
        assert_eq!(result.total_changes, 1);
        assert_eq!(result.bytes_changed, 1);
        assert_eq!(result.regions[0].start_addr, 1);
        assert_eq!(result.regions[0].old_bytes, vec![0x11]);
        assert_eq!(result.regions[0].new_bytes, vec![0xFF]);
    }

    #[test]
    fn multiple_regions() {
        let a = BinaryImage::from_raw(vec![0x00, 0x11, 0x22, 0x33, 0x44]);
        let b = BinaryImage::from_raw(vec![0xFF, 0x11, 0x22, 0xEE, 0xDD]);
        let result = diff_binaries(&a, &b);
        assert_eq!(result.total_changes, 2);
        assert_eq!(result.bytes_changed, 3); // byte 0 + bytes 3,4
    }

    #[test]
    fn contiguous_changes_merged() {
        let a = BinaryImage::from_raw(vec![0x00, 0x00, 0x00, 0x00]);
        let b = BinaryImage::from_raw(vec![0xFF, 0xFF, 0xFF, 0x00]);
        let result = diff_binaries(&a, &b);
        assert_eq!(result.total_changes, 1);
        assert_eq!(result.regions[0].length, 3);
    }

    #[test]
    fn different_lengths() {
        let a = BinaryImage::from_raw(vec![0x00, 0x11]);
        let b = BinaryImage::from_raw(vec![0x00, 0x11, 0x22, 0x33]);
        let result = diff_binaries(&a, &b);
        assert_eq!(result.total_changes, 1); // Tail region.
        assert_eq!(result.bytes_changed, 2);
    }

    #[test]
    fn summary_formatting() {
        let a = BinaryImage::from_raw(vec![0x00; 100]);
        let mut mod_data = vec![0x00; 100];
        mod_data[10] = 0xFF;
        mod_data[50] = 0xAA;
        mod_data[51] = 0xBB;
        let b = BinaryImage::from_raw(mod_data);
        let result = diff_binaries(&a, &b);
        let summary = diff_summary(&result);
        assert!(summary.contains("2 change region"));
        assert!(summary.contains("3 byte(s)"));
        assert!(summary.contains("0x0000000A")); // addr 10
    }
}
