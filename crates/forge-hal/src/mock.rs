//! Mock CAN adapter and ECU simulator for testing without hardware.
//!
//! [`MockAdapter`] implements [`CANAdapter`] and routes all traffic through
//! a simulated ECU ([`MockECU`]) that responds to common UDS services.

use async_trait::async_trait;
use forge_core::{DaedalusError, Result};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::adapter::{AdapterInfo, AdapterType, CANAdapter, CANFrame};

/// A simulated ECU that responds to UDS diagnostic requests.
///
/// Supports a minimal set of services for development and testing:
/// - DiagSessionControl (0x10)
/// - SecurityAccess seed request (0x27)
/// - ReadDataByIdentifier (0x22)
/// - ReadDTCInformation (0x19)
#[derive(Debug)]
pub struct MockECU {
    /// Simulated ECU identifier.
    pub ecu_id: String,
    /// Whether the ECU is in an extended diagnostic session.
    pub extended_session: bool,
    /// Whether security access has been granted.
    pub security_unlocked: bool,
    /// Fixed seed for security access (for deterministic tests).
    seed: [u8; 4],
}

impl MockECU {
    /// Create a new mock ECU with default settings.
    pub fn new() -> Self {
        Self {
            ecu_id: "MOCK-ECU-SIM-001".into(),
            extended_session: false,
            security_unlocked: false,
            seed: [0xDE, 0xAD, 0xBE, 0xEF],
        }
    }

    /// Process a UDS request and return the response payload.
    pub fn process_request(&mut self, request: &[u8]) -> Vec<u8> {
        if request.is_empty() {
            return self.negative_response(0x00, 0x10); // GeneralReject
        }

        let sid = request[0];
        match sid {
            // DiagSessionControl
            0x10 => self.handle_diag_session(request),
            // SecurityAccess
            0x27 => self.handle_security_access(request),
            // ReadDataByIdentifier
            0x22 => self.handle_read_data_by_id(request),
            // ReadDTCInformation
            0x19 => self.handle_read_dtc(request),
            // ClearDTC
            0x14 => self.handle_clear_dtc(request),
            // Unsupported service
            _ => self.negative_response(sid, 0x11), // ServiceNotSupported
        }
    }

    fn handle_diag_session(&mut self, request: &[u8]) -> Vec<u8> {
        if request.len() < 2 {
            return self.negative_response(0x10, 0x12);
        }

        let session = request[1];
        match session {
            0x01 => {
                // Default session
                self.extended_session = false;
                self.security_unlocked = false;
                vec![0x50, 0x01, 0x00, 0x19, 0x01, 0xF4]
            }
            0x02 | 0x03 => {
                // Programming or Extended session
                self.extended_session = true;
                vec![0x50, session, 0x00, 0x19, 0x01, 0xF4]
            }
            _ => self.negative_response(0x10, 0x12), // SubFunctionNotSupported
        }
    }

    fn handle_security_access(&mut self, request: &[u8]) -> Vec<u8> {
        if request.len() < 2 {
            return self.negative_response(0x27, 0x12);
        }

        if !self.extended_session {
            return self.negative_response(0x27, 0x22); // ConditionsNotCorrect
        }

        let level = request[1];
        if level % 2 == 1 {
            // Odd level: seed request
            let mut resp = vec![0x67, level];
            resp.extend_from_slice(&self.seed);
            resp
        } else {
            // Even level: key response
            if request.len() < 6 {
                return self.negative_response(0x27, 0x31); // RequestOutOfRange
            }
            // Accept any key that is the bitwise NOT of the seed.
            let key = &request[2..6];
            let expected: Vec<u8> = self.seed.iter().map(|b| !b).collect();
            if key == expected.as_slice() {
                self.security_unlocked = true;
                vec![0x67, level]
            } else {
                self.negative_response(0x27, 0x35) // InvalidKey
            }
        }
    }

    fn handle_read_data_by_id(&self, request: &[u8]) -> Vec<u8> {
        if request.len() < 3 {
            return self.negative_response(0x22, 0x31);
        }

        let did = ((request[1] as u16) << 8) | request[2] as u16;
        match did {
            // DID 0xF190: VIN (Vehicle Identification Number)
            0xF190 => {
                let mut resp = vec![0x62, 0xF1, 0x90];
                resp.extend_from_slice(b"WDB9634031L123456");
                resp
            }
            // DID 0xF187: Spare part number
            0xF187 => {
                let mut resp = vec![0x62, 0xF1, 0x87];
                resp.extend_from_slice(b"03L906012A");
                resp
            }
            // DID 0xF191: ECU hardware version
            0xF191 => {
                let mut resp = vec![0x62, 0xF1, 0x91];
                resp.extend_from_slice(b"HW01.03");
                resp
            }
            // DID 0xF195: ECU software version
            0xF195 => {
                let mut resp = vec![0x62, 0xF1, 0x95];
                resp.extend_from_slice(b"SW02.14.00");
                resp
            }
            _ => self.negative_response(0x22, 0x31), // RequestOutOfRange
        }
    }

    fn handle_read_dtc(&self, request: &[u8]) -> Vec<u8> {
        if request.len() < 2 {
            return self.negative_response(0x19, 0x12);
        }

        let sub_function = request[1];
        match sub_function {
            // Sub 0x01: reportNumberOfDTCByStatusMask
            0x01 => {
                // Return: positive response, availability mask, format, count high, count low
                vec![0x59, 0x01, 0xFF, 0x01, 0x00, 0x03] // 3 DTCs
            }
            // Sub 0x02: reportDTCByStatusMask
            0x02 => {
                let mut resp = vec![0x59, 0x02, 0xFF];
                // DTC format: 3 bytes DTC + 1 byte status
                // P0300 — Random/Multiple Cylinder Misfire Detected
                resp.extend_from_slice(&[0x03, 0x00, 0x00, 0x2F]);
                // P0171 — System Too Lean (Bank 1)
                resp.extend_from_slice(&[0x01, 0x71, 0x00, 0x2F]);
                // P0420 — Catalyst Efficiency Below Threshold
                resp.extend_from_slice(&[0x04, 0x20, 0x00, 0x24]);
                resp
            }
            _ => self.negative_response(0x19, 0x12),
        }
    }

    fn handle_clear_dtc(&mut self, _request: &[u8]) -> Vec<u8> {
        // Positive response to ClearDTC
        vec![0x54]
    }

    /// Build a UDS negative response: [0x7F, rejected_SID, NRC].
    fn negative_response(&self, sid: u8, nrc: u8) -> Vec<u8> {
        vec![0x7F, sid, nrc]
    }
}

impl Default for MockECU {
    fn default() -> Self {
        Self::new()
    }
}

// ── MockAdapter ────────────────────────────────────────────────────────

/// A mock CAN adapter that simulates ECU communication in software.
///
/// All frames sent to the adapter are processed by [`MockECU`], and
/// responses are queued for the next [`receive`](CANAdapter::receive) call.
pub struct MockAdapter {
    info: AdapterInfo,
    connected: bool,
    ecu: Arc<Mutex<MockECU>>,
    /// Pending response frames (FIFO).
    rx_queue: Arc<Mutex<Vec<CANFrame>>>,
    /// CAN ID used for ECU responses.
    response_id: u32,
    /// Monotonically increasing timestamp counter.
    timestamp_us: Arc<Mutex<u64>>,
}

impl MockAdapter {
    /// Create a new mock adapter with default ECU simulator.
    pub fn new() -> Self {
        Self {
            info: AdapterInfo {
                id: "mock-0".into(),
                name: "Mock ECU Simulator".into(),
                adapter_type: AdapterType::Mock,
                port: "virtual".into(),
                available: true,
            },
            connected: false,
            ecu: Arc::new(Mutex::new(MockECU::new())),
            rx_queue: Arc::new(Mutex::new(Vec::new())),
            response_id: 0x7E8,
            timestamp_us: Arc::new(Mutex::new(0)),
        }
    }

    /// Access the underlying mock ECU for test setup.
    pub fn ecu(&self) -> &Arc<Mutex<MockECU>> {
        &self.ecu
    }

    async fn next_timestamp(&self) -> u64 {
        let mut ts = self.timestamp_us.lock().await;
        *ts += 1000; // 1 ms increments
        *ts
    }
}

impl Default for MockAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for MockAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockAdapter")
            .field("connected", &self.connected)
            .field("response_id", &format_args!("0x{:03X}", self.response_id))
            .finish()
    }
}

#[async_trait]
impl CANAdapter for MockAdapter {
    async fn connect(&mut self, _baud_rate: u32) -> Result<()> {
        if self.connected {
            return Err(DaedalusError::ConnectionError {
                message: "mock adapter already connected".into(),
                source: None,
            });
        }
        self.connected = true;
        tracing::info!("MockAdapter connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if !self.connected {
            return Err(DaedalusError::ConnectionError {
                message: "mock adapter not connected".into(),
                source: None,
            });
        }
        self.connected = false;
        self.rx_queue.lock().await.clear();
        tracing::info!("MockAdapter disconnected");
        Ok(())
    }

    async fn send(&self, frame: CANFrame) -> Result<()> {
        if !self.connected {
            return Err(DaedalusError::ConnectionError {
                message: "cannot send: not connected".into(),
                source: None,
            });
        }

        tracing::debug!(
            id = format_args!("0x{:03X}", frame.id),
            data = ?frame.data,
            "MockAdapter TX"
        );

        // Extract the UDS payload from the ISO-TP Single Frame.
        // For simplicity, we only handle single-frame requests in mock.
        let uds_data = if !frame.data.is_empty() {
            let pci_type = frame.data[0] & 0xF0;
            let pci_len = (frame.data[0] & 0x0F) as usize;
            if pci_type == 0x00 && pci_len > 0 && frame.data.len() > pci_len {
                // Single Frame: skip PCI byte
                &frame.data[1..1 + pci_len]
            } else {
                // Not a single frame or raw UDS — pass through
                &frame.data[..]
            }
        } else {
            &frame.data[..]
        };

        // Process through ECU simulator.
        let response_payload = {
            let mut ecu = self.ecu.lock().await;
            ecu.process_request(uds_data)
        };

        // Wrap response in ISO-TP Single Frame.
        let mut response_data = Vec::with_capacity(8);
        response_data.push(response_payload.len() as u8); // SF PCI
        response_data.extend_from_slice(&response_payload);
        // Pad to 8 bytes.
        while response_data.len() < 8 {
            response_data.push(0xCC);
        }

        let ts = self.next_timestamp().await;
        let response_frame = CANFrame {
            id: self.response_id,
            data: response_data,
            extended: false,
            timestamp: ts,
        };

        self.rx_queue.lock().await.push(response_frame);
        Ok(())
    }

    async fn receive(&self, timeout_ms: u64) -> Result<Option<CANFrame>> {
        if !self.connected {
            return Err(DaedalusError::ConnectionError {
                message: "cannot receive: not connected".into(),
                source: None,
            });
        }

        // Check for queued responses first.
        {
            let mut queue = self.rx_queue.lock().await;
            if !queue.is_empty() {
                return Ok(Some(queue.remove(0)));
            }
        }

        // No queued frame — wait (simulated timeout).
        if timeout_ms > 0 {
            let wait = timeout_ms.min(100); // cap mock wait
            tokio::time::sleep(tokio::time::Duration::from_millis(wait)).await;
        }

        // Check again after sleep.
        let mut queue = self.rx_queue.lock().await;
        if !queue.is_empty() {
            Ok(Some(queue.remove(0)))
        } else {
            Ok(None)
        }
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn info(&self) -> &AdapterInfo {
        &self.info
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_adapter_connect_disconnect() {
        let mut adapter = MockAdapter::new();
        assert!(!adapter.is_connected());

        adapter.connect(500_000).await.unwrap();
        assert!(adapter.is_connected());

        adapter.disconnect().await.unwrap();
        assert!(!adapter.is_connected());
    }

    #[tokio::test]
    async fn mock_adapter_double_connect_fails() {
        let mut adapter = MockAdapter::new();
        adapter.connect(500_000).await.unwrap();
        assert!(adapter.connect(500_000).await.is_err());
    }

    #[tokio::test]
    async fn mock_adapter_send_receive_not_connected() {
        let adapter = MockAdapter::new();
        let frame = CANFrame::new(0x7E0, vec![0x02, 0x10, 0x03]);
        assert!(adapter.send(frame).await.is_err());
        assert!(adapter.receive(100).await.is_err());
    }

    #[tokio::test]
    async fn mock_ecu_diag_session_control() {
        let mut adapter = MockAdapter::new();
        adapter.connect(500_000).await.unwrap();

        // Send DiagSessionControl Extended (0x10, 0x03) as ISO-TP SF
        let request = CANFrame::new(0x7E0, vec![0x02, 0x10, 0x03, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC]);
        adapter.send(request).await.unwrap();

        let response = adapter.receive(1000).await.unwrap().unwrap();
        assert_eq!(response.id, 0x7E8);
        // Check positive response: SF PCI + [0x50, 0x03, ...]
        assert_eq!(response.data[1], 0x50); // positive response SID
        assert_eq!(response.data[2], 0x03); // extended session
    }

    #[tokio::test]
    async fn mock_ecu_read_vin() {
        let mut adapter = MockAdapter::new();
        adapter.connect(500_000).await.unwrap();

        // ReadDataByIdentifier 0xF190 (VIN)
        let request = CANFrame::new(0x7E0, vec![0x03, 0x22, 0xF1, 0x90, 0xCC, 0xCC, 0xCC, 0xCC]);
        adapter.send(request).await.unwrap();

        let response = adapter.receive(1000).await.unwrap().unwrap();
        // Positive response for RDBI: 0x62
        assert_eq!(response.data[1], 0x62);
    }

    #[tokio::test]
    async fn mock_ecu_security_access_seed() {
        let mut adapter = MockAdapter::new();
        adapter.connect(500_000).await.unwrap();

        // First switch to extended session
        let session_req = CANFrame::new(0x7E0, vec![0x02, 0x10, 0x03, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC]);
        adapter.send(session_req).await.unwrap();
        adapter.receive(1000).await.unwrap();

        // Request security seed (level 0x01)
        let seed_req = CANFrame::new(0x7E0, vec![0x02, 0x27, 0x01, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC]);
        adapter.send(seed_req).await.unwrap();

        let response = adapter.receive(1000).await.unwrap().unwrap();
        assert_eq!(response.data[1], 0x67); // positive SA response
        assert_eq!(response.data[2], 0x01); // level
    }

    #[tokio::test]
    async fn mock_ecu_unsupported_service() {
        let mut adapter = MockAdapter::new();
        adapter.connect(500_000).await.unwrap();

        // Send unknown service 0xAA
        let request = CANFrame::new(0x7E0, vec![0x01, 0xAA, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC]);
        adapter.send(request).await.unwrap();

        let response = adapter.receive(1000).await.unwrap().unwrap();
        assert_eq!(response.data[1], 0x7F); // negative response
        assert_eq!(response.data[2], 0xAA); // rejected SID
        assert_eq!(response.data[3], 0x11); // ServiceNotSupported
    }

    #[tokio::test]
    async fn mock_ecu_read_dtc() {
        let mut adapter = MockAdapter::new();
        adapter.connect(500_000).await.unwrap();

        // ReadDTCInformation sub=0x01 (report count)
        let request = CANFrame::new(0x7E0, vec![0x03, 0x19, 0x01, 0xFF, 0xCC, 0xCC, 0xCC, 0xCC]);
        adapter.send(request).await.unwrap();

        let response = adapter.receive(1000).await.unwrap().unwrap();
        assert_eq!(response.data[1], 0x59); // positive ReadDTC response
    }

    #[test]
    fn mock_ecu_process_empty_request() {
        let mut ecu = MockECU::new();
        let resp = ecu.process_request(&[]);
        assert_eq!(resp[0], 0x7F); // negative response
    }

    #[tokio::test]
    async fn receive_timeout_returns_none() {
        let mut adapter = MockAdapter::new();
        adapter.connect(500_000).await.unwrap();
        // No frame sent, should timeout.
        let result = adapter.receive(10).await.unwrap();
        assert!(result.is_none());
    }
}
