#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(clippy::all)]

mod commands;
mod state;

use state::AppState;

fn main() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            // Connection
            commands::connection::list_adapters,
            commands::connection::connect_adapter,
            commands::connection::disconnect_adapter,
            // DTC
            commands::dtc::read_dtc,
            commands::dtc::read_j1939_dtc,
            commands::dtc::clear_dtc,
            // AI
            commands::ai::ai_chat,
            // Binary
            commands::binary::load_binary,
            // Flash
            commands::flash::read_ecu,
            commands::flash::write_ecu,
            // Reverse Engineering
            commands::reverse::list_serial_ports,
            commands::reverse::connect_multimeter,
            commands::reverse::read_multimeter,
            commands::reverse::load_pinout_db,
            commands::reverse::analyze_reverse,
            commands::reverse::check_drivers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
