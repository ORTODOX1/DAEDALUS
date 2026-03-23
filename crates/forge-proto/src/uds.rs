//! UDS (ISO 14229) — Unified Diagnostic Services.
//!
//! Provides request building, response parsing, and Negative Response
//! Code (NRC) handling for standard UDS services used in ECU diagnostics.

use forge_core::{DaedalusError, Result};
use serde::{Deserialize, Serialize};

/// Negative response service ID.
const NEGATIVE_RESPONSE_SID: u8 = 0x7F;

/// Positive response offset: response SID = request SID + 0x40.
const POSITIVE_RESPONSE_OFFSET: u8 = 0x40;

// ── UDS Service IDs ────────────────────────────────────────────────────

/// Standard UDS service identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum UDSService {
    /// Start/switch diagnostic session.
    DiagSessionControl = 0x10,
    /// Reset the ECU.
    ECUReset = 0x11,
    /// Security seed/key handshake.
    SecurityAccess = 0x27,
    /// Read data by 2-byte identifier.
    ReadDataByIdentifier = 0x22,
    /// Write data by 2-byte identifier.
    WriteDataByIdentifier = 0x2E,
    /// Read DTC information (sub-functions for status, snapshot, etc.).
    ReadDTCInformation = 0x19,
    /// Clear stored DTCs.
    ClearDTC = 0x14,
    /// Request a download transfer to the ECU.
    RequestDownload = 0x34,
    /// Request an upload transfer from the ECU.
    RequestUpload = 0x35,
    /// Transfer a data block during flash read/write.
    TransferData = 0x36,
    /// Signal end of transfer.
    TransferExit = 0x37,
    /// Start/stop/query a routine on the ECU.
    RoutineControl = 0x31,
}

impl UDSService {
    /// Return the raw byte value of this service.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Try to convert a raw byte into a known UDS service.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x10 => Some(Self::DiagSessionControl),
            0x11 => Some(Self::ECUReset),
            0x27 => Some(Self::SecurityAccess),
            0x22 => Some(Self::ReadDataByIdentifier),
            0x2E => Some(Self::WriteDataByIdentifier),
            0x19 => Some(Self::ReadDTCInformation),
            0x14 => Some(Self::ClearDTC),
            0x34 => Some(Self::RequestDownload),
            0x35 => Some(Self::RequestUpload),
            0x36 => Some(Self::TransferData),
            0x37 => Some(Self::TransferExit),
            0x31 => Some(Self::RoutineControl),
            _ => None,
        }
    }
}

// ── Diagnostic Sessions ────────────────────────────────────────────────

/// Standard UDS diagnostic session types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum DiagSession {
    /// Default session — limited services.
    Default = 0x01,
    /// Programming session — flash read/write enabled.
    Programming = 0x02,
    /// Extended session — full diagnostic access.
    Extended = 0x03,
}

impl DiagSession {
    /// Raw sub-function byte.
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

// ── Negative Response Codes ────────────────────────────────────────────

/// ISO 14229 Negative Response Codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum NRC {
    /// General reject — service refused without specific reason.
    GeneralReject = 0x10,
    /// The requested service is not implemented.
    ServiceNotSupported = 0x11,
    /// Sub-function not supported for the given service.
    SubFunctionNotSupported = 0x12,
    /// Conditions not met to execute the request.
    ConditionsNotCorrect = 0x22,
    /// Request data out of valid range.
    RequestOutOfRange = 0x31,
    /// Security access denied — not unlocked.
    SecurityAccessDenied = 0x33,
    /// Invalid security key supplied.
    InvalidKey = 0x35,
    /// Too many failed security access attempts.
    ExceededAttempts = 0x36,
    /// Must wait before retrying security access.
    TimeDelayNotExpired = 0x37,
    /// Upload/download request rejected by the ECU.
    UploadDownloadNotAccepted = 0x70,
}

impl NRC {
    /// Raw NRC byte.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Try to convert a raw NRC byte into a known variant.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x10 => Some(Self::GeneralReject),
            0x11 => Some(Self::ServiceNotSupported),
            0x12 => Some(Self::SubFunctionNotSupported),
            0x22 => Some(Self::ConditionsNotCorrect),
            0x31 => Some(Self::RequestOutOfRange),
            0x33 => Some(Self::SecurityAccessDenied),
            0x35 => Some(Self::InvalidKey),
            0x36 => Some(Self::ExceededAttempts),
            0x37 => Some(Self::TimeDelayNotExpired),
            0x70 => Some(Self::UploadDownloadNotAccepted),
            _ => None,
        }
    }

    /// Human-readable description of the NRC.
    pub fn description(self) -> &'static str {
        match self {
            Self::GeneralReject => "general reject",
            Self::ServiceNotSupported => "service not supported",
            Self::SubFunctionNotSupported => "sub-function not supported",
            Self::ConditionsNotCorrect => "conditions not correct",
            Self::RequestOutOfRange => "request out of range",
            Self::SecurityAccessDenied => "security access denied",
            Self::InvalidKey => "invalid key",
            Self::ExceededAttempts => "exceeded number of attempts",
            Self::TimeDelayNotExpired => "required time delay not expired",
            Self::UploadDownloadNotAccepted => "upload/download not accepted",
        }
    }
}

// ── Request / Response types ───────────────────────────────────────────

/// A structured UDS request before encoding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UDSRequest {
    /// The UDS service being invoked.
    pub service: UDSService,
    /// Optional sub-function byte.
    pub sub_function: Option<u8>,
    /// Additional request payload.
    pub data: Vec<u8>,
}

/// A parsed UDS response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UDSResponse {
    /// Response service ID (positive = SID+0x40, negative = 0x7F).
    pub service: u8,
    /// Response payload (excluding service byte).
    pub data: Vec<u8>,
    /// `true` for positive responses, `false` for negative (NRC).
    pub is_positive: bool,
}

// ── Encoding helpers ───────────────────────────────────────────────────

/// Build a raw UDS request payload.
///
/// Layout: `[SID, sub_function?, data...]`
pub fn build_request(service: UDSService, sub: Option<u8>, data: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + sub.is_some() as usize + data.len());
    buf.push(service.as_u8());
    if let Some(sf) = sub {
        buf.push(sf);
    }
    buf.extend_from_slice(data);
    buf
}

/// Parse a raw UDS response payload.
///
/// Handles both positive responses (`SID + 0x40 ...`) and
/// negative responses (`0x7F, SID, NRC`).
///
/// # Errors
/// Returns [`DaedalusError::ProtocolError`] for negative responses,
/// [`DaedalusError::ParseError`] for malformed data.
pub fn parse_response(raw: &[u8]) -> Result<UDSResponse> {
    if raw.is_empty() {
        return Err(DaedalusError::ParseError {
            message: "empty UDS response".into(),
            source: None,
        });
    }

    if raw[0] == NEGATIVE_RESPONSE_SID {
        if raw.len() < 3 {
            return Err(DaedalusError::ParseError {
                message: format!("negative response too short: {} bytes", raw.len()),
                source: None,
            });
        }
        let rejected_sid = raw[1];
        let nrc_byte = raw[2];
        let description = NRC::from_u8(nrc_byte)
            .map(|n| n.description())
            .unwrap_or("unknown NRC");

        return Err(DaedalusError::ProtocolError {
            message: description.to_string(),
            service: rejected_sid,
            nrc: nrc_byte,
        });
    }

    // Positive response: service byte is request SID + 0x40.
    Ok(UDSResponse {
        service: raw[0],
        data: raw[1..].to_vec(),
        is_positive: true,
    })
}

/// Build a Security Access seed request for the given level.
///
/// Odd levels request a seed, even levels send the key.
pub fn security_access_request(level: u8) -> Vec<u8> {
    build_request(UDSService::SecurityAccess, Some(level), &[])
}

/// Build a `ReadDTCInformation` request.
///
/// Sub-function `0x01` = report number of DTCs by status mask.
/// Status mask `0xFF` = all DTC statuses.
pub fn read_dtc_request() -> Vec<u8> {
    build_request(
        UDSService::ReadDTCInformation,
        Some(0x01),
        &[0xFF], // status mask: all
    )
}

/// Build a `DiagSessionControl` request for the given session.
pub fn diag_session_request(session: DiagSession) -> Vec<u8> {
    build_request(UDSService::DiagSessionControl, Some(session.as_u8()), &[])
}

/// Build a `ReadDataByIdentifier` request for a 2-byte DID.
pub fn read_data_by_id_request(did: u16) -> Vec<u8> {
    build_request(
        UDSService::ReadDataByIdentifier,
        None,
        &did.to_be_bytes(),
    )
}

/// Build a `ClearDTC` request (clear all stored DTCs).
///
/// Group = `0xFFFFFF` = all groups.
pub fn clear_dtc_request() -> Vec<u8> {
    build_request(UDSService::ClearDTC, None, &[0xFF, 0xFF, 0xFF])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_diag_session_control() {
        let req = diag_session_request(DiagSession::Extended);
        assert_eq!(req, vec![0x10, 0x03]);
    }

    #[test]
    fn build_security_access() {
        let req = security_access_request(0x01);
        assert_eq!(req, vec![0x27, 0x01]);
    }

    #[test]
    fn build_read_dtc() {
        let req = read_dtc_request();
        assert_eq!(req, vec![0x19, 0x01, 0xFF]);
    }

    #[test]
    fn build_read_data_by_id() {
        let req = read_data_by_id_request(0xF190);
        assert_eq!(req, vec![0x22, 0xF1, 0x90]);
    }

    #[test]
    fn parse_positive_response() {
        let raw = vec![0x62, 0xF1, 0x90, 0x41, 0x42, 0x43];
        let resp = parse_response(&raw).unwrap();
        assert!(resp.is_positive);
        assert_eq!(resp.service, 0x62); // 0x22 + 0x40
        assert_eq!(resp.data, vec![0xF1, 0x90, 0x41, 0x42, 0x43]);
    }

    #[test]
    fn parse_negative_response() {
        let raw = vec![0x7F, 0x22, 0x33]; // ReadDataById, SecurityAccessDenied
        let err = parse_response(&raw).unwrap_err();
        match err {
            DaedalusError::ProtocolError {
                service, nrc, message,
            } => {
                assert_eq!(service, 0x22);
                assert_eq!(nrc, 0x33);
                assert!(message.contains("security access denied"));
            }
            other => panic!("expected ProtocolError, got: {other:?}"),
        }
    }

    #[test]
    fn parse_empty_response() {
        assert!(parse_response(&[]).is_err());
    }

    #[test]
    fn nrc_descriptions() {
        assert_eq!(NRC::InvalidKey.description(), "invalid key");
        assert_eq!(NRC::ExceededAttempts.description(), "exceeded number of attempts");
    }

    #[test]
    fn clear_dtc_request_format() {
        let req = clear_dtc_request();
        assert_eq!(req, vec![0x14, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn uds_service_roundtrip() {
        let svc = UDSService::TransferData;
        assert_eq!(UDSService::from_u8(svc.as_u8()), Some(svc));
    }
}
