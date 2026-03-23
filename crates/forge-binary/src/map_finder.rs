//! Heuristic map detection in ECU firmware binaries.
//!
//! Automotive ECU calibration data is stored as 1-D or 2-D tables ("maps")
//! with associated axis vectors.  This module scans a [`BinaryImage`] and
//! returns scored [`MapCandidate`]s that the frontend can present for review.

use serde::{Deserialize, Serialize};

use crate::parser::BinaryImage;

/// Numeric encoding of map cell values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    U8,
    U16BE,
    U16LE,
    S16BE,
    S16LE,
    F32,
}

impl DataType {
    /// Size of a single element in bytes.
    pub fn element_size(self) -> usize {
        match self {
            Self::U8 => 1,
            Self::U16BE | Self::U16LE | Self::S16BE | Self::S16LE => 2,
            Self::F32 => 4,
        }
    }
}

/// A candidate calibration map found by the heuristic scanner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapCandidate {
    /// Start address in the binary image.
    pub start_addr: u32,
    /// End address (exclusive).
    pub end_addr: u32,
    /// Number of rows (Y-axis elements).
    pub rows: u16,
    /// Number of columns (X-axis elements).
    pub cols: u16,
    /// Detected data encoding.
    pub data_type: DataType,
    /// Confidence score in `[0.0, 1.0]`.
    pub confidence: f32,
    /// Optional human-readable hint (e.g. "Fuel map", "Ignition timing").
    pub map_type_hint: Option<String>,
}

// ── Block-level analysis constants ───────────────────────────────────

/// Block size for entropy / structure analysis.
const BLOCK_SIZE: usize = 256;

/// Maximum Shannon entropy (bits) to consider a block "structured".
/// Pure random data ≈ 8.0; repeating calibration data typically < 6.0.
const MAX_STRUCTURED_ENTROPY: f64 = 5.5;

/// Minimum confidence to include a candidate in results.
const MIN_CONFIDENCE: f32 = 0.25;

// ── Public API ───────────────────────────────────────────────────────

/// Scan the binary image for calibration map candidates.
///
/// `min_size` is the minimum number of *data bytes* a map must contain
/// (e.g., 16 for a 4x4 U8 map).  Candidates below that are discarded.
pub fn find_maps(image: &BinaryImage, min_size: usize) -> Vec<MapCandidate> {
    let data = image.data();
    if data.len() < BLOCK_SIZE {
        return Vec::new();
    }

    let base = image.base_address();
    let num_blocks = data.len() / BLOCK_SIZE;
    let mut candidates: Vec<MapCandidate> = Vec::new();

    // Pass 1: identify structured blocks.
    let mut structured_runs: Vec<(usize, usize)> = Vec::new(); // (start_block, length)
    let mut run_start: Option<usize> = None;

    for block_idx in 0..num_blocks {
        let offset = block_idx * BLOCK_SIZE;
        let block = &data[offset..offset + BLOCK_SIZE];

        let entropy = calculate_entropy(block);
        let is_structured = entropy < MAX_STRUCTURED_ENTROPY
            && !is_all_same(block)
            && !is_all_ff(block);

        if is_structured {
            if run_start.is_none() {
                run_start = Some(block_idx);
            }
        } else if let Some(start) = run_start.take() {
            structured_runs.push((start, block_idx - start));
        }
    }
    // Close final run.
    if let Some(start) = run_start {
        structured_runs.push((start, num_blocks - start));
    }

    // Pass 2: for each structured run, try to detect map dimensions.
    for (start_block, block_count) in structured_runs {
        let byte_start = start_block * BLOCK_SIZE;
        let byte_end = (start_block + block_count) * BLOCK_SIZE;
        let region = &data[byte_start..byte_end.min(data.len())];
        let region_len = region.len();

        if region_len < min_size {
            continue;
        }

        // Try U16BE first (most common in Bosch ECUs), then U8.
        for dtype in &[DataType::U16BE, DataType::U16LE, DataType::U8] {
            let elem_size = dtype.element_size();
            let total_elements = region_len / elem_size;

            if total_elements < 4 {
                continue;
            }

            // Try common map dimensions.
            let (rows, cols, score) = guess_dimensions(region, *dtype, total_elements);

            if rows == 0 || cols == 0 {
                continue;
            }

            let map_bytes = (rows as usize) * (cols as usize) * elem_size;
            if map_bytes < min_size || map_bytes > region_len {
                continue;
            }

            let confidence = score;
            if confidence < MIN_CONFIDENCE {
                continue;
            }

            candidates.push(MapCandidate {
                start_addr: base + byte_start as u32,
                end_addr: base + (byte_start + map_bytes) as u32,
                rows,
                cols,
                data_type: *dtype,
                confidence,
                map_type_hint: None,
            });

            break; // Accept first matching data type for this region.
        }
    }

    // Sort by confidence descending.
    candidates.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
    candidates
}

/// Detect a monotonically increasing axis in a byte slice.
///
/// Returns `Some(values)` if the data contains a plausible axis vector
/// (at least 4 elements, monotonically non-decreasing).
pub fn detect_axis(data: &[u8], data_type: DataType) -> Option<Vec<f64>> {
    let elem_size = data_type.element_size();
    if data.len() < elem_size * 4 {
        return None;
    }

    let count = data.len() / elem_size;
    let mut values = Vec::with_capacity(count);

    for i in 0..count {
        let offset = i * elem_size;
        let val = read_element(data, offset, data_type)?;
        values.push(val);
    }

    // Check monotonicity: allow equal but require at least one strict increase.
    let mut has_increase = false;
    for w in values.windows(2) {
        if w[1] < w[0] {
            return None; // Not monotonic.
        }
        if w[1] > w[0] {
            has_increase = true;
        }
    }

    if has_increase {
        Some(values)
    } else {
        None
    }
}

/// Calculate Shannon entropy of a byte slice (0.0 = uniform, 8.0 = random).
pub fn calculate_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let mut freq = [0u32; 256];
    for &b in data {
        freq[b as usize] += 1;
    }

    let len = data.len() as f64;
    let mut entropy = 0.0;

    for &count in &freq {
        if count > 0 {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }
    }

    entropy
}

// ── Internal helpers ─────────────────────────────────────────────────

/// Check if every byte in the block is the same value.
fn is_all_same(data: &[u8]) -> bool {
    if data.is_empty() {
        return true;
    }
    let first = data[0];
    data.iter().all(|&b| b == first)
}

/// Check if the entire block is erased flash (0xFF).
fn is_all_ff(data: &[u8]) -> bool {
    data.iter().all(|&b| b == 0xFF)
}

/// Read a single element from a byte slice at the given offset.
fn read_element(data: &[u8], offset: usize, dtype: DataType) -> Option<f64> {
    match dtype {
        DataType::U8 => data.get(offset).map(|&b| b as f64),
        DataType::U16BE => {
            if offset + 1 >= data.len() {
                return None;
            }
            Some(u16::from_be_bytes([data[offset], data[offset + 1]]) as f64)
        }
        DataType::U16LE => {
            if offset + 1 >= data.len() {
                return None;
            }
            Some(u16::from_le_bytes([data[offset], data[offset + 1]]) as f64)
        }
        DataType::S16BE => {
            if offset + 1 >= data.len() {
                return None;
            }
            Some(i16::from_be_bytes([data[offset], data[offset + 1]]) as f64)
        }
        DataType::S16LE => {
            if offset + 1 >= data.len() {
                return None;
            }
            Some(i16::from_le_bytes([data[offset], data[offset + 1]]) as f64)
        }
        DataType::F32 => {
            if offset + 3 >= data.len() {
                return None;
            }
            let bytes = [data[offset], data[offset + 1], data[offset + 2], data[offset + 3]];
            let val = f32::from_be_bytes(bytes) as f64;
            if val.is_finite() {
                Some(val)
            } else {
                None
            }
        }
    }
}

/// Attempt to determine (rows, cols, confidence) for a structured region.
///
/// Strategy: try common factorizations of total_elements and score each one
/// by how well the data fits a 2-D table pattern (row-to-row smoothness,
/// column continuity, axis presence near the region).
fn guess_dimensions(
    region: &[u8],
    dtype: DataType,
    total_elements: usize,
) -> (u16, u16, f32) {
    // Common ECU map sizes (rows x cols).
    let common_dims: &[(u16, u16)] = &[
        (8, 8), (8, 16), (16, 16), (16, 20), (12, 12),
        (16, 8), (20, 16), (10, 10), (8, 12), (12, 16),
        (6, 6), (6, 8), (8, 10), (10, 12), (10, 16),
        (4, 4), (4, 8), (8, 4), (16, 32), (32, 16),
        (20, 20), (24, 24), (12, 8), (1, 16), (1, 8),
    ];

    let elem_size = dtype.element_size();
    let mut best = (0u16, 0u16, 0.0f32);

    for &(r, c) in common_dims {
        let needed = (r as usize) * (c as usize);
        if needed > total_elements || needed < 4 {
            continue;
        }

        let map_bytes = needed * elem_size;
        if map_bytes > region.len() {
            continue;
        }

        let score = score_map_fit(region, dtype, r, c);
        if score > best.2 {
            best = (r, c, score);
        }
    }

    // Also try exact factorizations if none of the common dims matched well.
    if best.2 < 0.3 {
        for r in 4u16..=64 {
            if total_elements % (r as usize) != 0 {
                continue;
            }
            let c = (total_elements / r as usize) as u16;
            if c < 4 || c > 64 {
                continue;
            }
            let map_bytes = (r as usize) * (c as usize) * elem_size;
            if map_bytes > region.len() {
                continue;
            }
            let score = score_map_fit(region, dtype, r, c);
            if score > best.2 {
                best = (r, c, score);
            }
        }
    }

    best
}

/// Score how well data[0 .. rows*cols*elem_size] looks like a 2-D map.
///
/// Criteria:
/// - Row-to-row smoothness (adjacent rows have similar values).
/// - Value range is reasonable (not all zeros, not all max).
/// - Low local variance per row (cells within a row are somewhat related).
fn score_map_fit(data: &[u8], dtype: DataType, rows: u16, cols: u16) -> f32 {
    let elem_size = dtype.element_size();
    let r = rows as usize;
    let c = cols as usize;
    let total_bytes = r * c * elem_size;

    if total_bytes > data.len() {
        return 0.0;
    }

    // Read all values into a matrix.
    let mut values = Vec::with_capacity(r * c);
    for i in 0..(r * c) {
        match read_element(data, i * elem_size, dtype) {
            Some(v) => values.push(v),
            None => return 0.0,
        }
    }

    // Check value range.
    let min_val = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max_val - min_val;

    if range < 1.0 {
        return 0.05; // Nearly constant — unlikely interesting map.
    }

    // Row-to-row smoothness: average absolute difference between adjacent rows.
    let mut row_diff_sum = 0.0;
    let mut row_diff_count = 0usize;

    for row in 1..r {
        for col in 0..c {
            let prev = values[(row - 1) * c + col];
            let curr = values[row * c + col];
            row_diff_sum += (curr - prev).abs();
            row_diff_count += 1;
        }
    }

    let avg_row_diff = if row_diff_count > 0 {
        row_diff_sum / row_diff_count as f64
    } else {
        range
    };

    // Smoothness ratio: smaller = smoother transitions between rows.
    let smoothness = 1.0 - (avg_row_diff / range).min(1.0);

    // Column continuity: within each row, values should not jump wildly.
    let mut col_diff_sum = 0.0;
    let mut col_diff_count = 0usize;

    for row in 0..r {
        for col in 1..c {
            let prev = values[row * c + col - 1];
            let curr = values[row * c + col];
            col_diff_sum += (curr - prev).abs();
            col_diff_count += 1;
        }
    }

    let avg_col_diff = if col_diff_count > 0 {
        col_diff_sum / col_diff_count as f64
    } else {
        range
    };

    let col_smoothness = 1.0 - (avg_col_diff / range).min(1.0);

    // Combined score.
    let score = (smoothness * 0.5 + col_smoothness * 0.3 + 0.2) as f32;
    score.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropy_uniform() {
        let data = vec![0xAA; 256];
        let e = calculate_entropy(&data);
        assert!(e < 0.01, "Uniform data should have ~0 entropy, got {e}");
    }

    #[test]
    fn entropy_diverse() {
        let data: Vec<u8> = (0..=255).collect();
        let e = calculate_entropy(&data);
        assert!(
            (e - 8.0).abs() < 0.01,
            "All-distinct data should have ~8.0 entropy, got {e}"
        );
    }

    #[test]
    fn detect_axis_monotonic() {
        // 8 ascending U16BE values.
        let mut data = Vec::new();
        for v in [100u16, 200, 500, 1000, 1500, 2000, 3000, 4000] {
            data.extend_from_slice(&v.to_be_bytes());
        }
        let axis = detect_axis(&data, DataType::U16BE);
        assert!(axis.is_some());
        let vals = axis.unwrap();
        assert_eq!(vals.len(), 8);
        assert!((vals[0] - 100.0).abs() < 0.1);
        assert!((vals[7] - 4000.0).abs() < 0.1);
    }

    #[test]
    fn detect_axis_non_monotonic() {
        let mut data = Vec::new();
        for v in [100u16, 50, 200, 150] {
            data.extend_from_slice(&v.to_be_bytes());
        }
        assert!(detect_axis(&data, DataType::U16BE).is_none());
    }

    #[test]
    fn find_maps_on_structured_data() {
        // Create a 16x16 U16BE map with smooth gradient.
        let mut data = vec![0xFFu8; 512]; // Padding before.
        for row in 0u16..16 {
            for col in 0u16..16 {
                let value = row * 100 + col * 10;
                data.extend_from_slice(&value.to_be_bytes());
            }
        }
        data.extend(vec![0xFF; 512]); // Padding after.

        let img = BinaryImage::from_raw(data);
        let candidates = find_maps(&img, 32);
        // We should get at least one candidate covering the map region.
        assert!(
            !candidates.is_empty(),
            "Should detect at least one map candidate"
        );
    }

    #[test]
    fn find_maps_empty() {
        let img = BinaryImage::from_raw(vec![0xFF; 64]);
        let candidates = find_maps(&img, 16);
        assert!(candidates.is_empty(), "Erased flash should yield no maps");
    }
}
