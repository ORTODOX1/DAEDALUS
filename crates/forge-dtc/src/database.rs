//! DTC database — load, search, and filter OBD-II / J1939 codes.
//!
//! Databases are shipped as JSON files under `data/dtc/` and loaded at
//! application startup.

use std::collections::HashMap;
use std::path::Path;

use forge_core::{DaedalusError, Result};

use crate::types::{DTCSeverity, J1939Code, OBD2Code, DTCStatus};

// ── JSON structures matching the on-disk format ──────────────────────

/// Intermediate representation for `obd2_standard.json` entries.
#[derive(Debug, serde::Deserialize)]
struct OBD2Entry {
    description: String,
    category: String,
    severity: String,
}

/// Intermediate representation for `j1939_spn_fmi.json` entries.
#[derive(Debug, serde::Deserialize)]
struct J1939Entry {
    spn: u32,
    fmi: u8,
    name: String,
    description: String,
    category: String,
    severity: String,
    ecu: Vec<String>,
}

/// Top-level wrapper for the J1939 JSON file.
#[derive(Debug, serde::Deserialize)]
struct J1939File {
    codes: Vec<J1939Entry>,
    // We ignore `description`, `standard`, `version`, `fmi_definitions` here.
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Map a free-form severity string to our enum.
fn parse_severity(s: &str) -> DTCSeverity {
    match s.to_lowercase().as_str() {
        "critical" | "high" => DTCSeverity::Critical,
        "warning" | "moderate" => DTCSeverity::Warning,
        _ => DTCSeverity::Info,
    }
}

// ── Database ─────────────────────────────────────────────────────────

/// In-memory DTC database holding both OBD-II and J1939 codes.
#[derive(Debug, Clone)]
pub struct DTCDatabase {
    /// OBD-II codes keyed by their 5-char code (e.g. `"P0420"`).
    pub obd2_codes: HashMap<String, OBD2Code>,
    /// J1939 codes keyed by `(SPN, FMI)`.
    pub j1939_codes: HashMap<(u32, u8), J1939Code>,
}

impl DTCDatabase {
    /// Create an empty database.
    pub fn new() -> Self {
        Self {
            obd2_codes: HashMap::new(),
            j1939_codes: HashMap::new(),
        }
    }

    /// Load both databases from the standard `data/dtc/` directory.
    ///
    /// `base_dir` should point to the `data/dtc/` folder (or wherever the
    /// two JSON files live).
    pub fn load_all(base_dir: &Path) -> Result<Self> {
        let obd2 = Self::load_obd2(&base_dir.join("obd2_standard.json"))?;
        let j1939 = Self::load_j1939(&base_dir.join("j1939_spn_fmi.json"))?;
        Ok(Self {
            obd2_codes: obd2,
            j1939_codes: j1939,
        })
    }

    /// Parse `obd2_standard.json` into a code-keyed map.
    pub fn load_obd2(path: &Path) -> Result<HashMap<String, OBD2Code>> {
        let data = std::fs::read_to_string(path).map_err(|e| DaedalusError::IoError {
            message: format!("Cannot read OBD2 database: {e}"),
            path: Some(path.to_path_buf()),
            source: Some(e),
        })?;

        // The JSON is `{ "_comment": "...", "P0420": { ... }, ... }`.
        let raw: HashMap<String, serde_json::Value> = serde_json::from_str(&data)?;
        let mut map = HashMap::new();

        for (code, value) in &raw {
            // Skip metadata keys (e.g. `_comment`).
            if code.starts_with('_') {
                continue;
            }
            let entry: OBD2Entry = serde_json::from_value(value.clone()).map_err(|e| {
                DaedalusError::ParseError {
                    message: format!("Bad OBD2 entry '{code}': {e}"),
                    source: Some(Box::new(e)),
                }
            })?;

            map.insert(
                code.clone(),
                OBD2Code {
                    code: code.clone(),
                    name: entry.description.clone(),
                    description: entry.description,
                    category: entry.category,
                    severity: parse_severity(&entry.severity),
                    status: DTCStatus::Stored,
                },
            );
        }

        tracing::info!(count = map.len(), "Loaded OBD2 codes");
        Ok(map)
    }

    /// Parse `j1939_spn_fmi.json` into an SPN/FMI-keyed map.
    pub fn load_j1939(path: &Path) -> Result<HashMap<(u32, u8), J1939Code>> {
        let data = std::fs::read_to_string(path).map_err(|e| DaedalusError::IoError {
            message: format!("Cannot read J1939 database: {e}"),
            path: Some(path.to_path_buf()),
            source: Some(e),
        })?;

        let file: J1939File = serde_json::from_str(&data)?;
        let mut map = HashMap::new();

        for entry in file.codes {
            let code = J1939Code {
                spn: entry.spn,
                fmi: entry.fmi,
                name: entry.name,
                description: entry.description,
                category: entry.category,
                severity: parse_severity(&entry.severity),
                ecu_types: entry.ecu,
            };
            map.insert((code.spn, code.fmi), code);
        }

        tracing::info!(count = map.len(), "Loaded J1939 codes");
        Ok(map)
    }

    // ── Search & filter ──────────────────────────────────────────────

    /// Full-text search across code, name, and description (case-insensitive).
    ///
    /// Returns matching OBD-II codes.  For J1939, use [`search_j1939`].
    pub fn search(&self, query: &str) -> Vec<&OBD2Code> {
        let q = query.to_lowercase();
        self.obd2_codes
            .values()
            .filter(|c| {
                c.code.to_lowercase().contains(&q)
                    || c.name.to_lowercase().contains(&q)
                    || c.description.to_lowercase().contains(&q)
            })
            .collect()
    }

    /// Full-text search across J1939 codes.
    pub fn search_j1939(&self, query: &str) -> Vec<&J1939Code> {
        let q = query.to_lowercase();
        self.j1939_codes
            .values()
            .filter(|c| {
                c.name.to_lowercase().contains(&q)
                    || c.description.to_lowercase().contains(&q)
                    || c.spn.to_string().contains(&q)
            })
            .collect()
    }

    /// Filter OBD-II codes by functional category.
    pub fn filter_by_category(&self, category: &str) -> Vec<&OBD2Code> {
        let cat = category.to_lowercase();
        self.obd2_codes
            .values()
            .filter(|c| c.category.to_lowercase() == cat)
            .collect()
    }

    /// Filter OBD-II codes by severity level.
    pub fn filter_by_severity(&self, severity: DTCSeverity) -> Vec<&OBD2Code> {
        self.obd2_codes
            .values()
            .filter(|c| c.severity == severity)
            .collect()
    }

    /// Filter J1939 codes by functional category.
    pub fn filter_j1939_by_category(&self, category: &str) -> Vec<&J1939Code> {
        let cat = category.to_lowercase();
        self.j1939_codes
            .values()
            .filter(|c| c.category.to_lowercase() == cat)
            .collect()
    }

    /// Filter J1939 codes by severity level.
    pub fn filter_j1939_by_severity(&self, severity: DTCSeverity) -> Vec<&J1939Code> {
        self.j1939_codes
            .values()
            .filter(|c| c.severity == severity)
            .collect()
    }

    /// Look up an OBD-II code by its string key (e.g. `"P0420"`).
    pub fn get_obd2(&self, code: &str) -> Option<&OBD2Code> {
        self.obd2_codes.get(code)
    }

    /// Look up a J1939 code by SPN + FMI.
    pub fn get_j1939(&self, spn: u32, fmi: u8) -> Option<&J1939Code> {
        self.j1939_codes.get(&(spn, fmi))
    }
}

impl Default for DTCDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn data_dir() -> PathBuf {
        // Assumes tests run from workspace root.
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("data")
            .join("dtc")
    }

    #[test]
    fn load_obd2_from_fixture() {
        let path = data_dir().join("obd2_standard.json");
        if !path.exists() {
            eprintln!("Skipping — fixture not found at {}", path.display());
            return;
        }
        let codes = DTCDatabase::load_obd2(&path).unwrap();
        assert!(codes.contains_key("P0420"));
        assert!(codes.contains_key("P2463"));
        // _comment key must be skipped
        assert!(!codes.contains_key("_comment"));
    }

    #[test]
    fn load_j1939_from_fixture() {
        let path = data_dir().join("j1939_spn_fmi.json");
        if !path.exists() {
            eprintln!("Skipping — fixture not found at {}", path.display());
            return;
        }
        let codes = DTCDatabase::load_j1939(&path).unwrap();
        assert!(codes.contains_key(&(91, 3)));
        assert!(codes.contains_key(&(157, 0)));
        let oil = codes.get(&(100, 3)).unwrap();
        assert_eq!(oil.name, "Engine Oil Pressure");
        assert_eq!(oil.severity, DTCSeverity::Critical);
    }

    #[test]
    fn load_all_and_search() {
        let dir = data_dir();
        if !dir.join("obd2_standard.json").exists() {
            return;
        }
        let db = DTCDatabase::load_all(&dir).unwrap();

        // OBD2 search
        let results = db.search("catalyst");
        assert!(!results.is_empty());
        assert!(results.iter().any(|c| c.code == "P0420"));

        // J1939 search
        let j_results = db.search_j1939("oil pressure");
        assert!(!j_results.is_empty());
    }

    #[test]
    fn filter_by_category() {
        let dir = data_dir();
        if !dir.join("obd2_standard.json").exists() {
            return;
        }
        let db = DTCDatabase::load_all(&dir).unwrap();

        let emissions = db.filter_by_category("emissions");
        assert!(!emissions.is_empty());
        assert!(emissions.iter().all(|c| c.category == "emissions"));

        let fuel = db.filter_j1939_by_category("fuel_system");
        assert!(!fuel.is_empty());
    }

    #[test]
    fn filter_by_severity() {
        let dir = data_dir();
        if !dir.join("j1939_spn_fmi.json").exists() {
            return;
        }
        let db = DTCDatabase::load_all(&dir).unwrap();

        let critical = db.filter_j1939_by_severity(DTCSeverity::Critical);
        assert!(!critical.is_empty());
        assert!(critical.iter().all(|c| c.severity == DTCSeverity::Critical));
    }

    #[test]
    fn get_by_key() {
        let dir = data_dir();
        if !dir.join("obd2_standard.json").exists() {
            return;
        }
        let db = DTCDatabase::load_all(&dir).unwrap();

        assert!(db.get_obd2("P0420").is_some());
        assert!(db.get_obd2("XXXXX").is_none());
        assert!(db.get_j1939(157, 0).is_some());
        assert!(db.get_j1939(0, 0).is_none());
    }
}
