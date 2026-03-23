//! forge-binary — Binary analysis: parsers, hex view, map finder, and diff.
//!
//! This crate provides the core binary manipulation layer for Daedalus.
//! It can load ECU firmware in Raw, Intel HEX, and Motorola S-Record formats,
//! expose data for the hex editor, heuristically locate calibration maps,
//! and compute diffs between stock and modified binaries.
#![deny(clippy::all)]

pub mod parser;
pub mod hex_view;
pub mod map_finder;
pub mod diff;
