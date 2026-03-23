# Python Components — OPTIONAL

This directory is **NOT required** for the production app.
The shipped Tauri binary has zero Python dependency.

Python here is only for:
- **Power users**: local Ghidra integration for deep firmware analysis
- **Developers**: training custom ONNX models for map detection  
- **Testing**: test_harness.py for quick pipeline verification

All production AI goes through `crates/forge-ai/` (pure Rust HTTP to providers).
