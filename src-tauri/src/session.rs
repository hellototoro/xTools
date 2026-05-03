use crate::serial::{SerialConnectionConfig, SerialEvent, SerialManager, SendPayload};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkMode {
    Terminal,
    Monitor,
}

impl Default for WorkMode {
    fn default() -> Self {
        Self::Monitor
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub connected: bool,
    pub mode: WorkMode,
    pub port: String,
}

pub struct SerialSession {
    manager: SerialManager,
    mode: WorkMode,
}

impl SerialSession {
    pub fn new(mode: WorkMode) -> Self {
        Self {
            manager: SerialManager::new(),
            mode,
        }
    }

    pub fn connect(&mut self, config: &SerialConnectionConfig) -> Result<(), String> {
        self.manager.connect_with_config(config)
    }

    pub fn disconnect(&mut self) -> Result<(), String> {
        self.manager.disconnect()
    }

    pub fn send(&mut self, payload: &SendPayload) -> Result<SerialEvent, String> {
        self.manager.send_payload(payload)
    }

    pub fn read_events(&mut self) -> Result<Vec<SerialEvent>, String> {
        self.manager.read_available()
    }

    pub fn set_mode(&mut self, mode: WorkMode) {
        self.mode = mode;
    }

    pub fn mode(&self) -> WorkMode {
        self.mode
    }

    pub fn state(&self) -> SessionState {
        SessionState {
            connected: self.manager.is_connected(),
            mode: self.mode,
            port: self.manager.port_name().to_string(),
        }
    }
}
