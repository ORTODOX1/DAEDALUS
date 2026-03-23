//! Real-time data types for live ECU parameter monitoring.

use serde::{Deserialize, Serialize};

/// A single live parameter reading from the ECU.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveParameter {
    /// Unique parameter identifier (e.g. "rpm", "boost_pressure").
    pub id: String,
    /// Human-readable parameter name.
    pub name: String,
    /// Current decoded physical value.
    pub value: f64,
    /// Engineering unit (e.g. "rpm", "kPa", "degC").
    pub unit: String,
    /// Minimum expected value (for gauge scaling).
    pub min: f64,
    /// Maximum expected value (for gauge scaling).
    pub max: f64,
    /// Timestamp in microseconds since session start.
    pub timestamp: u64,
}

/// Configuration for a single dashboard gauge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaugeConfig {
    /// ID of the parameter this gauge displays.
    pub parameter_id: String,
    /// Display label shown on the gauge.
    pub label: String,
    /// Gauge minimum scale value.
    pub min: f64,
    /// Gauge maximum scale value.
    pub max: f64,
    /// Low warning threshold (yellow zone start).
    pub warning_low: Option<f64>,
    /// High warning threshold (yellow zone start).
    pub warning_high: Option<f64>,
    /// Low critical threshold (red zone start).
    pub critical_low: Option<f64>,
    /// High critical threshold (red zone start).
    pub critical_high: Option<f64>,
    /// Engineering unit for display.
    pub unit: String,
}

/// Return a default set of dashboard gauges for truck/diesel ECU monitoring.
pub fn default_truck_gauges() -> Vec<GaugeConfig> {
    vec![
        GaugeConfig {
            parameter_id: "rpm".into(),
            label: "Engine RPM".into(),
            min: 0.0,
            max: 3000.0,
            warning_low: Some(400.0),
            warning_high: Some(2500.0),
            critical_low: Some(300.0),
            critical_high: Some(2800.0),
            unit: "rpm".into(),
        },
        GaugeConfig {
            parameter_id: "boost_pressure".into(),
            label: "Boost Pressure".into(),
            min: 0.0,
            max: 400.0,
            warning_low: None,
            warning_high: Some(320.0),
            critical_low: None,
            critical_high: Some(380.0),
            unit: "kPa".into(),
        },
        GaugeConfig {
            parameter_id: "coolant_temp".into(),
            label: "Coolant Temp".into(),
            min: -40.0,
            max: 150.0,
            warning_low: Some(-10.0),
            warning_high: Some(105.0),
            critical_low: Some(-20.0),
            critical_high: Some(115.0),
            unit: "\u{00B0}C".into(),
        },
        GaugeConfig {
            parameter_id: "oil_pressure".into(),
            label: "Oil Pressure".into(),
            min: 0.0,
            max: 1000.0,
            warning_low: Some(100.0),
            warning_high: Some(800.0),
            critical_low: Some(50.0),
            critical_high: Some(900.0),
            unit: "kPa".into(),
        },
        GaugeConfig {
            parameter_id: "fuel_rail_pressure".into(),
            label: "Fuel Rail Pressure".into(),
            min: 0.0,
            max: 2500.0,
            warning_low: Some(200.0),
            warning_high: Some(2200.0),
            critical_low: Some(100.0),
            critical_high: Some(2400.0),
            unit: "bar".into(),
        },
        GaugeConfig {
            parameter_id: "egt".into(),
            label: "Exhaust Gas Temp".into(),
            min: 0.0,
            max: 900.0,
            warning_low: None,
            warning_high: Some(700.0),
            critical_low: None,
            critical_high: Some(800.0),
            unit: "\u{00B0}C".into(),
        },
        GaugeConfig {
            parameter_id: "battery_voltage".into(),
            label: "Battery Voltage".into(),
            min: 0.0,
            max: 32.0,
            warning_low: Some(22.0),
            warning_high: Some(29.0),
            critical_low: Some(20.0),
            critical_high: Some(30.0),
            unit: "V".into(),
        },
        GaugeConfig {
            parameter_id: "vehicle_speed".into(),
            label: "Speed".into(),
            min: 0.0,
            max: 160.0,
            warning_low: None,
            warning_high: Some(130.0),
            critical_low: None,
            critical_high: Some(150.0),
            unit: "km/h".into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_gauges_has_expected_count() {
        let gauges = default_truck_gauges();
        assert_eq!(gauges.len(), 8);
    }

    #[test]
    fn default_gauges_contain_rpm() {
        let gauges = default_truck_gauges();
        assert!(gauges.iter().any(|g| g.parameter_id == "rpm"));
    }

    #[test]
    fn default_gauges_contain_boost() {
        let gauges = default_truck_gauges();
        assert!(gauges.iter().any(|g| g.parameter_id == "boost_pressure"));
    }

    #[test]
    fn live_parameter_serialization() {
        let param = LiveParameter {
            id: "rpm".into(),
            name: "Engine RPM".into(),
            value: 2150.5,
            unit: "rpm".into(),
            min: 0.0,
            max: 3000.0,
            timestamp: 1_000_000,
        };
        let json = serde_json::to_string(&param).unwrap();
        assert!(json.contains("2150.5"));
        let deserialized: LiveParameter = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "rpm");
    }

    #[test]
    fn gauge_config_serialization_roundtrip() {
        let gauge = GaugeConfig {
            parameter_id: "test".into(),
            label: "Test Gauge".into(),
            min: 0.0,
            max: 100.0,
            warning_low: Some(10.0),
            warning_high: Some(90.0),
            critical_low: None,
            critical_high: None,
            unit: "units".into(),
        };
        let json = serde_json::to_string(&gauge).unwrap();
        let deserialized: GaugeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.parameter_id, "test");
        assert_eq!(deserialized.warning_low, Some(10.0));
        assert!(deserialized.critical_low.is_none());
    }
}
