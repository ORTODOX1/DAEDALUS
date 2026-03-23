//! `forge-dtc` — Diagnostic Trouble Code database, parsing, and filtering
//! for OBD-II and SAE J1939 protocols.
//!
//! Loads DTC definitions from JSON files shipped with the application and
//! parses raw byte responses coming from the ECU (UDS service 0x19 for
//! OBD-II, PGN 0xFECA DM1 for J1939).
#![deny(clippy::all)]

pub mod database;
pub mod parser;
pub mod types;
