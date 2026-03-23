#!/bin/bash
set -e
echo "🔧 Installing development dependencies..."

# Rust
if ! command -v cargo &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

# Tauri CLI
cargo install tauri-cli

# Node.js deps
cd frontend && npm install && cd ..

# Python deps
cd python && pip install -e ".[dev]" && cd ..

echo "✅ Dev environment ready. Run: cargo tauri dev"
