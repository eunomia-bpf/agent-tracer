use super::{Analyzer, AnalyzerError};
use crate::framework::runners::EventStream;
use crate::framework::core::Event;
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::io::Write;

/// SSE Event Processor that merges Server-Sent Events content fragments
pub struct SSEProcessor {
    name: String,
    /// Store accumulated SSE content by connection + message ID
    sse_buffers: Arc<Mutex<HashMap<String, SSEAccumulator>>>,
    /// Timeout for incomplete SSE streams (in milliseconds)
    timeout_ms: u64,
}

/// Accumulator for SSE events belonging to the same message
struct SSEAccumulator {
    message_id: Option<String>,
    accumulated_text: String,
    accumulated_json: String,
    events: Vec<SSEEvent>,
    is_complete: bool,
    last_update: u64,
    /// Track if we've seen a message_start event
    has_message_start: bool,
}

/// Parsed SSE event
#[derive(Clone, Debug)]
pub struct SSEEvent {
    pub event_type: String,
    pub data: Value,
}

impl SSEProcessor {
    /// Create a new SSEProcessor with default timeout (30 seconds)
    pub fn new() -> Self {
        Self::new_with_timeout(30_000)
    }

    /// Create a new SSEProcessor with custom timeout
    pub fn new_with_timeout(timeout_ms: u64) -> Self {
        SSEProcessor {
            name: "SSEProcessor".to_string(),
            sse_buffers: Arc::new(Mutex::new(HashMap::new())),
            timeout_ms,
        }
    }

    /// Check if SSL data contains SSE events - enhanced detection
    pub fn is_sse_data(data: &str) -> bool {
        // Look for SSE patterns in the data
        let has_sse_patterns = data.contains("event:") && data.contains("data:");
        
        // Also check for Content-Type: text/event-stream
        let has_sse_content_type = data.contains("text/event-stream");
        
        // Check for chunked encoding with SSE events
        let has_chunked_sse = data.contains("Transfer-Encoding: chunked") && 
                              (data.contains("event:") || data.contains("data:"));
        
        has_sse_patterns || has_sse_content_type || has_chunked_sse
    }

    /// Parse SSE events from raw SSL data
    pub fn parse_sse_events(data: &str) -> Vec<SSEEvent> {
        let mut events = Vec::new();
        
        // Clean up chunked encoding first
        let clean_data = Self::clean_chunked_content(data);
        
        // Split by double newlines to separate events
        let parts: Vec<&str> = clean_data.split("\n\n").collect();
        
        for part in parts {
            if part.trim().is_empty() {
                continue;
            }
            
            let mut event_type = String::new();
            let mut data_content = String::new();
            
            // Parse event: and data: lines
            for line in part.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("event:") {
                    event_type = trimmed[6..].trim().to_string();
                } else if trimmed.starts_with("data:") {
                    data_content = trimmed[5..].trim().to_string();
                }
            }
            
            // Parse JSON data if present
            if !event_type.is_empty() && !data_content.is_empty() {
                match serde_json::from_str::<Value>(&data_content) {
                    Ok(json_data) => {
                        events.push(SSEEvent {
                            event_type,
                            data: json_data,
                        });
                    }
                    Err(e) => {
                        eprintln!(" SSEProcessor: Failed to parse SSE JSON data: {} - Error: {}", data_content, e);
                        std::io::stdout().flush().unwrap();
                    }
                }
            }
        }
        
        events
    }

    /// Clean HTTP chunked encoding artifacts from content
    pub fn clean_chunked_content(content: &str) -> String {
        let mut cleaned = String::new();
        let mut skip_next = false;
        
        for line in content.lines() {
            if skip_next {
                skip_next = false;
                continue;
            }
            
            let trimmed = line.trim();
            
            // Skip HTTP chunk size lines (hex numbers followed by \r\n)
            if trimmed.chars().all(|c| c.is_ascii_hexdigit()) && trimmed.len() < 10 {
                skip_next = false; // Don't skip next line unless it's empty
                continue;
            }
            
            // Skip empty lines that are chunk separators
            if trimmed.is_empty() {
                continue;
            }
            
            // Keep the actual content
            cleaned.push_str(line);
            cleaned.push('\n');
        }
        
        cleaned
    }

    /// Generate a connection ID from event data and SSE events
    fn generate_connection_id(event: &Event, sse_events: &[SSEEvent]) -> String {
        let pid = event.data.get("pid").and_then(|v| v.as_u64()).unwrap_or(0);
        let tid = event.data.get("tid").and_then(|v| v.as_u64()).unwrap_or(0);
        
        // First, try to extract message ID from the SSE events
        if let Some(message_id) = Self::extract_message_id(sse_events) {
            return format!("{}:{}:{}", pid, tid, message_id);
        }
        
        // If no message ID, use a persistent connection identifier
        // Use a larger time window (60 seconds) to keep long SSE streams together
        let timestamp = event.timestamp;
        let window = timestamp / 60_000_000_000; // Convert to 60-second windows
        format!("{}:{}:{}", pid, tid, window)
    }

    /// Extract message ID from SSE events
    fn extract_message_id(events: &[SSEEvent]) -> Option<String> {
        for event in events {
            if event.event_type == "message_start" {
                if let Some(message) = event.data.get("message") {
                    if let Some(id) = message.get("id") {
                        if let Some(id_str) = id.as_str() {
                            return Some(id_str.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    /// Check if SSE stream is complete
    fn is_sse_complete(accumulator: &SSEAccumulator) -> bool {
        // Check for completion events
        for event in &accumulator.events {
            match event.event_type.as_str() {
                "message_stop" | "content_block_stop" | "error" => return true,
                "message_delta" => {
                    // Check if this indicates completion
                    if let Some(delta) = event.data.get("delta") {
                        if delta.get("stop_reason").is_some() {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        
        // Check buffer size timeout
        accumulator.accumulated_text.len() > 10240 || // 10KB limit
        accumulator.accumulated_json.len() > 10240
    }

    /// Accumulate content from content_block_delta events
    fn accumulate_content(accumulator: &mut SSEAccumulator, events: &[SSEEvent]) {
        for event in events {
            accumulator.events.push(event.clone());
            
            match event.event_type.as_str() {
                "message_start" => {
                    accumulator.has_message_start = true;
                    // Extract message ID
                    if accumulator.message_id.is_none() {
                        accumulator.message_id = Self::extract_message_id(&[event.clone()]);
                    }
                }
                "content_block_delta" => {
                    if let Some(delta) = event.data.get("delta") {
                        // Handle text delta
                        if let Some(text_delta) = delta.get("text") {
                            if let Some(text) = text_delta.as_str() {
                                accumulator.accumulated_text.push_str(text);
                            }
                        }
                        
                        // Handle JSON delta (partial_json)
                        if let Some(partial_json) = delta.get("partial_json") {
                            if let Some(json_text) = partial_json.as_str() {
                                accumulator.accumulated_json.push_str(json_text);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Warn about unexpected conditions
    fn warn_if_unexpected(accumulator: &SSEAccumulator, connection_id: &str) {
        // Warn if we're completing without message_start
        if !accumulator.has_message_start && accumulator.events.len() > 0 {
            eprintln!(" SSEProcessor: Completing SSE stream {} without message_start event - may be incomplete!", connection_id);
            std::io::stdout().flush().unwrap();
        }
        
        // Warn if we have very little content
        if accumulator.accumulated_text.len() == 0 && accumulator.accumulated_json.len() == 0 && accumulator.events.len() > 3 {
            eprintln!(" SSEProcessor: SSE stream {} has {} events but no accumulated content - possible parsing issue!", connection_id, accumulator.events.len());
            std::io::stdout().flush().unwrap();
        }
        
        // Warn if JSON looks incomplete
        if !accumulator.accumulated_json.is_empty() && !accumulator.accumulated_json.starts_with('{') {
            eprintln!(" SSEProcessor: SSE stream {} has JSON content that doesn't start with '{{' - may be incomplete: {}", connection_id, &accumulator.accumulated_json[..std::cmp::min(50, accumulator.accumulated_json.len())]);
            std::io::stdout().flush().unwrap();
        }
        
        // Warn if buffer size limit was hit
        if accumulator.accumulated_text.len() > 10240 || accumulator.accumulated_json.len() > 10240 {
            eprintln!(" SSEProcessor: SSE stream {} hit buffer size limit - may be incomplete!", connection_id);
            std::io::stdout().flush().unwrap();
        }
    }

    /// Create merged event from accumulated SSE content
    fn create_merged_event(
        connection_id: String,
        accumulator: &SSEAccumulator,
        original_event: &Event,
    ) -> Event {
        let merged_content = if !accumulator.accumulated_json.is_empty() {
            // Try to parse accumulated JSON
            match serde_json::from_str::<Value>(&accumulator.accumulated_json) {
                Ok(parsed_json) => serde_json::to_string_pretty(&parsed_json).unwrap_or(accumulator.accumulated_json.clone()),
                Err(_) => accumulator.accumulated_json.clone(),
            }
        } else {
            accumulator.accumulated_text.clone()
        };

        Event::new(
            "chunk_merger".to_string(),
            json!({
                "connection_id": connection_id,
                "message_id": accumulator.message_id,
                "original_source": "ssl",
                "function": original_event.data.get("function").unwrap_or(&json!("unknown")).as_str().unwrap_or("unknown"),
                "comm": original_event.data.get("comm").unwrap_or(&json!("unknown")).as_str().unwrap_or("unknown"),
                "pid": original_event.data.get("pid").unwrap_or(&json!(0)),
                "tid": original_event.data.get("tid").unwrap_or(&json!(0)),
                "timestamp_ns": original_event.data.get("timestamp_ns").unwrap_or(&json!(0)),
                "merged_content": merged_content,
                "content_type": if !accumulator.accumulated_json.is_empty() { "json" } else { "text" },
                "total_size": merged_content.len(),
                "event_count": accumulator.events.len(),
                "has_message_start": accumulator.has_message_start,
                "sse_events": accumulator.events.iter().map(|e| json!({
                    "type": e.event_type,
                    "data": e.data
                })).collect::<Vec<_>>()
            })
        )
    }
}

#[async_trait]
impl Analyzer for SSEProcessor {
    async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
        let sse_buffers = Arc::clone(&self.sse_buffers);

        eprintln!("SSEProcessor: Starting SSE event processing");
        std::io::stdout().flush().unwrap();
        
        let processed_stream = stream.filter_map(move |event| {
            let buffers = Arc::clone(&sse_buffers);
            
            async move {
                // Only process SSL events with data
                if event.source != "ssl" {
                    return Some(event);
                }

                let data_str = match event.data.get("data").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return Some(event),
                };

                // Check if this is SSE data
                if !Self::is_sse_data(data_str) {
                    return Some(event);
                }

                // Parse SSE events from this data
                let sse_events = Self::parse_sse_events(data_str);
                if sse_events.is_empty() {
                    return Some(event); // Pass through if no SSE events found
                }

                let connection_id = Self::generate_connection_id(&event, &sse_events);
                
                // Store/accumulate SSE events for this connection
                let mut buffers_lock = buffers.lock().unwrap();
                
                // Check if we already have an accumulator with the same message ID
                let mut final_connection_id = connection_id.clone();
                if let Some(message_id) = Self::extract_message_id(&sse_events) {
                    // Look for existing accumulator with this message ID
                    for (existing_id, accumulator) in buffers_lock.iter() {
                        if let Some(existing_msg_id) = &accumulator.message_id {
                            if existing_msg_id == &message_id {
                                final_connection_id = existing_id.clone();
                                break;
                            }
                        }
                    }
                }
                
                let accumulator = buffers_lock.entry(final_connection_id.clone()).or_insert_with(|| SSEAccumulator {
                    message_id: None,
                    accumulated_text: String::new(),
                    accumulated_json: String::new(),
                    events: Vec::new(),
                    is_complete: false,
                    last_update: event.timestamp,
                    has_message_start: false,
                });
                
                // Update last update time
                accumulator.last_update = event.timestamp;
                
                // Accumulate content from SSE events
                Self::accumulate_content(accumulator, &sse_events);
                
                // Check if stream is complete
                if Self::is_sse_complete(accumulator) {
                    // Warn about unexpected conditions before completing
                    Self::warn_if_unexpected(accumulator, &final_connection_id);
                    eprintln!("SSEProcessor: Completed SSE stream for connection {} - {} text chars, {} json chars, {} events", 
                            final_connection_id, 
                            accumulator.accumulated_text.len(),
                            accumulator.accumulated_json.len(),
                            accumulator.events.len());
                    std::io::stdout().flush().unwrap();
                    
                    // Create merged event
                    let merged_event = Self::create_merged_event(
                        final_connection_id.clone(),
                        accumulator,
                        &event,
                    );
                    
                    // Clear this accumulator
                    buffers_lock.remove(&final_connection_id);
                    drop(buffers_lock);
                    
                    Some(merged_event)
                } else {
                    // Stream not complete yet, don't emit event
                    None
                }
            }
        });

        Ok(Box::pin(processed_stream))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

 