#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(clippy::all)]

mod commands;
mod state;

fn main() {
    tracing_subscriber::fmt::init();
    
    tauri::Builder::default()
        // .manage(state::AppState::new())
        // .invoke_handler(tauri::generate_handler![...])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
