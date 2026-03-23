//! OpenAI (GPT-4) AI Provider — placeholder
//! 
//! Same trait, different HTTP endpoint and auth header.
//! Implementation follows the same pattern as Claude provider.

use async_trait::async_trait;
use crate::provider::AIProvider;
use crate::types::*;

pub struct OpenAIProvider {
    _api_key: String,
    _model: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            _api_key: api_key,
            _model: "gpt-4o".to_string(),
        }
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    fn name(&self) -> &str { "OpenAI (GPT-4)" }
    async fn health_check(&self) -> Result<bool, AIError> { todo!("Implement OpenAI health check") }
    async fn classify_map(&self, _req: &MapClassifyRequest) -> Result<MapClassification, AIError> { todo!() }
    async fn explain_dtc(&self, _req: &DTCExplainRequest) -> Result<String, AIError> { todo!() }
    async fn find_maps_hint(&self, _req: &BinaryFeaturesRequest) -> Result<Vec<MapHint>, AIError> { todo!() }
    async fn validate_safety(&self, _req: &SafetyCheckRequest) -> Result<SafetyReport, AIError> { todo!() }
    async fn chat(&self, _messages: &[ChatMessage]) -> Result<String, AIError> { todo!() }
}
