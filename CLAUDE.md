# CLAUDE.md — Daedalus

## What is this?
Daedalus (`daedalus`) — open-source AI-assisted ECU chip-tuning platform.
Desktop app for reading, analyzing, modifying, and writing automotive ECU firmware.

**Primary focus: commercial vehicles (trucks)**
- Fuel consumption optimization
- ТНВД (Common Rail fuel pump) programming
- Operating mode optimization
- Target ECUs: Bosch EDC17/MD1, Delphi DCM, Denso, Cummins CM2350/CM2450
- Protocols: J1939 (SAE, CAN 250 kbps), UDS, KWP2000, J1708/J1587
- DTC: both OBD2 (P/B/C/U codes) and J1939 (SPN/FMI)
- Key maps: IQ→Rail Pressure, Injection Timing, Pilot/Main/Post, Torque Limiter, Boost, EGR, Speed Limiter

## Tech Stack
- **Backend**: Rust 1.78+ (Tauri 2.x, tokio, serde, thiserror, reqwest for API calls)
- **Frontend**: React 19 + TypeScript 5.x + Vite + Tailwind CSS 4 + Zustand
- **AI**: Cloud APIs via Provider trait (Claude/OpenAI/Gemini/Ollama) — pure Rust HTTP
- **3D Maps**: Three.js via React Three Fiber
- **Charts**: Recharts
- **IPC**: Tauri invoke (Rust↔JS)
- **NO Python in production** — optional dev tool only

## Project Structure
```
daedalus/
├── crates/              # Rust backend (Cargo workspace)
│   ├── forge-core/      # Types, config, errors, project management
│   ├── forge-hal/       # Hardware: SocketCAN, SLCAN, gs_usb, K-Line, J2534
│   ├── forge-proto/     # Protocols: ISO-TP, UDS, KWP2000, OBD2, J1939, seed/key
│   ├── forge-flash/     # ECU read/write, checksum, compression, encryption
│   ├── forge-binary/    # Binary analysis: parsers, map finder, identify, diff
│   ├── forge-dtc/       # DTC database, read, clear, filter
│   ├── forge-live/      # Real-time data logging, gauges
│   ├── forge-ai/        # AI Provider abstraction (Claude/OpenAI/Gemini/local)
│   └── forge-app/       # Tauri app shell, IPC commands, state
├── frontend/            # React UI
│   └── src/
│       ├── components/  # layout/, connection/, flash/, editor/, dtc/, live/, ai/, common/
│       ├── hooks/       # useConnection, useFlash, useLiveData, useProject, useAI
│       ├── stores/      # Zustand: connection, project, editor, settings
│       ├── lib/         # Tauri invoke wrappers, utilities
│       └── types/       # TypeScript type definitions
├── python/              # OPTIONAL — only for power users with GPU
│   └── daedalus/    # Local Ghidra integration, training custom models
├── data/                # Static: DTC databases, ECU signatures, checksum defs
├── docs/                # Architecture docs, protocol specs, ECU profiles
└── scripts/             # Dev setup, socketcan config, build
```

**Python is OPTIONAL.** The production app is pure Rust+React.
Python exists only for: (a) power users who want local Ghidra analysis,
(b) training custom ML models, (c) development/testing.
The shipped binary has zero Python dependency.

## Architecture Rules
1. **ALL CAN/serial timing-critical code in Rust** — never in JS or Python
2. **ZERO heavy compute on laptop** — all AI/ML runs in cloud via Provider API
3. **No Python dependency in production** — Provider calls are pure Rust HTTP
4. **Frontend ↔ Rust**: Tauri IPC (`invoke`)
5. **Rust → Cloud**: HTTP to AI Provider (Claude/OpenAI/Gemini/local)
6. **Every ECU write requires**: backup + diff + safety check + user confirmation
7. **Undo/redo** for all map edits (command pattern in Zustand store)
8. **No auto-apply from AI** — AI suggests, human decides
9. **Minimum laptop spec**: 8 GB RAM, any modern CPU, USB 2.0, Linux/Win/Mac

## AI Provider System — Dual Mode (local-first / cloud-first)

Two operating profiles defined in `config/profiles.yaml`:
- **local-first**: Ollama LLM on GPU + ONNX map finder. 0 API calls. Full offline.
- **cloud-first**: Claude/OpenAI/Gemini APIs. No GPU needed. Needs internet.

```rust
// crates/forge-ai/src/provider.rs
pub trait AIProvider: Send + Sync {
    async fn classify_map(&self, req: MapClassifyRequest) -> Result<MapClassification>;
    async fn explain_dtc(&self, req: DTCExplainRequest) -> Result<String>;
    async fn find_maps(&self, req: BinaryAnalysisRequest) -> Result<Vec<MapCandidate>>;
    async fn validate_safety(&self, req: SafetyCheckRequest) -> Result<SafetyReport>;
    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String>;
}

// Implementations:
pub struct ClaudeProvider { api_key: String, model: String }
pub struct OpenAIProvider { api_key: String, model: String }
pub struct GeminiProvider { api_key: String, model: String }
pub struct OllamaProvider { endpoint: String, model: String }  // local LLM
pub struct MockProvider {}  // for testing without API keys
```

User configures provider in Settings. API keys stored in OS keychain (not plaintext).
Provider selection is runtime — switch between Claude/GPT/Gemini/local without restart.

What goes to the cloud: statistical features of binary regions (entropy, gradients,
axis candidates, data ranges) — NEVER the full binary file. ~2-5 KB per request.

What stays local: all binary data, all CAN communication, all file I/O.

## Coding Standards

### Rust
- `thiserror` for error enums, `anyhow` in application code
- All async with `tokio`; `serde` for serialization
- Clippy: `#![deny(clippy::all)]`
- `///` doc comments on all public items
- Tests in `#[cfg(test)]` modules

### TypeScript/React
- Functional components only, hooks for logic
- Zustand stores (no Redux), no context for global state
- All Tauri calls in typed wrappers (`lib/tauri.ts`)
- `React.lazy` for HexEditor, MapEditor3D
- Tailwind utilities, no CSS modules, no styled-components

### Python
- Type hints + mypy strict
- Pydantic models for all API schemas
- async/await for I/O
- pytest for tests

## Key Components to Build (Priority Order)

### P0 — Must Have First
1. `forge-hal`: SocketCAN driver + virtual CAN for testing
2. `forge-proto`: ISO-TP + UDS (DiagSession, SecurityAccess, ReadDTC, ReadDataById)
3. `forge-core`: Project structure, undo/redo, backup system
4. Frontend: App shell, sidebar nav, connection panel, status bar
5. Frontend: DTC viewer with filter/search
6. `forge-binary`: Raw binary parser, hex view data provider

### P1 — Core Features
7. `forge-binary/maps`: Heuristic map finder + axis detection
8. Frontend: Hex editor (virtual scroll, region highlighting)
9. Frontend: Map editor (2D table + Recharts line/area chart)
10. `forge-dtc`: Full DTC database (JSON), freeze frame parsing
11. `forge-ai`: Claude provider + Ollama provider + map classifier prompts
12. Frontend: AI assistant panel (chat UI)

### P2 — Full Pipeline
13. `forge-flash`: OBD flash read/write for Simos18 or MED17
14. `forge-flash/checksum`: CRC32, Bosch multipoint, auto-detect
15. Frontend: 3D map surface (React Three Fiber)
16. Frontend: Diff view (stock vs modified, side-by-side)
17. `forge-ai`: Safety constraint engine (hardcoded rules + AI review)
18. Frontend: Flash wizard with progress + verification

## Open Source Dependencies
- **MIT (safe to use)**: VW_Flash (checksums, crypto), python-udsoncan, python-can-isotp, CANable/candleLight firmware docs
- **LGPL (link OK)**: python-can (reference only, rewrite in Rust)
- **GPL-2.0 (careful)**: pyA2L (use as Python dependency only), RomRaider (reference for formats only)
- **Apache-2.0**: ReVa/reverse-engineering-assistant (direct integration OK)

## Testing Without Hardware
```bash
# Create virtual CAN interface
sudo modprobe vcan
sudo ip link add dev vcan0 type vcan
sudo ip link set up vcan0

# Run ECU simulator on vcan0 (from python/tests)
python -m daedalus.test.ecu_sim --interface vcan0

# Run app pointing to vcan0
DAEDALUS_CAN_INTERFACE=vcan0 cargo tauri dev
```

## Build Commands
```bash
# Full dev mode
./scripts/setup_dev.sh           # One-time: install Rust, Node deps
cd frontend && npm run dev &     # Vite HMR
cd crates/forge-app && cargo tauri dev  # Tauri dev

# With local LLM (optional, needs Ollama + GPU)
ollama pull phi3:3.8b-mini-4k-instruct-q4_K_M

# Production build
cargo tauri build

# Test harness (standalone, no Tauri)
cd tests/harness && python test_harness.py --mode demo
```

## Safety: CRITICAL
- Never write to ECU without creating backup first
- Never let AI auto-modify binary without human review
- Always show full diff before write confirmation
- Checksum correction is mandatory before write
- Safety limits are hard-coded per ECU type, not configurable by user
- Lambda < 0.78 under boost = BLOCK WRITE
- Timing beyond knock limit = BLOCK WRITE
- Log every write operation with timestamp + hash
