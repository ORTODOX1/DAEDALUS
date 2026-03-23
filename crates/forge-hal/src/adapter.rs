//! CAN adapter abstraction — the hardware-agnostic interface for all
//! CAN communication in Daedalus.

use async_trait::async_trait;
use forge_core::Result;
use serde::{Deserialize, Serialize};

/// A single CAN bus frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CANFrame {
    /// CAN arbitration ID (11-bit standard or 29-bit extended).
    pub id: u32,
    /// Frame payload (0..8 bytes for classic CAN).
    pub data: Vec<u8>,
    /// `true` for 29-bit extended IDs, `false` for 11-bit standard.
    pub extended: bool,
    /// Timestamp in microseconds since adapter connection.
    pub timestamp: u64,
}

impl CANFrame {
    /// Create a new standard (11-bit) CAN frame.
    pub fn new(id: u32, data: Vec<u8>) -> Self {
        Self {
            id,
            data,
            extended: false,
            timestamp: 0,
        }
    }

    /// Create a new extended (29-bit) CAN frame.
    pub fn new_extended(id: u32, data: Vec<u8>) -> Self {
        Self {
            id,
            data,
            extended: true,
            timestamp: 0,
        }
    }
}

/// Supported CAN adapter hardware types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdapterType {
    /// Linux SocketCAN (vcan, can0, etc.).
    SocketCAN,
    /// Serial Line CAN (Lawicel protocol).
    SLCAN,
    /// Direct USB adapters (candleLight / gs_usb).
    USB,
    /// SAE J2534 pass-through interface.
    J2534,
    /// Software mock for testing without hardware.
    Mock,
}

/// Information about a discovered CAN adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterInfo {
    /// Unique identifier for this adapter instance.
    pub id: String,
    /// Human-readable adapter name.
    pub name: String,
    /// Hardware type.
    pub adapter_type: AdapterType,
    /// Port or interface path (e.g. "vcan0", "COM3", "/dev/ttyUSB0").
    pub port: String,
    /// Whether the adapter is currently available for connection.
    pub available: bool,
}

/// Async trait for all CAN adapter implementations.
///
/// Every hardware driver (SocketCAN, SLCAN, USB, J2534) and the
/// mock adapter implement this trait, allowing protocol code to
/// be fully hardware-agnostic.
#[async_trait]
pub trait CANAdapter: Send + Sync {
    /// Open a connection to the adapter at the given baud rate.
    async fn connect(&mut self, baud_rate: u32) -> Result<()>;

    /// Close the connection and release hardware resources.
    async fn disconnect(&mut self) -> Result<()>;

    /// Transmit a CAN frame.
    async fn send(&self, frame: CANFrame) -> Result<()>;

    /// Receive a CAN frame, waiting up to `timeout_ms` milliseconds.
    ///
    /// Returns `Ok(None)` on timeout (no frame received).
    async fn receive(&self, timeout_ms: u64) -> Result<Option<CANFrame>>;

    /// Whether the adapter is currently connected.
    fn is_connected(&self) -> bool;

    /// Adapter metadata.
    fn info(&self) -> &AdapterInfo;
}

/// Enumerate available CAN adapters on this system.
///
/// Currently returns only the mock adapter. Real hardware discovery
/// (SocketCAN, USB) will be added in future iterations.
pub fn list_adapters() -> Vec<AdapterInfo> {
    vec![AdapterInfo {
        id: "mock-0".into(),
        name: "Mock ECU Simulator".into(),
        adapter_type: AdapterType::Mock,
        port: "virtual".into(),
        available: true,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_frame_standard() {
        let f = CANFrame::new(0x7E0, vec![0x02, 0x10, 0x03]);
        assert!(!f.extended);
        assert_eq!(f.id, 0x7E0);
        assert_eq!(f.data.len(), 3);
    }

    #[test]
    fn can_frame_extended() {
        let f = CANFrame::new_extended(0x18DA00F1, vec![0x01]);
        assert!(f.extended);
    }

    #[test]
    fn list_adapters_has_mock() {
        let adapters = list_adapters();
        assert!(!adapters.is_empty());
        assert!(adapters.iter().any(|a| a.adapter_type == AdapterType::Mock));
    }
}
