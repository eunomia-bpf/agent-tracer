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
            tid,
            json_content,
            text_content,
            total_size,
            event_count,
            has_message_start,
            sse_events,
        }
    }

    pub fn to_event(&self, original_event: &Event) -> Event {
        // Serialize struct to JSON Value to ensure exact match with struct fields
        let data = serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}));

        // Use merged end_time if events were merged, otherwise use original timestamp
        let timestamp = if self.event_count > 1 {
            self.end_time
        } else {
            original_event.timestamp
        };

        Event::new_with_timestamp(
            timestamp,
            "sse_processor".to_string(), 
            original_event.pid, 
            original_event.comm.clone(), 
            data
        )
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
    pub raw_data: Option<String>,
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
            raw_data: None,
        }
    }

    pub fn with_raw_data(mut self, raw_data: String) -> Self {
        self.raw_data = Some(raw_data);
        self
    }

    pub fn to_event(&self, original_event: &Event) -> Event {
        // Serialize struct to JSON Value to ensure exact match with struct fields
        let data = serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}));

        Event::new_with_timestamp(
            original_event.timestamp,  // Use original event timestamp directly
            "http_parser".to_string(), 
            original_event.pid, 
            original_event.comm.clone(), 
            data
        )
    }

}