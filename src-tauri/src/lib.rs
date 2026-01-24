pub mod cli;
pub mod serial;
pub mod config;

use serial::SerialManager;
use config::AppConfig;
use std::sync::Arc;
use parking_lot::Mutex;
use tauri::State;

pub struct AppState {
    pub serial_manager: Arc<Mutex<SerialManager>>,
    pub config: Arc<Mutex<AppConfig>>,
}

// ============ Tauri Commands ============

#[tauri::command]
fn list_ports() -> Result<Vec<serial::PortInfo>, String> {
    serial::list_available_ports()
}

#[tauri::command]
fn connect_serial(
    state: State<AppState>,
    port: String,
    baud_rate: u32,
    data_bits: u8,
    stop_bits: u8,
    parity: String,
) -> Result<(), String> {
    let mut manager = state.serial_manager.lock();
    manager.connect(&port, baud_rate, data_bits, stop_bits, &parity)
}

#[tauri::command]
fn disconnect_serial(state: State<AppState>) -> Result<(), String> {
    let mut manager = state.serial_manager.lock();
    manager.disconnect()
}

#[tauri::command]
fn send_data(state: State<AppState>, data: String, hex_mode: bool) -> Result<(), String> {
    let mut manager = state.serial_manager.lock();
    manager.send(&data, hex_mode)
}

#[tauri::command]
fn read_data(state: State<AppState>) -> Result<Vec<serial::DataEntry>, String> {
    let mut manager = state.serial_manager.lock();
    manager.read_available()
}

#[tauri::command]
fn is_connected(state: State<AppState>) -> bool {
    let manager = state.serial_manager.lock();
    manager.is_connected()
}

#[tauri::command]
fn get_config(state: State<AppState>) -> AppConfig {
    state.config.lock().clone()
}

#[tauri::command]
fn save_config(state: State<AppState>, config: AppConfig) -> Result<(), String> {
    let mut cfg = state.config.lock();
    *cfg = config.clone();
    config::save_config(&config)
}

#[tauri::command]
fn save_log(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = config::load_config().unwrap_or_default();
    
    let state = AppState {
        serial_manager: Arc::new(Mutex::new(SerialManager::new())),
        config: Arc::new(Mutex::new(config)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            list_ports,
            connect_serial,
            disconnect_serial,
            send_data,
            read_data,
            is_connected,
            get_config,
            save_config,
            save_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
