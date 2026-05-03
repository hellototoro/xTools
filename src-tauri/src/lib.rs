pub mod cli;
pub mod serial;
pub mod config;
pub mod session;

use config::AppConfig;
use serial::{SendPayload, SerialConnectionConfig, SerialEvent};
use session::{SerialSession, SessionState, WorkMode};
use std::sync::Arc;
use parking_lot::Mutex;
use tauri::State;

pub struct AppState {
    pub session: Arc<Mutex<SerialSession>>,
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
    config: SerialConnectionConfig,
) -> Result<(), String> {
    let mut session = state.session.lock();
    session.connect(&config)
}

#[tauri::command]
fn disconnect_serial(state: State<AppState>) -> Result<(), String> {
    let mut session = state.session.lock();
    session.disconnect()
}

#[tauri::command]
fn send_data(state: State<AppState>, payload: SendPayload) -> Result<SerialEvent, String> {
    let mut session = state.session.lock();
    session.send(&payload)
}

#[tauri::command]
fn read_serial_events(state: State<AppState>) -> Result<Vec<SerialEvent>, String> {
    let mut session = state.session.lock();
    session.read_events()
}

#[tauri::command]
fn is_connected(state: State<AppState>) -> bool {
    state.session.lock().state().connected
}

#[tauri::command]
fn get_session_state(state: State<AppState>) -> SessionState {
    state.session.lock().state()
}

#[tauri::command]
fn set_work_mode(state: State<AppState>, mode: WorkMode) {
    state.session.lock().set_mode(mode);
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
    let mode = config.mode;
    
    let state = AppState {
        session: Arc::new(Mutex::new(SerialSession::new(mode))),
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
            read_serial_events,
            is_connected,
            get_session_state,
            set_work_mode,
            get_config,
            save_config,
            save_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
