use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};
use std::io::{Read, Write};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialEvent {
    pub timestamp: String,
    pub text: String,
    pub hex: String,
    pub direction: String,
}

pub type DataEntry = SerialEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialConnectionConfig {
    pub port: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub stop_bits: u8,
    pub parity: String,
}

impl Default for SerialConnectionConfig {
    fn default() -> Self {
        Self {
            port: String::new(),
            baud_rate: 115200,
            data_bits: 8,
            stop_bits: 1,
            parity: "none".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendPayload {
    pub data: String,
    pub hex_mode: bool,
}

pub struct SerialManager {
    port: Option<Box<dyn SerialPort>>,
    port_name: String,
    buffer: Vec<u8>,
}

impl SerialManager {
    pub fn new() -> Self {
        Self {
            port: None,
            port_name: String::new(),
            buffer: Vec::with_capacity(4096),
        }
    }

    pub fn connect_with_config(&mut self, config: &SerialConnectionConfig) -> Result<(), String> {
        self.connect(
            &config.port,
            config.baud_rate,
            config.data_bits,
            config.stop_bits,
            &config.parity,
        )
    }

    pub fn connect(
        &mut self,
        port_name: &str,
        baud_rate: u32,
        data_bits: u8,
        stop_bits: u8,
        parity: &str,
    ) -> Result<(), String> {
        if self.port.is_some() {
            self.disconnect()?;
        }

        let data_bits = match data_bits {
            5 => DataBits::Five,
            6 => DataBits::Six,
            7 => DataBits::Seven,
            8 => DataBits::Eight,
            _ => DataBits::Eight,
        };

        let stop_bits = match stop_bits {
            1 => StopBits::One,
            2 => StopBits::Two,
            _ => StopBits::One,
        };

        let parity = match parity.to_lowercase().as_str() {
            "none" => Parity::None,
            "odd" => Parity::Odd,
            "even" => Parity::Even,
            _ => Parity::None,
        };

        let port = serialport::new(port_name, baud_rate)
            .data_bits(data_bits)
            .stop_bits(stop_bits)
            .parity(parity)
            .flow_control(FlowControl::None)
            .timeout(Duration::from_millis(10))
            .open()
            .map_err(|e| format!("无法打开串口 {}: {}", port_name, e))?;

        self.port = Some(port);
        self.port_name = port_name.to_string();
        self.buffer.clear();

        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), String> {
        self.port = None;
        self.port_name.clear();
        self.buffer.clear();
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.port.is_some()
    }

    pub fn port_name(&self) -> &str {
        &self.port_name
    }

    pub fn send(&mut self, data: &str, hex_mode: bool) -> Result<(), String> {
        let port = self.port.as_mut().ok_or("串口未连接")?;

        let bytes = if hex_mode {
            parse_hex_string(data)?
        } else {
            data.as_bytes().to_vec()
        };

        port.write_all(&bytes)
            .map_err(|e| format!("发送失败: {}", e))?;

        Ok(())
    }

    pub fn send_payload(&mut self, payload: &SendPayload) -> Result<SerialEvent, String> {
        let bytes = if payload.hex_mode {
            parse_hex_string(&payload.data)?
        } else {
            payload.data.as_bytes().to_vec()
        };

        let port = self.port.as_mut().ok_or("串口未连接")?;
        port.write_all(&bytes)
            .map_err(|e| format!("发送失败: {}", e))?;

        Ok(SerialEvent {
            timestamp: current_timestamp(),
            text: payload.data.clone(),
            hex: bytes_to_hex_string(&bytes),
            direction: "tx".to_string(),
        })
    }

    pub fn read_available(&mut self) -> Result<Vec<SerialEvent>, String> {
        let port = match self.port.as_mut() {
            Some(p) => p,
            None => return Ok(vec![]),
        };

        let mut temp_buf = [0u8; 1024];
        let mut entries = Vec::new();

        loop {
            match port.read(&mut temp_buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data_slice = &temp_buf[..n];
                    
                    entries.push(SerialEvent {
                        timestamp: current_timestamp(),
                        text: String::from_utf8_lossy(data_slice).to_string(),
                        hex: bytes_to_hex_string(data_slice),
                        direction: "rx".to_string(),
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => break,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(format!("读取错误: {}", e)),
            }
        }

        Ok(entries)
    }
}

fn current_timestamp() -> String {
    let now: DateTime<Local> = Local::now();
    now.format("%H:%M:%S%.3f").to_string()
}

pub fn list_available_ports() -> Result<Vec<PortInfo>, String> {
    let ports = serialport::available_ports()
        .map_err(|e| format!("无法获取串口列表: {}", e))?;

    Ok(ports
        .into_iter()
        .map(|p| PortInfo {
            name: p.port_name.clone(),
            description: match p.port_type {
                serialport::SerialPortType::UsbPort(info) => {
                    format!(
                        "{} - {}",
                        info.manufacturer.unwrap_or_default(),
                        info.product.unwrap_or_default()
                    )
                }
                serialport::SerialPortType::PciPort => "PCI 串口".to_string(),
                serialport::SerialPortType::BluetoothPort => "蓝牙串口".to_string(),
                serialport::SerialPortType::Unknown => "未知类型".to_string(),
            },
        })
        .collect())
}

pub fn parse_hex_string(s: &str) -> Result<Vec<u8>, String> {
    let s = s.replace(" ", "").replace("\n", "").replace("\r", "");
    
    if s.len() % 2 != 0 {
        return Err("十六进制字符串长度必须为偶数".to_string());
    }

    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|_| format!("无效的十六进制字符: {}", &s[i..i + 2]))
        })
        .collect()
}

pub fn bytes_to_hex_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn apply_newline(data: &str, append_newline: bool, newline_type: &str) -> String {
    if !append_newline {
        return data.to_string();
    }

    let newline = match newline_type {
        "lf" => "\n",
        "cr" => "\r",
        _ => "\r\n",
    };

    format!("{}{}", data, newline)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_with_spaces_and_newlines() {
        assert_eq!(parse_hex_string("48 65\n6C\r6C 6F").unwrap(), b"Hello");
    }

    #[test]
    fn rejects_odd_hex_length() {
        assert!(parse_hex_string("ABC").is_err());
    }

    #[test]
    fn formats_bytes_as_uppercase_hex() {
        assert_eq!(bytes_to_hex_string(&[0, 10, 255]), "00 0A FF");
    }

    #[test]
    fn applies_requested_newline() {
        assert_eq!(apply_newline("AT", true, "crlf"), "AT\r\n");
        assert_eq!(apply_newline("AT", true, "lf"), "AT\n");
        assert_eq!(apply_newline("AT", true, "cr"), "AT\r");
        assert_eq!(apply_newline("AT", false, "crlf"), "AT");
    }
}
