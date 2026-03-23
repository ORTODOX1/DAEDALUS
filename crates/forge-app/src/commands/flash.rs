#[tauri::command]
pub fn read_ecu() -> Result<String, String> {
    Err("ECU reading not yet implemented. Connect an adapter first.".into())
}

#[tauri::command]
pub fn write_ecu() -> Result<String, String> {
    Err("ECU writing not yet implemented. Safety checks required.".into())
}
