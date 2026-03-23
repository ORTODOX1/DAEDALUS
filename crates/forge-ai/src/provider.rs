//! AI Provider trait — the interface all providers implement.

use async_trait::async_trait;
use crate::types::*;

/// Core trait for AI providers.
/// Implement this for each API (Claude, OpenAI, Gemini, Ollama, etc.)
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Provider name for display ("Claude", "GPT-4", "Gemini", "Ollama")
    fn name(&self) -> &str;

    /// Test connectivity and API key validity
    async fn health_check(&self) -> Result<bool, AIError>;

    /// Classify a calibration map based on its statistical features.
    /// Input: axis ranges, data statistics, ECU type.
    /// Output: parameter name, units, category, confidence.
    async fn classify_map(&self, req: &MapClassifyRequest) -> Result<MapClassification, AIError>;

    /// Explain a DTC code in human-readable language.
    async fn explain_dtc(&self, req: &DTCExplainRequest) -> Result<String, AIError>;

    /// Analyze binary region features and suggest where maps might be.
    /// Does NOT receive the actual binary — only statistical features.
    async fn find_maps_hint(&self, req: &BinaryFeaturesRequest) -> Result<Vec<MapHint>, AIError>;

    /// Validate proposed modifications against safety constraints.
    async fn validate_safety(&self, req: &SafetyCheckRequest) -> Result<SafetyReport, AIError>;

    /// General chat for the AI assistant panel.
    async fn chat(&self, messages: &[ChatMessage]) -> Result<String, AIError>;
}

/// Registry of available providers. User selects active one in Settings.
pub struct ProviderRegistry {
    providers: Vec<Box<dyn AIProvider>>,
    active_index: usize,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            active_index: 0,
        }
    }

    pub fn register(&mut self, provider: Box<dyn AIProvider>) {
        self.providers.push(provider);
    }

    pub fn active(&self) -> Option<&dyn AIProvider> {
        self.providers.get(self.active_index).map(|p| p.as_ref())
    }

    pub fn set_active(&mut self, index: usize) -> Result<(), AIError> {
        if index < self.providers.len() {
            self.active_index = index;
            Ok(())
        } else {
            Err(AIError::ProviderNotFound)
        }
    }

    pub fn list_providers(&self) -> Vec<&str> {
        self.providers.iter().map(|p| p.name()).collect()
    }
}
