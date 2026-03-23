use serde::Serialize;
use tauri::State;
use crate::state::AppState;

#[derive(Serialize)]
pub struct AdapterInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub adapter_type: String,
    pub port: String,
    pub available: bool,
}

#[tauri::command]
pub fn list_adapters() -> Vec<AdapterInfo> {
    vec![
        AdapterInfo {
            id: "vcan0".into(),
            name: "Virtual CAN (vcan0)".into(),
            adapter_type: "socketcan".into(),
            port: "vcan0".into(),
            available: true,
        },
        AdapterInfo {
            id: "canable".into(),
            name: "CANable 2.0".into(),
            adapter_type: "usb".into(),
            port: "COM3".into(),
            available: false,
        },
    ]
}

#[tauri::command]
pub fn connect_adapter(
    adapter_id: String,
    baud_rate: u32,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let mut connected = state.connected.lock().map_err(|e| e.to_string())?;
    *connected = true;
    Ok(format!("Connected to {} at {} baud", adapter_id, baud_rate))
}

#[tauri::command]
pub fn disconnect_adapter(state: State<'_, AppState>) -> Result<(), String> {
    let mut connected = state.connected.lock().map_err(|e| e.to_string())?;
    *connected = false;
    Ok(())
}
