//! Claude (Anthropic) AI Provider
//!
//! Reference implementation. Other providers follow the same pattern.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use crate::provider::AIProvider;
use crate::types::*;
use crate::prompts;

pub struct ClaudeProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl ClaudeProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: "claude-sonnet-4-20250514".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    async fn send_message(&self, system: &str, user_msg: &str) -> Result<String, AIError> {
        let body = json!({
            "model": self.model,
            "max_tokens": 2048,
            "system": system,
            "messages": [{"role": "user", "content": user_msg}]
        });

        let resp = self.client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if resp.status() == 401 {
            return Err(AIError::InvalidApiKey);
        }
        if resp.status() == 429 {
            return Err(AIError::RateLimited { retry_after_secs: 30 });
        }
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(AIError::RequestFailed(text));
        }

        let data: serde_json::Value = resp.json().await?;
        let text = data["content"][0]["text"]
            .as_str()
            .ok_or_else(|| AIError::ParseError("No text in response".into()))?
            .to_string();

        Ok(text)
    }
}

#[async_trait]
impl AIProvider for ClaudeProvider {
    fn name(&self) -> &str {
        "Claude (Anthropic)"
    }

    async fn health_check(&self) -> Result<bool, AIError> {
        let resp = self.send_message(
            "Respond with exactly: OK",
            "Health check. Respond with exactly: OK"
        ).await?;
        Ok(resp.contains("OK"))
    }

    async fn classify_map(&self, req: &MapClassifyRequest) -> Result<MapClassification, AIError> {
        let system = prompts::SYSTEM_MAP_CLASSIFIER;
        let user_msg = serde_json::to_string_pretty(req)
            .map_err(|e| AIError::ParseError(e.to_string()))?;

        let response = self.send_message(system, &user_msg).await?;

        // Parse JSON from response (Claude might wrap in ```json blocks)
        let json_str = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        serde_json::from_str(json_str)
            .map_err(|e| AIError::ParseError(format!("Failed to parse classification: {e}")))
    }

    async fn explain_dtc(&self, req: &DTCExplainRequest) -> Result<String, AIError> {
        let system = prompts::SYSTEM_DTC_EXPLAINER;
        let user_msg = serde_json::to_string_pretty(req)
            .map_err(|e| AIError::ParseError(e.to_string()))?;

        self.send_message(system, &user_msg).await
    }

    async fn find_maps_hint(&self, req: &BinaryFeaturesRequest) -> Result<Vec<MapHint>, AIError> {
        let system = prompts::SYSTEM_MAP_FINDER;
        let user_msg = serde_json::to_string_pretty(req)
            .map_err(|e| AIError::ParseError(e.to_string()))?;

        let response = self.send_message(system, &user_msg).await?;

        let json_str = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        serde_json::from_str(json_str)
            .map_err(|e| AIError::ParseError(format!("Failed to parse map hints: {e}")))
    }

    async fn validate_safety(&self, req: &SafetyCheckRequest) -> Result<SafetyReport, AIError> {
        let system = prompts::SYSTEM_SAFETY_VALIDATOR;
        let user_msg = serde_json::to_string_pretty(req)
            .map_err(|e| AIError::ParseError(e.to_string()))?;

        let response = self.send_message(system, &user_msg).await?;

        let json_str = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        serde_json::from_str(json_str)
            .map_err(|e| AIError::ParseError(format!("Failed to parse safety report: {e}")))
    }

    async fn chat(&self, messages: &[ChatMessage]) -> Result<String, AIError> {
        let body = json!({
            "model": self.model,
            "max_tokens": 4096,
            "system": prompts::SYSTEM_ECU_ASSISTANT,
            "messages": messages.iter().map(|m| json!({
                "role": m.role,
                "content": m.content
            })).collect::<Vec<_>>()
        });

        let resp = self.client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(AIError::RequestFailed(text));
        }

        let data: serde_json::Value = resp.json().await?;
        data["content"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| AIError::ParseError("No text in response".into()))
    }
}
