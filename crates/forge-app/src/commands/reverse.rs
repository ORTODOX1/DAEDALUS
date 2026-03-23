use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct SerialPort {
    pub name: String,
}

#[derive(Serialize)]
pub struct MultimeterReading {
    #[serde(rename = "type")]
    pub reading_type: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Serialize)]
pub struct DriverStatus {
    pub name: String,
    pub installed: bool,
}

#[tauri::command]
pub fn list_serial_ports() -> Vec<String> {
    // Mock: in production, use serialport crate to enumerate
    vec!["COM3".into(), "COM4".into(), "COM5".into()]
}

#[tauri::command]
pub fn connect_multimeter(port: String) -> Result<bool, String> {
    // Mock: in production, open serial port with SCPI/proprietary protocol
    Ok(true)
}

#[tauri::command]
pub fn read_multimeter() -> Result<MultimeterReading, String> {
    // Mock reading
    Ok(MultimeterReading {
        reading_type: "voltage".into(),
        value: 3.3,
        unit: "V".into(),
    })
}

#[tauri::command]
pub fn load_pinout_db(ecu_type: String) -> Result<String, String> {
    // TODO: load from data/pinouts/{ecu_type}.json
    Ok(format!("Pinout DB loaded for {}", ecu_type))
}

#[derive(Deserialize)]
pub struct ReadingInput {
    pub pad_id: String,
    pub reading_type: String,
    pub value: f64,
}

#[tauri::command]
pub fn analyze_reverse(
    ecu_type: String,
    readings: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    // TODO: invoke forge-ai agents for multi-agent analysis
    Ok(vec![serde_json::json!({
        "agentRole": "Confidence Scorer",
        "content": format!("Analysis complete for {}. {} measurements processed.", ecu_type, readings.len()),
        "confidence": 0.85,
        "round": 1,
        "phase": "vote"
    })])
}

#[tauri::command]
pub fn check_drivers() -> Vec<DriverStatus> {
    vec![
        DriverStatus { name: "CP2102 USB-Serial".into(), installed: true },
        DriverStatus { name: "CH340 USB-Serial".into(), installed: false },
        DriverStatus { name: "FTDI USB-Serial".into(), installed: true },
        DriverStatus { name: "SocketCAN (Linux)".into(), installed: cfg!(target_os = "linux") },
    ]
}
