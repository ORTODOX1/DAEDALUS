//! # forge-ai — AI Provider Abstraction Layer
//!
//! Zero local ML. All intelligence comes from cloud APIs.
//! Supports Claude, OpenAI, Gemini, Ollama (local), and custom endpoints.
//!
//! ## Key principle: NEVER send full binary files to APIs.
//! Only statistical features (entropy, gradients, ranges) go to the cloud.
//! All binary data stays on the user's machine.

#![deny(clippy::all)]

pub mod provider;
pub mod providers;
pub mod types;
pub mod prompts;

pub use provider::AIProvider;
pub use types::*;
