//! Types for AI provider requests and responses.

use serde::{Deserialize, Serialize};

/// Error types for AI operations
#[derive(Debug, thiserror::Error)]
pub enum AIError {
    #[error("API request failed: {0}")]
    RequestFailed(String),
    #[error("Invalid API key")]
    InvalidApiKey,
    #[error("Rate limited — retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
    #[error("Provider not found")]
    ProviderNotFound,
    #[error("Provider not configured — add API key in Settings")]
    NotConfigured,
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
}

// ─── Map Classification ───

#[derive(Debug, Serialize)]
pub struct MapClassifyRequest {
    pub ecu_type: String,           // "Bosch MED17.5.2"
    pub ecu_fuel: String,           // "gasoline_di" | "diesel" | "gasoline_port"
    pub dimensions: (usize, usize), // (16, 16)
    pub x_axis_samples: Vec<f64>,   // First 5 + last 3 values
    pub y_axis_samples: Vec<f64>,
    pub x_range: (f64, f64),        // (min, max)
    pub y_range: (f64, f64),
    pub data_range: (f64, f64),
    pub data_mean: f64,
    pub data_std: f64,
    pub data_monotonic: bool,       // Is data monotonically increasing?
    pub gradient_direction: String, // "positive", "negative", "mixed"
}

#[derive(Debug, Deserialize)]
pub struct MapClassification {
    pub parameter_name: String,     // "Injection timing main"
    pub x_axis_name: String,        // "Engine speed"
    pub x_axis_unit: String,        // "RPM"
    pub y_axis_name: String,        // "Load"
    pub y_axis_unit: String,        // "mg/stroke"
    pub data_unit: String,          // "°BTDC"
    pub category: MapCategory,
    pub confidence: f64,            // 0.0–1.0
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum MapCategory {
    Fuel,
    Boost,
    Timing,
    Torque,
    Emissions,
    Limiters,
    Transmission,
    Other,
}

// ─── DTC Explanation ───

#[derive(Debug, Serialize)]
pub struct DTCExplainRequest {
    pub dtc_code: String,
    pub ecu_type: String,
    pub freeze_frame: serde_json::Value,
    pub language: String,           // "ru", "en", "de"
}

// ─── Binary Analysis Hints ───

#[derive(Debug, Serialize)]
pub struct BinaryFeaturesRequest {
    pub ecu_type: String,
    pub total_size: usize,
    pub regions: Vec<RegionFeatures>,
}

#[derive(Debug, Serialize)]
pub struct RegionFeatures {
    pub offset: usize,
    pub size: usize,
    pub entropy: f64,               // Shannon entropy (0–8)
    pub mean: f64,
    pub std_dev: f64,
    pub zero_ratio: f64,            // Ratio of 0x00/0xFF bytes
    pub has_monotonic_sequences: bool,
    pub suspected_type: String,     // "code", "data", "calibration", "empty"
}

#[derive(Debug, Deserialize)]
pub struct MapHint {
    pub likely_offset: usize,
    pub likely_size: usize,
    pub suggested_dimensions: (usize, usize),
    pub confidence: f64,
    pub reasoning: String,
}

// ─── Safety Validation ───

#[derive(Debug, Serialize)]
pub struct SafetyCheckRequest {
    pub ecu_type: String,
    pub ecu_fuel: String,
    pub modifications: Vec<MapModification>,
}

#[derive(Debug, Serialize)]
pub struct MapModification {
    pub map_name: String,
    pub category: MapCategory,
    pub original_range: (f64, f64),
    pub modified_range: (f64, f64),
    pub max_change_percent: f64,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct SafetyReport {
    pub overall_safe: bool,
    pub risk_level: RiskLevel,
    pub checks: Vec<SafetyCheck>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Safe,
    Caution,
    Warning,
    Danger,
    Blocked,
}

#[derive(Debug, Deserialize)]
pub struct SafetyCheck {
    pub name: String,
    pub passed: bool,
    pub message: String,
}

// ─── Chat ───

#[derive(Debug, Serialize, Clone)]
pub struct ChatMessage {
    pub role: String,     // "user" | "assistant"
    pub content: String,
}
