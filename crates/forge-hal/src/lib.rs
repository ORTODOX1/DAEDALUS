//! forge-hal — Hardware abstraction layer for CAN adapters.
//!
//! Provides a unified async trait for communicating with ECUs through
//! various CAN interfaces (SocketCAN, SLCAN, USB, J2534) and a mock
//! adapter for testing without physical hardware.
#![deny(clippy::all)]

pub mod adapter;
pub mod mock;
