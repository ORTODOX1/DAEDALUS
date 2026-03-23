use std::sync::Mutex;

pub struct AppState {
    pub connected: Mutex<bool>,
    pub ecu_type: Mutex<Option<String>>,
    pub multimeter_port: Mutex<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            connected: Mutex::new(false),
            ecu_type: Mutex::new(None),
            multimeter_port: Mutex::new(None),
        }
    }
}
