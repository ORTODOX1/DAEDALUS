//! ISO 15765 (ISO-TP) transport protocol for CAN bus.
//!
//! Handles segmentation and reassembly of messages that exceed
//! the 8-byte CAN frame payload limit.

use forge_core::{DaedalusError, Result};
use serde::{Deserialize, Serialize};

/// Maximum payload in a single CAN frame (classic CAN).
const CAN_MAX_DLC: usize = 8;

/// Maximum data bytes in a Single Frame (1 byte for PCI).
const SF_MAX_DATA: usize = CAN_MAX_DLC - 1;

/// Maximum data bytes in the first First Frame (2 bytes for PCI).
const FF_DATA_LEN: usize = CAN_MAX_DLC - 2;

/// Maximum data bytes in a Consecutive Frame (1 byte for PCI).
const CF_DATA_LEN: usize = CAN_MAX_DLC - 1;

// ── PCI type nibbles ───────────────────────────────────────────────────

const PCI_SINGLE: u8 = 0x00;
const PCI_FIRST: u8 = 0x10;
const PCI_CONSECUTIVE: u8 = 0x20;
const PCI_FLOW_CONTROL: u8 = 0x30;

/// ISO-TP transport configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsoTpConfig {
    /// CAN arbitration ID used for transmitted frames.
    pub tx_id: u32,
    /// CAN arbitration ID expected for received frames.
    pub rx_id: u32,
    /// Number of Consecutive Frames the receiver can accept before
    /// the next Flow Control (0 = no limit).
    pub block_size: u8,
    /// Minimum Separation Time between Consecutive Frames (milliseconds).
    pub st_min: u8,
    /// Padding byte for frames shorter than 8 bytes.
    pub padding: u8,
}

impl Default for IsoTpConfig {
    fn default() -> Self {
        Self {
            tx_id: 0x7E0,
            rx_id: 0x7E8,
            block_size: 0,
            st_min: 10,
            padding: 0xCC,
        }
    }
}

/// Flow Control status flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowStatus {
    /// Receiver is ready — continue sending.
    ContinueToSend = 0,
    /// Receiver requests a pause — wait for another FC.
    Wait = 1,
    /// Receiver buffer overflow — abort transfer.
    Overflow = 2,
}

impl TryFrom<u8> for FlowStatus {
    type Error = DaedalusError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::ContinueToSend),
            1 => Ok(Self::Wait),
            2 => Ok(Self::Overflow),
            other => Err(DaedalusError::ParseError {
                message: format!("invalid FlowStatus: 0x{other:02X}"),
                source: None,
            }),
        }
    }
}

/// A decoded ISO-TP frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IsoTpFrame {
    /// Single Frame — complete message in one CAN frame (data <= 7 bytes).
    Single { data: Vec<u8> },
    /// First Frame — beginning of a multi-frame message.
    First { total_len: u16, data: Vec<u8> },
    /// Consecutive Frame — continuation segment.
    Consecutive { seq: u8, data: Vec<u8> },
    /// Flow Control — receiver flow management.
    FlowControl {
        flag: FlowStatus,
        block_size: u8,
        st_min: u8,
    },
}

/// Encode a single-frame ISO-TP message.
///
/// # Errors
/// Returns an error if `data` exceeds 7 bytes.
pub fn encode_single(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() > SF_MAX_DATA {
        return Err(DaedalusError::ParseError {
            message: format!(
                "single frame data too long: {} bytes (max {})",
                data.len(),
                SF_MAX_DATA
            ),
            source: None,
        });
    }

    let mut frame = Vec::with_capacity(CAN_MAX_DLC);
    frame.push(PCI_SINGLE | data.len() as u8);
    frame.extend_from_slice(data);
    Ok(frame)
}

/// Encode a multi-frame ISO-TP message (First Frame + Consecutive Frames).
///
/// Returns a vector of raw CAN payloads. The caller must send each one
/// as a CAN frame, respecting Flow Control timing from the receiver.
///
/// # Errors
/// Returns an error if `data` is empty or exceeds 4095 bytes.
pub fn encode_multi(data: &[u8], config: &IsoTpConfig) -> Result<Vec<Vec<u8>>> {
    if data.is_empty() {
        return Err(DaedalusError::ParseError {
            message: "cannot encode empty data as multi-frame".into(),
            source: None,
        });
    }
    if data.len() > 4095 {
        return Err(DaedalusError::ParseError {
            message: format!("data too long for ISO-TP: {} bytes (max 4095)", data.len()),
            source: None,
        });
    }

    // If it fits in a single frame, use that instead.
    if data.len() <= SF_MAX_DATA {
        return Ok(vec![encode_single(data)?]);
    }

    let total_len = data.len() as u16;
    let mut frames = Vec::new();

    // ── First Frame ────────────────────────────────────────────────────
    let mut ff = Vec::with_capacity(CAN_MAX_DLC);
    ff.push(PCI_FIRST | ((total_len >> 8) & 0x0F) as u8);
    ff.push((total_len & 0xFF) as u8);

    let ff_payload = data.len().min(FF_DATA_LEN);
    ff.extend_from_slice(&data[..ff_payload]);
    // Pad to 8 bytes.
    while ff.len() < CAN_MAX_DLC {
        ff.push(config.padding);
    }
    frames.push(ff);

    // ── Consecutive Frames ─────────────────────────────────────────────
    let mut offset = ff_payload;
    let mut seq: u8 = 1;
    while offset < data.len() {
        let mut cf = Vec::with_capacity(CAN_MAX_DLC);
        cf.push(PCI_CONSECUTIVE | (seq & 0x0F));

        let end = (offset + CF_DATA_LEN).min(data.len());
        cf.extend_from_slice(&data[offset..end]);

        // Pad to 8 bytes.
        while cf.len() < CAN_MAX_DLC {
            cf.push(config.padding);
        }

        frames.push(cf);
        offset = end;
        seq = (seq + 1) & 0x0F; // wraps 0..15
    }

    Ok(frames)
}

/// Decode a raw CAN payload into an [`IsoTpFrame`].
///
/// # Errors
/// Returns an error if the payload is empty or has an unknown PCI type.
pub fn decode_frame(raw: &[u8]) -> Result<IsoTpFrame> {
    if raw.is_empty() {
        return Err(DaedalusError::ParseError {
            message: "empty ISO-TP frame".into(),
            source: None,
        });
    }

    let pci_type = raw[0] & 0xF0;

    match pci_type {
        PCI_SINGLE => {
            let len = (raw[0] & 0x0F) as usize;
            if len == 0 || raw.len() < 1 + len {
                return Err(DaedalusError::ParseError {
                    message: format!(
                        "invalid single frame: declared len={len}, payload len={}",
                        raw.len()
                    ),
                    source: None,
                });
            }
            Ok(IsoTpFrame::Single {
                data: raw[1..1 + len].to_vec(),
            })
        }

        PCI_FIRST => {
            if raw.len() < 2 {
                return Err(DaedalusError::ParseError {
                    message: "first frame too short".into(),
                    source: None,
                });
            }
            let total_len = (((raw[0] & 0x0F) as u16) << 8) | raw[1] as u16;
            let data_end = raw.len().min(CAN_MAX_DLC);
            Ok(IsoTpFrame::First {
                total_len,
                data: raw[2..data_end].to_vec(),
            })
        }

        PCI_CONSECUTIVE => {
            let seq = raw[0] & 0x0F;
            Ok(IsoTpFrame::Consecutive {
                seq,
                data: raw[1..].to_vec(),
            })
        }

        PCI_FLOW_CONTROL => {
            if raw.len() < 3 {
                return Err(DaedalusError::ParseError {
                    message: "flow control frame too short".into(),
                    source: None,
                });
            }
            let flag = FlowStatus::try_from(raw[0] & 0x0F)?;
            Ok(IsoTpFrame::FlowControl {
                flag,
                block_size: raw[1],
                st_min: raw[2],
            })
        }

        other => Err(DaedalusError::ParseError {
            message: format!("unknown ISO-TP PCI type: 0x{other:02X}"),
            source: None,
        }),
    }
}

/// Reassembles multi-frame ISO-TP messages from individual frames.
///
/// Feed decoded [`IsoTpFrame`]s in order. When a complete message is
/// assembled, [`feed`](IsoTpAssembler::feed) returns `Some(data)`.
#[derive(Debug)]
pub struct IsoTpAssembler {
    buffer: Vec<u8>,
    expected_len: usize,
    next_seq: u8,
    active: bool,
}

impl IsoTpAssembler {
    /// Create a new assembler in idle state.
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            expected_len: 0,
            next_seq: 1,
            active: false,
        }
    }

    /// Reset the assembler, discarding any partial message.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.expected_len = 0;
        self.next_seq = 1;
        self.active = false;
    }

    /// Feed a decoded frame into the assembler.
    ///
    /// Returns `Some(complete_data)` when all segments have been received.
    /// Returns `None` when more frames are still expected.
    pub fn feed(&mut self, frame: IsoTpFrame) -> Option<Vec<u8>> {
        match frame {
            IsoTpFrame::Single { data } => {
                self.reset();
                Some(data)
            }

            IsoTpFrame::First { total_len, data } => {
                self.reset();
                self.expected_len = total_len as usize;
                self.buffer.extend_from_slice(&data);
                self.next_seq = 1;
                self.active = true;

                if self.buffer.len() >= self.expected_len {
                    self.buffer.truncate(self.expected_len);
                    let result = std::mem::take(&mut self.buffer);
                    self.reset();
                    Some(result)
                } else {
                    None
                }
            }

            IsoTpFrame::Consecutive { seq, data } => {
                if !self.active {
                    tracing::warn!(
                        "received consecutive frame seq={seq} without active transfer"
                    );
                    return None;
                }

                if seq != self.next_seq {
                    tracing::warn!(
                        "sequence mismatch: expected {}, got {seq}",
                        self.next_seq
                    );
                    self.reset();
                    return None;
                }

                self.buffer.extend_from_slice(&data);
                self.next_seq = (self.next_seq + 1) & 0x0F;

                if self.buffer.len() >= self.expected_len {
                    self.buffer.truncate(self.expected_len);
                    let result = std::mem::take(&mut self.buffer);
                    self.reset();
                    Some(result)
                } else {
                    None
                }
            }

            IsoTpFrame::FlowControl { .. } => {
                // Flow Control frames are handled by the sender side;
                // the assembler (receiver) just ignores them.
                None
            }
        }
    }
}

impl Default for IsoTpAssembler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_single_frame() {
        let data = vec![0x22, 0xF1, 0x90];
        let encoded = encode_single(&data).unwrap();
        assert_eq!(encoded[0], 0x03); // PCI = SF, len = 3

        let decoded = decode_frame(&encoded).unwrap();
        assert_eq!(decoded, IsoTpFrame::Single { data });
    }

    #[test]
    fn single_frame_too_long() {
        let data = vec![0u8; 8]; // 8 > 7
        assert!(encode_single(&data).is_err());
    }

    #[test]
    fn encode_decode_multi_frame() {
        let data: Vec<u8> = (0..20).collect();
        let config = IsoTpConfig::default();
        let frames = encode_multi(&data, &config).unwrap();

        // First frame + ceil((20 - 6) / 7) = 1 + 2 = 3 frames
        assert_eq!(frames.len(), 3);

        // Decode and reassemble.
        let mut assembler = IsoTpAssembler::new();
        for (i, raw) in frames.iter().enumerate() {
            let frame = decode_frame(raw).unwrap();
            let result = assembler.feed(frame);
            if i < frames.len() - 1 {
                assert!(result.is_none());
            } else {
                assert_eq!(result, Some(data.clone()));
            }
        }
    }

    #[test]
    fn decode_flow_control() {
        let raw = [0x30, 0x00, 0x0A]; // CTS, BS=0, STmin=10
        let frame = decode_frame(&raw).unwrap();
        assert_eq!(
            frame,
            IsoTpFrame::FlowControl {
                flag: FlowStatus::ContinueToSend,
                block_size: 0,
                st_min: 10,
            }
        );
    }

    #[test]
    fn assembler_single_frame() {
        let mut asm = IsoTpAssembler::new();
        let result = asm.feed(IsoTpFrame::Single {
            data: vec![0x62, 0xF1, 0x90, 0x41],
        });
        assert_eq!(result, Some(vec![0x62, 0xF1, 0x90, 0x41]));
    }

    #[test]
    fn assembler_resets_on_sequence_error() {
        let mut asm = IsoTpAssembler::new();
        asm.feed(IsoTpFrame::First {
            total_len: 20,
            data: vec![1, 2, 3, 4, 5, 6],
        });
        // Feed wrong sequence number.
        let result = asm.feed(IsoTpFrame::Consecutive {
            seq: 5, // expected 1
            data: vec![7, 8, 9, 10, 11, 12, 13],
        });
        assert!(result.is_none());
        assert!(!asm.active);
    }

    #[test]
    fn small_data_in_encode_multi_uses_single() {
        let data = vec![0x10, 0x03];
        let config = IsoTpConfig::default();
        let frames = encode_multi(&data, &config).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0][0] & 0xF0, PCI_SINGLE);
    }
}
