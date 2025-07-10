use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityEvent {
    pub id: String,
    pub timestamp: u64,
    pub source: EventSource,
    pub event_type: EventType,
    pub data: EventData,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventSource {
    Process,
    SSL,
    Network,
    FileSystem,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    ProcessStart,
    ProcessExit,
    FileAccess,
    NetworkConnection,
    SSLHandshake,
    SSLData,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventData {
    Process {
        pid: u32,
        ppid: u32,
        comm: String,
        filename: String,
    },
    SSL {
        function: String,
        pid: u32,
        comm: String,
        data: String,
        data_len: usize,
        is_handshake: bool,
    },
    Custom(HashMap<String, serde_json::Value>),
}

impl ObservabilityEvent {
    pub fn new(source: EventSource, event_type: EventType, data: EventData) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            source,
            event_type,
            data,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}