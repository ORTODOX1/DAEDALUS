//! forge-proto — Automotive diagnostic protocol implementations.
//!
//! Provides ISO-TP transport, UDS diagnostics, SAE J1939 decoding,
//! and OBD-II PID encoding/decoding for ECU communication.
#![deny(clippy::all)]

pub mod isotp;
pub mod j1939;
pub mod obd2;
pub mod uds;
