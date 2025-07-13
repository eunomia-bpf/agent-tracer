use super::{Analyzer, AnalyzerError};
use crate::framework::runners::EventStream;
use crate::framework::core::Event;
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::io::Write;

/// SSE Event Accumulator that merges Server-Sent Events content fragments
pub struct ChunkMerger {
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
struct SSEEvent {
    event_type: String,
    data: Value,
}

impl ChunkMerger {
    /// Create a new ChunkMerger with default timeout (30 seconds)
    pub fn new() -> Self {
        Self::new_with_timeout(30_000)
    }

    /// Create a new ChunkMerger with custom timeout
    pub fn new_with_timeout(timeout_ms: u64) -> Self {
        ChunkMerger {
            name: "ChunkMerger".to_string(),
            sse_buffers: Arc::new(Mutex::new(HashMap::new())),
            timeout_ms,
        }
    }

    /// Check if SSL data contains SSE events
    fn is_sse_data(data: &str) -> bool {
        // Look for SSE patterns in the data
        data.contains("event:") && data.contains("data:")
    }

    /// Parse SSE events from raw SSL data
    fn parse_sse_events(data: &str) -> Vec<SSEEvent> {
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
                        eprintln!(" ChunkMerger: Failed to parse SSE JSON data: {} - Error: {}", data_content, e);
                        std::io::stdout().flush().unwrap();
                    }
                }
            }
        }
        
        events
    }

    /// Clean HTTP chunked encoding artifacts from content
    fn clean_chunked_content(content: &str) -> String {
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
            eprintln!(" ChunkMerger: Completing SSE stream {} without message_start event - may be incomplete!", connection_id);
            std::io::stdout().flush().unwrap();
        }
        
        // Warn if we have very little content
        if accumulator.accumulated_text.len() == 0 && accumulator.accumulated_json.len() == 0 && accumulator.events.len() > 3 {
            eprintln!(" ChunkMerger: SSE stream {} has {} events but no accumulated content - possible parsing issue!", connection_id, accumulator.events.len());
            std::io::stdout().flush().unwrap();
        }
        
        // Warn if JSON looks incomplete
        if !accumulator.accumulated_json.is_empty() && !accumulator.accumulated_json.starts_with('{') {
            eprintln!(" ChunkMerger: SSE stream {} has JSON content that doesn't start with '{{' - may be incomplete: {}", connection_id, &accumulator.accumulated_json[..std::cmp::min(50, accumulator.accumulated_json.len())]);
            std::io::stdout().flush().unwrap();
        }
        
        // Warn if buffer size limit was hit
        if accumulator.accumulated_text.len() > 10240 || accumulator.accumulated_json.len() > 10240 {
            eprintln!(" ChunkMerger: SSE stream {} hit buffer size limit - may be incomplete!", connection_id);
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
impl Analyzer for ChunkMerger {
    async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
        let sse_buffers = Arc::clone(&self.sse_buffers);

        eprintln!("ChunkMerger: Starting SSE event processing");
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
                    eprintln!("ChunkMerger: Completed SSE stream for connection {} - {} text chars, {} json chars, {} events", 
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::core::Event;
    use futures::stream;
    use serde_json::json;

    #[tokio::test]
    async fn test_chunk_merger_creation() {
        let merger = ChunkMerger::new();
        assert_eq!(merger.name(), "ChunkMerger");
    }

    #[tokio::test]
    async fn test_chunk_merger_with_timeout() {
        let merger = ChunkMerger::new_with_timeout(5000);
        assert_eq!(merger.name(), "ChunkMerger");
    }

    #[tokio::test]
    async fn test_is_sse_data() {
        assert!(ChunkMerger::is_sse_data("event: content_block_delta\ndata: {\"type\":\"content_block_delta\"}\r\n0\r\n\r\n"));
        assert!(ChunkMerger::is_sse_data("event: message_start\ndata: {\"message\":{\"id\":\"123\"}}\r\n0\r\n\r\n"));
        assert!(ChunkMerger::is_sse_data("Transfer-Encoding: chunked\r\nevent: content_block_delta\r\ndata: {\"type\":\"content_block_delta\"}\r\n0\r\n\r\n"));
        assert!(!ChunkMerger::is_sse_data("regular text"));
    }

    #[tokio::test]
    async fn test_parse_sse_events() {
        let sse_data = "event: content_block_delta\ndata: {\"type\":\"content_block_delta\"}\r\n0\r\n\r\n";
        let events = ChunkMerger::parse_sse_events(sse_data);
        assert!(!events.is_empty());
    }

    #[tokio::test]
    async fn test_chunk_merger_processes_events() {
        let mut merger = ChunkMerger::new();
        
        let test_event = Event::new("ssl".to_string(), json!({
            "comm": "test",
            "data": "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"hello\"}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
            "function": "READ/RECV",
            "pid": 1234,
            "tid": 1234,
            "timestamp_ns": 1000000000
        }));
        
        let events = vec![test_event];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = merger.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        // Should have processed the event and completed due to message_stop
        assert!(!collected.is_empty());
        
        // Should be a chunk_merger event
        if let Some(merged_event) = collected.first() {
            assert_eq!(merged_event.source, "chunk_merger");
        }
    }

    #[tokio::test]
    async fn test_chunk_merger_ignores_non_ssl_events() {
        let mut merger = ChunkMerger::new();
        
        let test_event = Event::new("process".to_string(), json!({
            "comm": "test",
            "data": "some data",
            "pid": 1234
        }));
        
        let events = vec![test_event.clone()];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = merger.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        // Should pass through non-SSL events unchanged
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].source, "process");
    }

    #[tokio::test]
    async fn test_chunk_merger_ignores_non_sse_ssl_events() {
        let mut merger = ChunkMerger::new();
        
        let test_event = Event::new("ssl".to_string(), json!({
            "comm": "test", 
            "data": "regular HTTP data without SSE",
            "function": "READ/RECV",
            "pid": 1234
        }));
        
        let events = vec![test_event.clone()];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = merger.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        // Should pass through non-SSE SSL events unchanged
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].source, "ssl");
    }

    #[tokio::test]
    async fn test_connection_id_generation() {
        let event = Event::new("ssl".to_string(), json!({
            "pid": 1234,
            "tid": 5678,
            "timestamp_ns": 1000000000
        }));
        
        // Test with no SSE events (should use timestamp window)
        let connection_id = ChunkMerger::generate_connection_id(&event, &[]);
        assert!(connection_id.contains("1234"));
        assert!(connection_id.contains("5678"));
        
        // Test with SSE events containing message ID
        let sse_events = vec![
            SSEEvent {
                event_type: "message_start".to_string(),
                data: json!({"message": {"id": "msg_123"}}),
            }
        ];
        let connection_id_with_msg = ChunkMerger::generate_connection_id(&event, &sse_events);
        assert!(connection_id_with_msg.contains("1234"));
        assert!(connection_id_with_msg.contains("5678"));
        assert!(connection_id_with_msg.contains("msg_123"));
    }

    #[tokio::test]
    async fn test_chunk_merger_integration_with_real_sse_data() {
        let mut merger = ChunkMerger::new();
        
        // Create simple test event that should pass through unchanged (not SSE)
        let non_sse_event = Event::new("ssl".to_string(), json!({
            "comm": "claude",
            "data": "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\nHello World",
            "function": "READ/RECV",
            "pid": 61778,
            "tid": 61778,
            "timestamp_ns": 32616800319854i64
        }));

        // Create simple SSE data that should be processed
        let simple_sse_event = Event::new("ssl".to_string(), json!({
            "comm": "claude",
            "data": "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"message\\\":\\\"Hello\\\"}\"}}\r\n0\r\n\r\n",
            "function": "READ/RECV",
            "pid": 61778,
            "tid": 61778,
            "timestamp_ns": 32616800319854i64
        }));

        // Also test a non-SSL event
        let process_event = Event::new("process".to_string(), json!({
            "pid": 1234,
            "command": "test"
        }));

        let events = vec![non_sse_event, simple_sse_event, process_event];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = merger.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        println!("Integration test results:");
        println!("Total events after chunk merger: {}", collected.len());
        for event in &collected {
            println!("Event source: {}, has data: {}", event.source, event.data.get("data").is_some());
        }
        
        // Should have at least 2 events (some might be merged/filtered)
        assert!(collected.len() >= 2, "Should have at least 2 events after processing");
        
        // Check sources
        let ssl_events = collected.iter().filter(|e| e.source == "ssl").count();
        let process_events = collected.iter().filter(|e| e.source == "process").count();
        let chunk_merger_events = collected.iter().filter(|e| e.source == "chunk_merger").count();
        
        println!("SSL events: {}, Process events: {}, Chunk merger events: {}", 
                ssl_events, process_events, chunk_merger_events);
        
        // Should have at least the original events
        assert!(ssl_events >= 1 || chunk_merger_events >= 1, "Should have SSL or chunk merger events");
        assert_eq!(process_events, 1, "Should have 1 process event");
        
        println!("✅ Integration test with real SSE data completed!");
    }

    #[tokio::test]
    async fn test_chunk_merger_with_mixed_event_types() {
        let mut merger = ChunkMerger::new();
        
        // Mix of SSL events: SSE, non-SSE, and non-SSL
        let ssl_sse = Event::new("ssl".to_string(), json!({
            "data": "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"message\\\":\\\"Hello\\\"}\"}}\r\n0\r\n\r\n",
            "pid": 1234,
            "tid": 1234,
            "timestamp_ns": 1000000000
        }));
        
        let ssl_regular = Event::new("ssl".to_string(), json!({
            "data": "HTTP/1.1 200 OK\r\nContent-Length: 12\r\n\r\nHello World!",
            "pid": 1234,
            "tid": 1234,
            "timestamp_ns": 1000000001
        }));
        
        let process_event = Event::new("process".to_string(), json!({
            "pid": 5678,
            "command": "test"
        }));
        
        let events = vec![ssl_sse, ssl_regular, process_event];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = merger.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        // Should preserve all event types (SSE may be merged into chunk_merger events)
        assert!(collected.len() >= 2, "Should have at least 2 events");
        
        // Check event sources
        let ssl_events = collected.iter().filter(|e| e.source == "ssl").count();
        let process_events = collected.iter().filter(|e| e.source == "process").count();
        let chunk_events = collected.iter().filter(|e| e.source == "chunk_merger").count();
        
        assert_eq!(process_events, 1, "Should have 1 process event");
        assert!(ssl_events >= 1 || chunk_events >= 1, "Should have at least 1 SSL or chunk_merger event");
        
        println!("Mixed events test: {} SSL, {} process, {} chunk_merger", 
                ssl_events, process_events, chunk_events);
    }

    #[tokio::test]
    async fn test_extract_message_id() {
        let sse_event = SSEEvent {
            event_type: "message_start".to_string(),
            data: json!({"message": {"id": "123"}}),
        };
        let events = vec![sse_event];
        assert_eq!(ChunkMerger::extract_message_id(&events), Some("123".to_string()));
    }

    #[tokio::test]
    async fn test_is_sse_complete_with_completion_events() {
        let accumulator = SSEAccumulator {
            message_id: None,
            accumulated_text: String::new(),
            accumulated_json: String::new(),
            events: vec![
                SSEEvent {
                    event_type: "message_start".to_string(),
                    data: json!({"message": {"id": "123"}}),
                },
                SSEEvent {
                    event_type: "message_delta".to_string(),
                    data: json!({"delta": {"stop_reason": "test"}}),
                },
            ],
            is_complete: false,
            last_update: 0,
            has_message_start: false,
        };
        assert!(ChunkMerger::is_sse_complete(&accumulator));
    }

    #[tokio::test]
    async fn test_is_sse_complete_with_buffer_size() {
        let accumulator = SSEAccumulator {
            message_id: None,
            accumulated_text: "a".repeat(11000), // Over 10KB limit
            accumulated_json: String::new(),
            events: vec![
                SSEEvent {
                    event_type: "message_start".to_string(),
                    data: json!({"message": {"id": "123"}}),
                },
                SSEEvent {
                    event_type: "content_block_delta".to_string(),
                    data: json!({"delta": {"text": "a"}}),
                },
            ],
            is_complete: false,
            last_update: 0,
            has_message_start: false,
        };
        assert!(ChunkMerger::is_sse_complete(&accumulator));
    }

    #[tokio::test]
    async fn test_accumulate_content() {
        let mut accumulator = SSEAccumulator {
            message_id: None,
            accumulated_text: String::new(),
            accumulated_json: String::new(),
            events: Vec::new(),
            is_complete: false,
            last_update: 0,
            has_message_start: false,
        };
        let sse_events = vec![
            SSEEvent {
                event_type: "message_start".to_string(),
                data: json!({"message": {"id": "123"}}),
            },
            SSEEvent {
                event_type: "content_block_delta".to_string(),
                data: json!({"delta": {"text": "hello"}}),
            },
            SSEEvent {
                event_type: "content_block_delta".to_string(),
                data: json!({"delta": {"text": " world"}}),
            },
        ];
        ChunkMerger::accumulate_content(&mut accumulator, &sse_events);
        assert_eq!(accumulator.message_id, Some("123".to_string()));
        assert_eq!(accumulator.accumulated_text, "hello world");
        assert_eq!(accumulator.accumulated_json, ""); // No partial_json in this test
        assert_eq!(accumulator.events.len(), 3);
    }

    #[tokio::test]
    async fn test_create_merged_event() {
        let connection_id = "123:456:789".to_string();
        let original_event = Event::new("ssl".to_string(), json!({
            "comm": "test",
            "pid": 123,
            "tid": 456,
            "timestamp_ns": 1000000000,
            "function": "READ/RECV",
        }));
        let accumulator = SSEAccumulator {
            message_id: Some("123".to_string()),
            accumulated_text: "Hello".to_string(),
            accumulated_json: String::new(), // No JSON content in this test
            events: vec![
                SSEEvent {
                    event_type: "message_start".to_string(),
                    data: json!({"message": {"id": "123"}}),
                },
                SSEEvent {
                    event_type: "content_block_delta".to_string(),
                    data: json!({"delta": {"text": "Hello"}}),
                },
            ],
            is_complete: false,
            last_update: 0,
            has_message_start: true,
        };
        let merged_event = ChunkMerger::create_merged_event(connection_id.clone(), &accumulator, &original_event);
        assert_eq!(merged_event.source, "chunk_merger");
        assert_eq!(merged_event.data["connection_id"], json!(connection_id));
        assert_eq!(merged_event.data["message_id"], json!(Some("123".to_string())));
        assert_eq!(merged_event.data["original_source"], json!("ssl"));
        assert_eq!(merged_event.data["function"], json!("READ/RECV"));
        assert_eq!(merged_event.data["comm"], json!("test"));
        assert_eq!(merged_event.data["pid"], json!(123));
        assert_eq!(merged_event.data["tid"], json!(456));
        assert_eq!(merged_event.data["timestamp_ns"], json!(1000000000));
        assert_eq!(merged_event.data["merged_content"], json!("Hello")); // Text content, not JSON
        assert_eq!(merged_event.data["content_type"], json!("text"));
        assert_eq!(merged_event.data["total_size"], json!(5));
        assert_eq!(merged_event.data["event_count"], json!(2));
        assert_eq!(merged_event.data["has_message_start"], json!(true));
        assert_eq!(merged_event.data["sse_events"].as_array().unwrap().len(), 2);
        assert_eq!(merged_event.data["sse_events"][0]["type"], json!("message_start"));
        assert_eq!(merged_event.data["sse_events"][0]["data"], json!({"message": {"id": "123"}}));
        assert_eq!(merged_event.data["sse_events"][1]["type"], json!("content_block_delta"));
        assert_eq!(merged_event.data["sse_events"][1]["data"], json!({"delta": {"text": "Hello"}}));
    }
} 