use serde::Serialize;

#[derive(Serialize)]
pub struct DTCCode {
    pub code: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub severity: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct J1939DTC {
    pub spn: u32,
    pub fmi: u8,
    pub name: String,
    pub description: String,
    pub category: String,
    pub severity: String,
    pub ecu: Vec<String>,
}

#[tauri::command]
pub fn read_dtc() -> Vec<DTCCode> {
    vec![
        DTCCode {
            code: "P0420".into(),
            name: "Catalyst System Efficiency".into(),
            description: "Catalyst system efficiency below threshold".into(),
            category: "emissions".into(),
            severity: "warning".into(),
            status: "stored".into(),
        },
    ]
}

#[tauri::command]
pub fn read_j1939_dtc() -> Vec<J1939DTC> {
    vec![
        J1939DTC {
            spn: 157,
            fmi: 1,
            name: "Fuel Rail Pressure".into(),
            description: "Common rail fuel pressure below normal".into(),
            category: "fuel_system".into(),
            severity: "critical".into(),
            ecu: vec!["EDC17".into(), "MD1".into()],
        },
    ]
}

#[tauri::command]
pub fn clear_dtc() -> Result<(), String> {
    Ok(())
}
