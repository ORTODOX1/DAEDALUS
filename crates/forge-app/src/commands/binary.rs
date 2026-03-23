#[tauri::command]
pub fn load_binary(_path: String) -> Result<String, String> {
    Err("Binary loading not yet implemented".into())
}
