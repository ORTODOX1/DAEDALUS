//! forge-flash — ECU flash read/write, checksums, backups, and safety validation.
//!
//! This crate provides the safety-critical layer between binary editing and
//! actual ECU writes.  Every write path **must** pass through backup creation,
//! checksum correction, and safety validation before bytes hit the wire.
#![deny(clippy::all)]

pub mod checksum;
pub mod backup;
pub mod safety;
