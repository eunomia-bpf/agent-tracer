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
    /// Enable debug output (matches Python quiet flag)
    debug: bool,
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

/// Parsed SSE event - matches ssl_log_analyzer.py structure
#[derive(Clone, Debug)]
pub struct SSEEvent {
    pub event: Option<String>,
    pub data: Option<String>,
    pub id: Option<String>,
    pub parsed_data: Option<Value>,
    pub raw_data: Option<String>,
}

impl SSEProcessor {
    /// Create a new SSEProcessor with default timeout (30 seconds)
    pub fn new() -> Self {
        Self::new_with_timeout(30_000)
    }

    /// Create a new SSEProcessor with debug output enabled
    pub fn new_with_debug() -> Self {
        SSEProcessor {
            name: "SSEProcessor".to_string(),
            sse_buffers: Arc::new(Mutex::new(HashMap::new())),
            timeout_ms: 30_000,
            debug: true,
        }
    }

    /// Create a new SSEProcessor with custom timeout
    pub fn new_with_timeout(timeout_ms: u64) -> Self {
        SSEProcessor {
            name: "SSEProcessor".to_string(),
            sse_buffers: Arc::new(Mutex::new(HashMap::new())),
            timeout_ms,
            debug: true,
        }
    }

    /// Debug print function - only prints if debug is enabled (matches Python debug_print)
    fn debug_print(&self, message: &str) {
        if self.debug {
            eprintln!("{}", message);
            std::io::stdout().flush().unwrap();
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
        
        // Check for standalone data: field (SSE can have just data: without event:)
        let has_sse_data_only = data.contains("data:") && (data.contains("\r\n\r\n") || data.contains("\n\n"));
        
        has_sse_patterns || has_sse_content_type || has_chunked_sse || has_sse_data_only
    }

    /// Parse SSE events from a single chunk - matches ssl_log_analyzer.py parse_sse_events_from_chunk
    pub fn parse_sse_events_from_chunk(chunk_content: &str) -> Vec<SSEEvent> {
        let mut events = Vec::new();
        
        // Split by double newlines to separate events - matches Python: re.split(r'\n\s*\n', chunk_content)
        let event_blocks: Vec<&str> = chunk_content.split("\n\n").collect();
        
        for block in event_blocks {
            if block.trim().is_empty() {
                continue;
            }
            
            let mut event = SSEEvent {
                event: None,
                data: None,
                id: None,
                parsed_data: None,
                raw_data: None,
            };
            let mut data_lines = Vec::new();
            
            for line in block.split('\n') {
                let line = line.trim();
                if line.starts_with("event:") {
                    event.event = Some(line[6..].trim().to_string());
                } else if line.starts_with("data:") {
                    data_lines.push(line[5..].trim());
                } else if line.starts_with("id:") {
                    event.id = Some(line[3..].trim().to_string());
                }
            }
            
            if !data_lines.is_empty() {
                let combined_data = data_lines.join("\n");
                event.data = Some(combined_data.clone());
                
                // Try to parse as JSON
                match serde_json::from_str::<Value>(&combined_data) {
                    Ok(parsed_json) => {
                        event.parsed_data = Some(parsed_json);
                    }
                    Err(_) => {
                        event.raw_data = Some(combined_data);
                    }
                }
            }
            
            if event.event.is_some() || event.data.is_some() {
                events.push(event);
            }
        }
        
        events
    }

    /// Parse SSE events from raw SSL data
    pub fn parse_sse_events(data: &str) -> Vec<SSEEvent> {
        // Clean up chunked encoding first
        let clean_data = Self::clean_chunked_content(data);
        
        // Use the chunk parser
        Self::parse_sse_events_from_chunk(&clean_data)
    }

    /// Clean HTTP chunked encoding artifacts from content - matches ssl_log_analyzer.py logic
    pub fn clean_chunked_content(content: &str) -> String {
        let mut content_parts = Vec::new();
        let lines: Vec<&str> = content.split("\r\n").collect();
        
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();
            
            // Check if this is a chunk size (hex number) - matches Python regex r'^[0-9a-fA-F]+$'
            if !line.is_empty() && line.chars().all(|c| c.is_ascii_hexdigit()) {
                let chunk_size = u32::from_str_radix(line, 16).unwrap_or(0);
                if chunk_size == 0 {
                    break;
                }
                
                // Get the chunk content (next line)
                i += 1;
                if i < lines.len() {
                    content_parts.push(lines[i]);
                }
            }
            i += 1;
        }
        
        // Join all content and return - matches Python: '\n'.join(content_parts)
        content_parts.join("\n")
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

    /// Extract message ID from SSE events - matches ssl_log_analyzer.py logic
    fn extract_message_id(events: &[SSEEvent]) -> Option<String> {
        for event in events {
            if let Some(event_type) = &event.event {
                if event_type == "message_start" {
                    if let Some(parsed_data) = &event.parsed_data {
                        if let Some(message) = parsed_data.get("message") {
                            if let Some(id) = message.get("id") {
                                if let Some(id_str) = id.as_str() {
                                    return Some(id_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Check if SSE stream is complete - follows Claude API streaming docs
    fn is_sse_complete(accumulator: &SSEAccumulator) -> bool {
        // According to Claude docs, the proper completion sequence is:
        // 1. message_start
        // 2. content_block_start, content_block_delta(s), content_block_stop
        // 3. message_delta (with stop_reason)
        // 4. message_stop (final event)
        
        let mut has_message_stop = false;
        let mut has_content_block_stop = false;
        let mut has_stop_reason = false;
        
        for event in &accumulator.events {
            if let Some(event_type) = &event.event {
                match event_type.as_str() {
                    "message_stop" => has_message_stop = true,
                    "content_block_stop" => has_content_block_stop = true,
                    "error" => return true, // Immediate completion on error
                    "message_delta" => {
                        // Check if this indicates completion with stop_reason
                        if let Some(parsed_data) = &event.parsed_data {
                            if let Some(delta) = parsed_data.get("delta") {
                                if delta.get("stop_reason").is_some() {
                                    has_stop_reason = true;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        
        // Stream is complete when we have message_stop (the final event)
        // OR when we have both content_block_stop and message_delta with stop_reason
        let is_complete = has_message_stop || (has_content_block_stop && has_stop_reason);
        
        // Also check buffer size timeout as fallback
        let size_timeout = accumulator.accumulated_text.len() > 10240 || 
                          accumulator.accumulated_json.len() > 10240;
        
        is_complete || size_timeout
    }

    /// Check if SSE stream contains meaningful content worth creating an event for
    fn has_meaningful_content(accumulator: &SSEAccumulator) -> bool {
        // Content is meaningful if:
        // 1. We have accumulated text content
        // 2. We have accumulated JSON content 
        // 3. We have content_block_delta events (indicates content stream)
        // 4. We have a substantial number of events (suggests real content stream)
        
        if !accumulator.accumulated_text.is_empty() || !accumulator.accumulated_json.is_empty() {
            return true;
        }
        
        // Check if we have content_block_delta events (indicates content stream)
        let mut has_content_deltas = false;
        let mut has_message_start = false;
        let mut metadata_only_count = 0;
        
        for event in &accumulator.events {
            if let Some(event_type) = &event.event {
                match event_type.as_str() {
                    "content_block_delta" => has_content_deltas = true,
                    "message_start" => has_message_start = true,
                    // These are metadata-only events
                    "message_stop" | "message_delta" | "ping" | "content_block_stop" | "content_block_start" => {
                        metadata_only_count += 1;
                    }
                    _ => {}
                }
            }
        }
        
        // Stream is meaningful if:
        // - It has content_block_delta events, OR
        // - It has message_start and is not just a few metadata events
        has_content_deltas || (has_message_start && accumulator.events.len() > 3 && metadata_only_count < accumulator.events.len())
    }

    /// Accumulate content from content_block_delta events - matches ssl_log_analyzer.py logic
    fn accumulate_content(accumulator: &mut SSEAccumulator, events: &[SSEEvent], debug: bool) {
        let mut chunk_text_parts = Vec::new();
        
        for event in events {
            accumulator.events.push(event.clone());
            
            // Check event type (matches ssl_log_analyzer.py)
            if let Some(event_type) = &event.event {
                if debug {
                    eprintln!("[DEBUG]   Processing event type: {}", event_type);
                }
                
                match event_type.as_str() {
                    "message_start" => {
                        accumulator.has_message_start = true;
                        // Extract message ID
                        if accumulator.message_id.is_none() {
                            accumulator.message_id = Self::extract_message_id(&[event.clone()]);
                        }
                        if debug {
                            eprintln!("[DEBUG]     Found message_start, has_message_start=true");
                        }
                    }
                    "content_block_delta" => {
                        // Handle deltas - matches ssl_log_analyzer.py logic
                        if let Some(parsed_data) = &event.parsed_data {
                            if let Some(delta) = parsed_data.get("delta") {
                                let mut text = String::new();
                                
                                // Handle text delta
                                if delta.get("type").and_then(|v| v.as_str()) == Some("text_delta") {
                                    if let Some(text_value) = delta.get("text").and_then(|v| v.as_str()) {
                                        text = text_value.to_string();
                                        if debug {
                                            eprintln!("[DEBUG]     Extracted text_delta: '{}'", text);
                                        }
                                    }
                                }
                                // Handle thinking delta
                                else if delta.get("type").and_then(|v| v.as_str()) == Some("thinking_delta") {
                                    if let Some(thinking_value) = delta.get("thinking").and_then(|v| v.as_str()) {
                                        text = thinking_value.to_string();
                                        if debug {
                                            eprintln!("[DEBUG]     Extracted thinking_delta: '{}'", text);
                                        }
                                    }
                                }
                                
                                if !text.is_empty() {
                                    chunk_text_parts.push(text.clone());
                                    accumulator.accumulated_text.push_str(&text);
                                }
                                
                                // Handle JSON delta (partial_json)
                                if let Some(partial_json) = delta.get("partial_json").and_then(|v| v.as_str()) {
                                    accumulator.accumulated_json.push_str(partial_json);
                                    if debug {
                                        eprintln!("[DEBUG]     Extracted partial_json: '{}'", partial_json);
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        if debug {
                            eprintln!("[DEBUG]     Skipping event type: {}", event_type);
                        }
                    }
                }
            } else if debug {
                eprintln!("[DEBUG]   Event with no type field");
            }
        }
        
        if debug && !chunk_text_parts.is_empty() {
            eprintln!("[DEBUG]   Accumulated {} text parts: {:?}", chunk_text_parts.len(), chunk_text_parts);
        }
    }

    /// Warn about unexpected conditions (debug output only)
    fn warn_if_unexpected(&self, accumulator: &SSEAccumulator, connection_id: &str) {
        // Warn if we're completing without message_start
        if !accumulator.has_message_start && accumulator.events.len() > 0 {
            self.debug_print(&format!(" SSEProcessor: Completing SSE stream {} without message_start event - may be incomplete!", connection_id));
        }
        
        // Warn if we have very little content
        if accumulator.accumulated_text.len() == 0 && accumulator.accumulated_json.len() == 0 && accumulator.events.len() > 3 {
            self.debug_print(&format!(" SSEProcessor: SSE stream {} has {} events but no accumulated content - possible parsing issue!", connection_id, accumulator.events.len()));
        }
        
        // Warn if JSON looks incomplete
        if !accumulator.accumulated_json.is_empty() && !accumulator.accumulated_json.starts_with('{') {
            self.debug_print(&format!(" SSEProcessor: SSE stream {} has JSON content that doesn't start with '{{' - may be incomplete: {}", connection_id, &accumulator.accumulated_json[..std::cmp::min(50, accumulator.accumulated_json.len())]));
        }
        
        // Warn if buffer size limit was hit
        if accumulator.accumulated_text.len() > 10240 || accumulator.accumulated_json.len() > 10240 {
            self.debug_print(&format!(" SSEProcessor: SSE stream {} hit buffer size limit - may be incomplete!", connection_id));
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
            "sse_processor".to_string(),
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
                    "event": e.event,
                    "data": e.data,
                    "id": e.id,
                    "parsed_data": e.parsed_data,
                    "raw_data": e.raw_data
                })).collect::<Vec<_>>()
            })
        )
    }
}

#[async_trait]
impl Analyzer for SSEProcessor {
    async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
        let sse_buffers = Arc::clone(&self.sse_buffers);

        self.debug_print("[DEBUG] SSEProcessor: Starting SSE event processing");
        
        let debug = self.debug;
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

                // Check if this chunk contains only metadata events (no content potential)
                let has_content_potential = sse_events.iter().any(|sse_event| {
                    if let Some(event_type) = &sse_event.event {
                        match event_type.as_str() {
                            // These events can contain or lead to content
                            "message_start" | "content_block_start" | "content_block_delta" => true,
                            // These are pure metadata
                            "message_stop" | "message_delta" | "ping" | "content_block_stop" => false,
                            // Unknown events might have content
                            _ => true,
                        }
                    } else {
                        // Events without type might have content
                        true
                    }
                });

                // If this chunk has no content potential and no existing accumulator, skip it
                if !has_content_potential {
                    let connection_id = Self::generate_connection_id(&event, &sse_events);
                    let buffers_lock = buffers.lock().unwrap();
                    let has_existing_accumulator = buffers_lock.contains_key(&connection_id);
                    drop(buffers_lock);
                    
                    if !has_existing_accumulator {
                        if debug {
                            eprintln!("[DEBUG] Skipping metadata-only chunk with no existing accumulator: {:?}", 
                                     sse_events.iter().map(|e| e.event.as_deref().unwrap_or("none")).collect::<Vec<_>>());
                        }
                        return None;
                    }
                }

                if debug {
                    eprintln!("[DEBUG] Processing SSE chunk at timestamp {} - found {} events", 
                             event.timestamp, sse_events.len());
                    // Log event types for each SSE event
                    for (i, sse_event) in sse_events.iter().enumerate() {
                        let event_type = sse_event.event.as_deref().unwrap_or("none");
                        eprintln!("[DEBUG]   Event {}: type={}", i + 1, event_type);
                    }
                    std::io::stdout().flush().unwrap();
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
                Self::accumulate_content(accumulator, &sse_events, debug);
                
                // Check if stream is complete
                if Self::is_sse_complete(accumulator) {
                    // Add detailed debug output like ssl_log_analyzer.py _finalize_sse_response
                    if debug {
                        eprintln!("[DEBUG] Finalizing SSE response:");
                        eprintln!("  - Text parts: {:?}", accumulator.accumulated_text);
                        eprintln!("  - JSON parts: {:?}", accumulator.accumulated_json);
                        eprintln!("  - Merged text: '{}'", accumulator.accumulated_text);
                        eprintln!("  - Merged JSON: '{}'", accumulator.accumulated_json);
                        eprintln!("  - Event count: {}", accumulator.events.len());
                        eprintln!("[DEBUG] SSEProcessor: Completed SSE stream for connection {} - {} text chars, {} json chars, {} events", 
                                final_connection_id, 
                                accumulator.accumulated_text.len(),
                                accumulator.accumulated_json.len(),
                                accumulator.events.len());
                        std::io::stdout().flush().unwrap();
                    }
                    
                    // Only create merged event if stream has meaningful content
                    let result_event = if Self::has_meaningful_content(accumulator) {
                        let merged_event = Self::create_merged_event(
                            final_connection_id.clone(),
                            accumulator,
                            &event,
                        );
                        Some(merged_event)
                    } else {
                        if debug {
                            eprintln!("[DEBUG] SSE stream {} contains no meaningful content - skipping event creation", final_connection_id);
                        }
                        None
                    };
                    
                    // Clear this accumulator
                    buffers_lock.remove(&final_connection_id);
                    drop(buffers_lock);
                    
                    result_event
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

 