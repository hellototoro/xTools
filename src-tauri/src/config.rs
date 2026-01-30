use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub serial: SerialConfig,
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialConfig {
    pub port: String,
    pub baud_rate: u32,
    #[serde(default)]
    pub custom_baud_rate: u32,
    pub data_bits: u8,
    pub stop_bits: u8,
    pub parity: String,
    pub hex_mode: bool,
    pub append_newline: bool,
    pub newline_type: String, // "crlf", "lf", "cr"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub auto_scroll: bool,
    pub show_timestamp: bool,
    pub show_hex: bool,
    pub font_size: u32,
    pub terminal_mode: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            serial: SerialConfig {
                port: String::new(),
                baud_rate: 115200,
                custom_baud_rate: 0,
                data_bits: 8,
                stop_bits: 1,
                parity: "none".to_string(),
                hex_mode: false,
                append_newline: true,
                newline_type: "crlf".to_string(),
            },
            display: DisplayConfig {
                auto_scroll: true,
                show_timestamp: true,
                show_hex: false,
                font_size: 14,
                terminal_mode: false,
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
    
    serde_json::from_str(&content)
        .map_err(|e| format!("解析配置失败: {}", e))
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = get_config_path();
    
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;
    
    fs::write(&path, content)
        .map_err(|e| format!("保存配置失败: {}", e))
}
