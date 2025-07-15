use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use crate::framework::core::Event;

/// SSE Processor Event - represents a complete SSE interaction with timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSEProcessorEvent {
    pub connection_id: String,
    pub message_id: Option<String>,
    pub start_time: u64,
    pub end_time: u64,
    pub duration_ns: u64,
    pub original_source: String,
    pub function: String,
    pub comm: String,
    pub pid: u64,
    pub tid: u64,
    pub json_content: String,
    pub text_content: String,
    pub total_size: usize,
    pub event_count: usize,
    pub has_message_start: bool,
    pub sse_events: Vec<Value>,
}

impl SSEProcessorEvent {
    pub fn new(
        connection_id: String,
        message_id: Option<String>,
        start_time: u64,
        end_time: u64,
        original_source: String,
        function: String,
        comm: String,
        pid: u64,
        tid: u64,
        json_content: String,
        text_content: String,
        total_size: usize,
        event_count: usize,
        has_message_start: bool,
        sse_events: Vec<Value>,
    ) -> Self {
        let duration_ns = end_time.saturating_sub(start_time);
        
        SSEProcessorEvent {
            connection_id,
            message_id,
            start_time,
            end_time,
            duration_ns,
            original_source,
            function,
            comm,
            pid,
            tid,
            json_content,
            text_content,
            total_size,
            event_count,
            has_message_start,
            sse_events,
        }
    }

    pub fn to_event(&self) -> Event {
        let data = serde_json::json!({
            "connection_id": self.connection_id,
            "message_id": self.message_id,
            "start_time": self.start_time,
            "end_time": self.end_time,
            "duration_ns": self.duration_ns,
            "duration_ms": self.duration_ns as f64 / 1_000_000.0,
            "duration_seconds": self.duration_ns as f64 / 1_000_000_000.0,
            "original_source": self.original_source,
            "function": self.function,
            "comm": self.comm,
            "pid": self.pid,
            "tid": self.tid,
            "json_content": self.json_content,
            "text_content": self.text_content,
            "total_size": self.total_size,
            "event_count": self.event_count,
            "has_message_start": self.has_message_start,
            "sse_events": self.sse_events
        });

        Event::new("sse_processor".to_string(), data)
    }
}

/// HTTP Event - represents a parsed HTTP request or response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPEvent {
    pub tid: u64,
    pub message_type: String,
    pub first_line: String,
    pub method: Option<String>,
    pub path: Option<String>,
    pub protocol: Option<String>,
    pub status_code: Option<u16>,
    pub status_text: Option<String>,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub total_size: usize,
    pub has_body: bool,
    pub is_chunked: bool,
    pub content_length: Option<usize>,
    pub original_source: String,
    pub comm: String,
    pub pid: u64,
    pub timestamp_ns: u64,
}

impl HTTPEvent {
    pub fn new(
        tid: u64,
        message_type: String,
        first_line: String,
        method: Option<String>,
        path: Option<String>,
        protocol: Option<String>,
        status_code: Option<u16>,
        status_text: Option<String>,
        headers: HashMap<String, String>,
        body: Option<String>,
        total_size: usize,
        has_body: bool,
        is_chunked: bool,
        content_length: Option<usize>,
        original_source: String,
        comm: String,
        pid: u64,
        timestamp_ns: u64,
    ) -> Self {
        HTTPEvent {
            tid,
            message_type,
            first_line,
            method,
            path,
            protocol,
            status_code,
            status_text,
            headers,
            body,
            total_size,
            has_body,
            is_chunked,
            content_length,
            original_source,
            comm,
            pid,
            timestamp_ns,
        }
    }

    pub fn to_event(&self) -> Event {
        let data = serde_json::json!({
            "tid": self.tid,
            "message_type": self.message_type,
            "first_line": self.first_line,
            "method": self.method,
            "path": self.path,
            "protocol": self.protocol,
            "status_code": self.status_code,
            "status_text": self.status_text,
            "headers": self.headers,
            "body": self.body,
            "total_size": self.total_size,
            "has_body": self.has_body,
            "is_chunked": self.is_chunked,
            "content_length": self.content_length,
            "original_source": self.original_source,
            "comm": self.comm,
            "pid": self.pid,
            "timestamp_ns": self.timestamp_ns,
        });

        Event::new("http_parser".to_string(), data)
    }

    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}