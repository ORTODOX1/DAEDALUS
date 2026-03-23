//! Safety validation for ECU calibration modifications.
//!
//! This module enforces **hard-coded** physical safety limits that cannot be
//! overridden by the user or the AI.  Any modification that violates a safety
//! rule **must** block the write operation.
//!
//! # Design philosophy
//!
//! - Safety limits are per-parameter, not per-ECU map.
//! - Rules are hard-coded (not user-configurable) to prevent accidental
//!   weakening.
//! - The AI assistant can *suggest* changes, but `validate()` is the final
//!   gate before any write reaches the wire.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A single safety constraint on a tuning parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyRule {
    /// Parameter name (e.g. `"lambda_min_boost"`, `"rail_pressure_max"`).
    pub parameter: String,
    /// Minimum allowed value (inclusive).
    pub min: f64,
    /// Maximum allowed value (inclusive).
    pub max: f64,
    /// Physical unit (e.g. `"bar"`, `"°C"`, `"km/h"`).
    pub unit: String,
    /// Severity level: `"critical"` blocks the write, `"warning"` alerts.
    pub severity: String,
    /// Human-readable explanation shown in the UI / safety report.
    pub message: String,
}

/// A single violation found during validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyViolation {
    /// The rule that was violated.
    pub rule: SafetyRule,
    /// The actual value that triggered the violation.
    pub actual_value: f64,
    /// Formatted message describing the violation.
    pub message: String,
}

/// The result of running safety validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyReport {
    /// `true` if all rules passed (no critical violations).
    pub passed: bool,
    /// Number of rules that were checked.
    pub rules_checked: usize,
    /// List of violations (may be empty).
    pub violations: Vec<SafetyViolation>,
}

// ── Default rule sets ────────────────────────────────────────────────

/// Return the default safety rules for truck / commercial-vehicle ECUs.
///
/// These limits are **hard-coded** and represent physical safety boundaries
/// that must never be exceeded.
pub fn default_truck_rules() -> Vec<SafetyRule> {
    vec![
        // ── Lambda / AFR ────────────────────────────────────────────
        SafetyRule {
            parameter: "lambda_min_boost".into(),
            min: 0.78,
            max: 1.50,
            unit: "lambda".into(),
            severity: "critical".into(),
            message: "Lambda below 0.78 under boost causes dangerously rich combustion, \
                      leading to excessive EGT and potential piston/turbo damage."
                .into(),
        },
        // ── Rail pressure ───────────────────────────────────────────
        SafetyRule {
            parameter: "rail_pressure_max_edc17".into(),
            min: 0.0,
            max: 2000.0,
            unit: "bar".into(),
            severity: "critical".into(),
            message: "EDC17 common-rail pressure must not exceed 2000 bar. \
                      Higher values risk injector / rail failure."
                .into(),
        },
        SafetyRule {
            parameter: "rail_pressure_max_cm2350".into(),
            min: 0.0,
            max: 2500.0,
            unit: "bar".into(),
            severity: "critical".into(),
            message: "CM2350 (Cummins) rail pressure must not exceed 2500 bar."
                .into(),
        },
        // ── Boost pressure ──────────────────────────────────────────
        SafetyRule {
            parameter: "boost_pressure_max".into(),
            min: 0.0,
            max: 3.5,
            unit: "bar abs".into(),
            severity: "critical".into(),
            message: "Maximum boost pressure of 3.5 bar absolute. \
                      Exceeding this risks turbocharger over-speed and compressor surge."
                .into(),
        },
        // ── Exhaust Gas Temperature ─────────────────────────────────
        SafetyRule {
            parameter: "egt_max".into(),
            min: 0.0,
            max: 850.0,
            unit: "°C".into(),
            severity: "critical".into(),
            message: "EGT must not exceed 850 °C. \
                      Higher temperatures cause turbo housing cracking and manifold failure."
                .into(),
        },
        // ── Timing advance ──────────────────────────────────────────
        SafetyRule {
            parameter: "timing_advance_max".into(),
            min: -10.0,
            max: 40.0,
            unit: "°BTDC".into(),
            severity: "critical".into(),
            message: "Injection timing beyond 40° BTDC risks detonation / knock damage. \
                      Negative values beyond -10° indicate post-injection out of range."
                .into(),
        },
        // ── Speed limiter ───────────────────────────────────────────
        SafetyRule {
            parameter: "speed_limiter_min".into(),
            min: 89.0,
            max: 180.0,
            unit: "km/h".into(),
            severity: "critical".into(),
            message: "EU truck speed limiter must remain at or above 89 km/h \
                      (legal minimum for ≥ 3.5 t vehicles). Removing it is illegal."
                .into(),
        },
        // ── Torque limit ────────────────────────────────────────────
        SafetyRule {
            parameter: "torque_limit_max".into(),
            min: 0.0,
            max: 3500.0,
            unit: "Nm".into(),
            severity: "critical".into(),
            message: "Maximum torque must not exceed 3500 Nm to protect the drivetrain \
                      (clutch, gearbox, driveshaft)."
                .into(),
        },
        // ── Smoke limiter ───────────────────────────────────────────
        SafetyRule {
            parameter: "smoke_limiter_min".into(),
            min: 30.0,
            max: 100.0,
            unit: "%".into(),
            severity: "warning".into(),
            message: "Smoke limiter should not be reduced below 30 %. \
                      Disabling it causes excessive particulate emissions."
                .into(),
        },
    ]
}

// ── Validation engine ────────────────────────────────────────────────

/// Validate a set of proposed modifications against the given safety rules.
///
/// `modifications` maps parameter names to their proposed values.
/// Only parameters present in both `rules` and `modifications` are checked.
///
/// The report's `passed` field is `false` if any `"critical"` rule is violated.
pub fn validate(
    rules: &[SafetyRule],
    modifications: &HashMap<String, f64>,
) -> SafetyReport {
    let mut violations = Vec::new();
    let mut rules_checked = 0usize;

    for rule in rules {
        if let Some(&value) = modifications.get(&rule.parameter) {
            rules_checked += 1;

            if value < rule.min || value > rule.max {
                let msg = format!(
                    "{}: value {:.2} {} is outside safe range [{:.2}, {:.2}] {}. {}",
                    rule.parameter, value, rule.unit, rule.min, rule.max, rule.unit, rule.message,
                );
                violations.push(SafetyViolation {
                    rule: rule.clone(),
                    actual_value: value,
                    message: msg,
                });
            }
        }
    }

    let has_critical = violations
        .iter()
        .any(|v| v.rule.severity == "critical");

    SafetyReport {
        passed: !has_critical,
        rules_checked,
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> Vec<SafetyRule> {
        default_truck_rules()
    }

    #[test]
    fn all_within_limits() {
        let mods: HashMap<String, f64> = HashMap::from([
            ("lambda_min_boost".into(), 0.85),
            ("boost_pressure_max".into(), 2.8),
            ("egt_max".into(), 750.0),
            ("speed_limiter_min".into(), 90.0),
        ]);

        let report = validate(&rules(), &mods);
        assert!(report.passed);
        assert_eq!(report.violations.len(), 0);
        assert_eq!(report.rules_checked, 4);
    }

    #[test]
    fn lambda_too_low_blocks() {
        let mods: HashMap<String, f64> = HashMap::from([
            ("lambda_min_boost".into(), 0.70), // below 0.78 — CRITICAL
        ]);

        let report = validate(&rules(), &mods);
        assert!(!report.passed);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rule.parameter, "lambda_min_boost");
        assert!((report.violations[0].actual_value - 0.70).abs() < 0.001);
    }

    #[test]
    fn egt_too_high_blocks() {
        let mods: HashMap<String, f64> = HashMap::from([
            ("egt_max".into(), 900.0), // above 850 — CRITICAL
        ]);

        let report = validate(&rules(), &mods);
        assert!(!report.passed);
        assert_eq!(report.violations.len(), 1);
    }

    #[test]
    fn speed_limiter_removal_blocks() {
        let mods: HashMap<String, f64> = HashMap::from([
            ("speed_limiter_min".into(), 50.0), // below 89 — CRITICAL
        ]);

        let report = validate(&rules(), &mods);
        assert!(!report.passed);
    }

    #[test]
    fn warning_does_not_block() {
        let mods: HashMap<String, f64> = HashMap::from([
            ("smoke_limiter_min".into(), 20.0), // below 30 — WARNING only
        ]);

        let report = validate(&rules(), &mods);
        assert!(report.passed); // Warnings don't block.
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rule.severity, "warning");
    }

    #[test]
    fn multiple_violations() {
        let mods: HashMap<String, f64> = HashMap::from([
            ("lambda_min_boost".into(), 0.65),
            ("boost_pressure_max".into(), 5.0),
            ("egt_max".into(), 1000.0),
            ("timing_advance_max".into(), 55.0),
        ]);

        let report = validate(&rules(), &mods);
        assert!(!report.passed);
        assert_eq!(report.violations.len(), 4);
        assert_eq!(report.rules_checked, 4);
    }

    #[test]
    fn unrelated_parameters_ignored() {
        let mods: HashMap<String, f64> = HashMap::from([
            ("some_custom_param".into(), 99999.0),
        ]);

        let report = validate(&rules(), &mods);
        assert!(report.passed);
        assert_eq!(report.rules_checked, 0);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn boundary_values_pass() {
        let mods: HashMap<String, f64> = HashMap::from([
            ("lambda_min_boost".into(), 0.78),  // exactly at min
            ("egt_max".into(), 850.0),           // exactly at max
            ("speed_limiter_min".into(), 89.0),  // exactly at min
        ]);

        let report = validate(&rules(), &mods);
        assert!(report.passed);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn default_rules_count() {
        let r = default_truck_rules();
        assert!(r.len() >= 9, "Expected at least 9 default rules");

        // All critical rules should have non-empty messages.
        for rule in &r {
            assert!(!rule.message.is_empty());
            assert!(!rule.parameter.is_empty());
            assert!(!rule.unit.is_empty());
        }
    }
}
