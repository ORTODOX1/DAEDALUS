//! Hex editor data provider for the frontend.
//!
//! Translates a [`BinaryImage`] into rows of 16 bytes that the React hex
//! editor can render via Tauri IPC (all types derive `Serialize`).

use serde::Serialize;

use crate::parser::BinaryImage;

/// A single row in the hex editor: 16 bytes + ASCII representation.
#[derive(Debug, Clone, Serialize)]
pub struct HexViewRow {
    /// Logical start address of this row.
    pub address: u32,
    /// Up to 16 raw bytes (may be fewer for the last row).
    pub bytes: Vec<u8>,
    /// Printable-ASCII representation (non-printable replaced with `.`).
    pub ascii: String,
}

/// Thin wrapper providing paginated, searchable access to a [`BinaryImage`]
/// for the hex editor component.
pub struct HexView<'a> {
    image: &'a BinaryImage,
}

impl<'a> HexView<'a> {
    /// Create a new view over the given image.
    pub fn new(image: &'a BinaryImage) -> Self {
        Self { image }
    }

    /// Return `count` rows of 16 bytes starting at `start_addr`.
    ///
    /// Addresses that fall before the image start or after its end produce
    /// rows padded with `0x00`.
    pub fn rows(&self, start_addr: u32, count: usize) -> Vec<HexViewRow> {
        let mut result = Vec::with_capacity(count);
        let base = self.image.base_address();
        let img_end = base + self.image.len() as u32;

        for i in 0..count {
            let row_addr = start_addr.wrapping_add((i as u32) * 16);
            // Align to 16-byte boundary.
            let aligned = row_addr & !0xF;

            let mut bytes = Vec::with_capacity(16);
            let mut ascii = String::with_capacity(16);

            for col in 0u32..16 {
                let addr = aligned.wrapping_add(col);
                let byte = if addr >= base && addr < img_end {
                    self.image.read_u8(addr).unwrap_or(0)
                } else {
                    0x00
                };
                bytes.push(byte);
                ascii.push(to_printable(byte));
            }

            result.push(HexViewRow {
                address: aligned,
                bytes,
                ascii,
            });
        }

        result
    }

    /// Total number of 16-byte rows in the image (rounded up).
    pub fn total_rows(&self) -> usize {
        (self.image.len() + 15) / 16
    }

    /// Search for a byte pattern, returning all matching addresses.
    ///
    /// Uses a simple sliding-window scan.  For large binaries this is
    /// O(n) which is acceptable for typical ECU firmware sizes (< 16 MB).
    pub fn search_bytes(&self, pattern: &[u8]) -> Vec<u32> {
        if pattern.is_empty() || pattern.len() > self.image.len() {
            return Vec::new();
        }

        let data = self.image.data();
        let base = self.image.base_address();
        let mut matches = Vec::new();

        for i in 0..=data.len() - pattern.len() {
            if data[i..i + pattern.len()] == *pattern {
                matches.push(base + i as u32);
            }
        }

        matches
    }

    /// Search for an ASCII string, returning all matching addresses.
    pub fn search_string(&self, text: &str) -> Vec<u32> {
        self.search_bytes(text.as_bytes())
    }
}

/// Map a byte to its printable ASCII character, or `.` if non-printable.
fn to_printable(b: u8) -> char {
    if b.is_ascii_graphic() || b == b' ' {
        b as char
    } else {
        '.'
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::BinaryImage;

    #[test]
    fn rows_basic() {
        let data: Vec<u8> = (0..48).collect(); // 3 full rows
        let img = BinaryImage::from_raw(data);
        let view = HexView::new(&img);

        assert_eq!(view.total_rows(), 3);

        let rows = view.rows(0, 3);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].address, 0);
        assert_eq!(rows[0].bytes.len(), 16);
        assert_eq!(rows[1].address, 16);
        assert_eq!(rows[2].address, 32);
    }

    #[test]
    fn search_bytes_found() {
        let img = BinaryImage::from_raw(vec![0x00, 0xDE, 0xAD, 0x00, 0xDE, 0xAD]);
        let view = HexView::new(&img);
        let hits = view.search_bytes(&[0xDE, 0xAD]);
        assert_eq!(hits, vec![1, 4]);
    }

    #[test]
    fn search_string_found() {
        let mut data = vec![0u8; 10];
        data[3..7].copy_from_slice(b"ABCD");
        let img = BinaryImage::from_raw(data);
        let view = HexView::new(&img);
        let hits = view.search_string("ABCD");
        assert_eq!(hits, vec![3]);
    }

    #[test]
    fn search_empty_pattern() {
        let img = BinaryImage::from_raw(vec![0; 10]);
        let view = HexView::new(&img);
        assert!(view.search_bytes(&[]).is_empty());
    }

    #[test]
    fn ascii_representation() {
        assert_eq!(to_printable(b'A'), 'A');
        assert_eq!(to_printable(b' '), ' ');
        assert_eq!(to_printable(0x00), '.');
        assert_eq!(to_printable(0xFF), '.');
    }
}
