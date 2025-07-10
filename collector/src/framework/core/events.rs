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
    pub tags: Vec<String>,
    pub severity: EventSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventSource {
    Process,
    SSL,
    Network,
    FileSystem,
    Agent,
    Tool,
    Conversation,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    // Process events
    ProcessStart,
    ProcessExit,
    ProcessError,
    
    // File system events
    FileAccess,
    FileCreate,
    FileDelete,
    FileModify,
    
    // Network events
    NetworkConnection,
    NetworkDisconnection,
    NetworkError,
    
    // SSL/TLS events
    SSLHandshake,
    SSLData,
    SSLError,
    
    // Agent-specific events
    AgentStart,
    AgentStop,
    AgentError,
    ToolCall,
    ToolResponse,
    ToolError,
    ConversationStart,
    ConversationEnd,
    ConversationTurn,
    
    // Generic events
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventData {
    Process {
        pid: u32,
        ppid: u32,
        comm: String,
        filename: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    },
    SSL {
        function: String,
        pid: u32,
        comm: String,
        data: String,
        data_len: usize,
        is_handshake: bool,
        cipher_suite: Option<String>,
        protocol_version: Option<String>,
    },
    Network {
        src_ip: String,
        dst_ip: String,
        src_port: u16,
        dst_port: u16,
        protocol: String,
        bytes_sent: u64,
        bytes_received: u64,
    },
    Agent {
        agent_id: String,
        session_id: String,
        model: Option<String>,
        tokens_used: Option<u32>,
        latency_ms: Option<u64>,
    },
    Tool {
        tool_name: String,
        parameters: HashMap<String, serde_json::Value>,
        result: Option<serde_json::Value>,
        execution_time_ms: Option<u64>,
        success: bool,
    },
    Conversation {
        conversation_id: String,
        turn_id: String,
        role: String, // user, assistant, system
        content: String,
        token_count: Option<u32>,
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
            tags: Vec::new(),
            severity: EventSeverity::Info,
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_severity(mut self, severity: EventSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags.extend(tags);
        self
    }
}

impl Default for EventSeverity {
    fn default() -> Self {
        EventSeverity::Info
    }
} 