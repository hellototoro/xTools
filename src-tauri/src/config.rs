use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::serial::SerialConnectionConfig;
use crate::session::WorkMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub serial: SerialConnectionConfig,
    pub mode: WorkMode,
    pub monitor: MonitorConfig,
    pub terminal: TerminalConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub hex_mode: bool,
    pub append_newline: bool,
    pub newline_type: String, // "crlf", "lf", "cr"
    pub auto_scroll: bool,
    pub show_timestamp: bool,
    pub show_hex: bool,
    pub font_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    pub font_size: u32,
    pub copy_on_ctrl_c_selection: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            serial: SerialConnectionConfig::default(),
            mode: WorkMode::Monitor,
            monitor: MonitorConfig {
                hex_mode: false,
                append_newline: true,
                newline_type: "crlf".to_string(),
                auto_scroll: true,
                show_timestamp: true,
                show_hex: false,
                font_size: 14,
            },
            terminal: TerminalConfig {
                font_size: 14,
                copy_on_ctrl_c_selection: true,
            },
        }
    }
}

fn get_config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("xtools");
    
    if !config_dir.exists() {
        let _ = fs::create_dir_all(&config_dir);
    }
    
    config_dir.join("config.json")
}

pub fn load_config() -> Result<AppConfig, String> {
    let path = get_config_path();
    
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("读取配置失败: {}", e))?;
    
    serde_json::from_str(&content).or_else(|_| Ok(AppConfig::default()))
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = get_config_path();
    
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;
    
    fs::write(&path, content)
        .map_err(|e| format!("保存配置失败: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_monitor_mode() {
        let config = AppConfig::default();
        assert_eq!(config.mode, WorkMode::Monitor);
        assert_eq!(config.serial.baud_rate, 115200);
        assert!(config.monitor.auto_scroll);
        assert!(config.terminal.copy_on_ctrl_c_selection);
    }
}
