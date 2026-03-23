//! Ollama AI Provider — for local LLM inference
//!
//! Connects to Ollama running on localhost (or remote server).
//! Use this when you want zero cloud dependency or have a powerful GPU.
//! Can also point to your remote server with a GPU for "private cloud" setup.

use async_trait::async_trait;
use crate::provider::AIProvider;
use crate::types::*;

pub struct OllamaProvider {
    _endpoint: String,   // e.g. "http://localhost:11434" or "http://your-server:11434"
    _model: String,      // e.g. "llama3.1:70b", "mixtral", "codestral"
}

impl OllamaProvider {
    pub fn new(endpoint: String, model: String) -> Self {
        Self {
            _endpoint: endpoint,
            _model: model,
        }
    }

    /// Point to your remote server (e.g. your desktop PC with RTX 5080)
    /// while running the app on a lightweight laptop
    pub fn remote(host: &str, port: u16, model: &str) -> Self {
        Self {
            _endpoint: format!("http://{}:{}", host, port),
            _model: model.to_string(),
        }
    }
}

#[async_trait]
impl AIProvider for OllamaProvider {
    fn name(&self) -> &str { "Ollama (Local/Remote LLM)" }
    async fn health_check(&self) -> Result<bool, AIError> { todo!("Implement Ollama health check") }
    async fn classify_map(&self, _req: &MapClassifyRequest) -> Result<MapClassification, AIError> { todo!() }
    async fn explain_dtc(&self, _req: &DTCExplainRequest) -> Result<String, AIError> { todo!() }
    async fn find_maps_hint(&self, _req: &BinaryFeaturesRequest) -> Result<Vec<MapHint>, AIError> { todo!() }
    async fn validate_safety(&self, _req: &SafetyCheckRequest) -> Result<SafetyReport, AIError> { todo!() }
    async fn chat(&self, _messages: &[ChatMessage]) -> Result<String, AIError> { todo!() }
}
