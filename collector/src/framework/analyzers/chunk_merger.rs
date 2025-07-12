use super::{Analyzer, AnalyzerError};
use crate::framework::runners::EventStream;
use async_trait::async_trait;
use chunked_transfer::Decoder;
use futures::stream::StreamExt;
use log::debug;
use serde_json::json;
use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::io::Write; // Add this for stdout flushing

/// ChunkMerger analyzer that merges HTTP chunked transfer encoding fragments
pub struct ChunkMerger {
    name: String,
    /// Store chunked streams by connection ID (using timestamp as approximation)
    chunk_buffers: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    /// Timeout for incomplete chunks (in milliseconds)
    timeout_ms: u64,
}

impl ChunkMerger {
    /// Create a new ChunkMerger with default timeout
    pub fn new() -> Self {
        Self::new_with_timeout(30000) // 30 seconds default timeout
    }

    /// Create a new ChunkMerger with custom timeout
    pub fn new_with_timeout(timeout_ms: u64) -> Self {
        Self {
            name: "ChunkMerger".to_string(),
            chunk_buffers: Arc::new(Mutex::new(HashMap::new())),
            timeout_ms,
        }
    }

    /// Check if data contains HTTP chunked transfer encoding
    fn is_chunked_data(data: &str) -> bool {
        // Look for typical chunked patterns:
        // 1. HTTP response with "Transfer-Encoding: chunked"
        if data.contains("Transfer-Encoding: chunked") {
            println!("🔧 ChunkMerger: DEBUG - Found Transfer-Encoding: chunked header");
            std::io::stdout().flush().unwrap();
            return true;
        }
        
        // 2. Check for chunk size pattern (hex number followed by \r\n)
        let lines: Vec<&str> = data.split('\n').collect();
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_end_matches('\r');
            if !trimmed.is_empty() {
                // Try to parse as hex number
                if let Ok(size) = u32::from_str_radix(trimmed, 16) {
                    println!("🔧 ChunkMerger: DEBUG - Found potential chunk size '{}' = {} at line {}", trimmed, size, i);
                    std::io::stdout().flush().unwrap();
                    return true;
                }
            }
        }
        
        // 3. Check for SSE (Server-Sent Events) patterns which are often sent chunked
        if data.contains("event:") && (data.contains("data:") || data.contains("id:")) {
            println!("🔧 ChunkMerger: DEBUG - Found SSE pattern");
            std::io::stdout().flush().unwrap();
            return true;
        }
        
        false
    }

    /// Extract chunks from HTTP chunked transfer encoding
    fn extract_chunks(data: &str) -> Option<Vec<String>> {
        if !Self::is_chunked_data(data) {
            return None;
        }

        // If this is just SSE data without chunk headers, treat it as a single chunk
        if data.contains("event:") && !data.contains("Transfer-Encoding: chunked") {
            return Some(vec![data.to_string()]);
        }

        let mut chunks = Vec::new();
        let mut decoder = Decoder::new(data.as_bytes());
        let mut decoded = String::new();
        
        match decoder.read_to_string(&mut decoded) {
            Ok(_) => {
                if !decoded.is_empty() {
                    chunks.push(decoded);
                }
                Some(chunks)
            }
            Err(_) => {
                // If full decoding fails, try to extract partial chunks
                Self::extract_partial_chunks(data)
            }
        }
    }

    /// Extract partial chunks when full decoding fails
    fn extract_partial_chunks(data: &str) -> Option<Vec<String>> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = data.split('\n').collect();
        let mut i = 0;
        let mut found_chunk_header = false;
        
        while i < lines.len() {
            let line = lines[i].trim_end_matches('\r');
            
            // Try to parse as hex chunk size
            if let Ok(chunk_size) = u32::from_str_radix(line, 16) {
                found_chunk_header = true;
                
                if chunk_size == 0 {
                    // End of chunks
                    break;
                }
                
                // Get the chunk data
                i += 1;
                if i < lines.len() {
                    let chunk_data = lines[i].trim_end_matches('\r');
                    if chunk_data.len() <= chunk_size as usize {
                        chunks.push(chunk_data.to_string());
                    }
                }
            }
            i += 1;
        }
        
        // If we didn't find any chunk headers, treat the entire data as a single chunk
        if !found_chunk_header && !data.is_empty() {
            chunks.push(data.to_string());
        }
        
        if chunks.is_empty() {
            None
        } else {
            Some(chunks)
        }
    }

    /// Generate a connection ID from event data
    fn generate_connection_id(event: &crate::framework::core::Event) -> String {
        // Use combination of PID, TID, and rough timestamp to identify connection
        let pid = event.data.get("pid").and_then(|v| v.as_u64()).unwrap_or(0);
        let tid = event.data.get("tid").and_then(|v| v.as_u64()).unwrap_or(0);
        let timestamp = event.timestamp;
        
        // Group by connection using timestamp windows (1 second windows)
        let window = timestamp / 1_000_000_000; // Convert to seconds
        format!("{}:{}:{}", pid, tid, window)
    }

    /// Merge chunks for a connection
    fn merge_chunks(chunks: &[String]) -> String {
        chunks.join("")
    }
}

#[async_trait]
impl Analyzer for ChunkMerger {
    async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
        let chunk_buffers = Arc::clone(&self.chunk_buffers);

        println!("🔧 ChunkMerger: Processing stream STARTED");
        std::io::stdout().flush().unwrap(); // Force flush
        
        let processed_stream = stream.filter_map(move |event| {
            let buffers = Arc::clone(&chunk_buffers);
            
            async move {
                // Only process SSL events with data
                if event.source != "ssl" {
                    return Some(event);
                }

                let data_str = match event.data.get("data")
                    .and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return Some(event),
                };

                // Check if this is HTTP chunked data
                if !Self::is_chunked_data(data_str) {
                    return Some(event);
                }

                println!("🔧 ChunkMerger: Found chunked data! Processing...");
                std::io::stdout().flush().unwrap();

                // Extract chunks from this event
                let chunks = match Self::extract_chunks(data_str) {
                    Some(chunks) => chunks,
                    None => {
                        println!("🔧 ChunkMerger: Failed to extract chunks, returning original event");
                        std::io::stdout().flush().unwrap();
                        return Some(event);
                    },
                };

                let connection_id = Self::generate_connection_id(&event);
                
                // Store/merge chunks for this connection
                let mut buffers_lock = buffers.lock().unwrap();
                let buffer = buffers_lock.entry(connection_id.clone()).or_insert_with(Vec::new);
                
                // Add chunks to buffer
                for chunk in chunks {
                    buffer.extend_from_slice(chunk.as_bytes());
                }

                // Check if we have a complete message
                let buffer_str = String::from_utf8_lossy(buffer);
                println!("🔧 ChunkMerger: Buffer size: {}", buffer.len());
                std::io::stdout().flush().unwrap();
                
                // Improved completion conditions for SSE and chunked data
                let is_complete = 
                    // Traditional HTTP chunked ending
                    buffer_str.contains("0\r\n\r\n") || 
                    buffer_str.ends_with("0\r\n\r\n") ||
                    // SSE completion patterns
                    buffer_str.contains("event: message_stop") ||
                    buffer_str.contains("event: done") ||
                    buffer_str.contains("event: error") ||
                    // Size-based completion - flush large buffers
                    buffer.len() > 8192 ||
                    // Time-based completion - if this looks like a complete SSE message
                    (buffer_str.contains("event:") && buffer_str.contains("data:") && buffer.len() > 100);
                
                if is_complete {
                    println!("🔧 ChunkMerger: Completion condition met! Buffer size: {}", buffer.len());
                    std::io::stdout().flush().unwrap();
                    
                    // Complete message, create merged event
                    let merged_data = buffer_str.to_string();
                    let final_buffer = buffer.clone();
                    buffers_lock.remove(&connection_id);
                    drop(buffers_lock);
                    
                    // Create merged event
                    let mut merged_event = event.clone();
                    merged_event.source = "chunk_merger".to_string();
                    merged_event.data = json!({
                        "original_source": "ssl",
                        "merged_data": merged_data,
                        "chunk_count": 1, // Could be enhanced to count actual chunks
                        "connection_id": connection_id,
                        "total_size": final_buffer.len(),
                        "comm": event.data.get("comm").unwrap_or(&json!("unknown")),
                        "function": event.data.get("function").unwrap_or(&json!("unknown")),
                        "pid": event.data.get("pid").unwrap_or(&json!(0)),
                        "tid": event.data.get("tid").unwrap_or(&json!(0)),
                        "timestamp_ns": event.data.get("timestamp_ns").unwrap_or(&json!(0))
                    });
                    println!("🔧 ChunkMerger: Created merged event with {} bytes", final_buffer.len());
                    std::io::stdout().flush().unwrap();
                    Some(merged_event)
                } else {
                    // Incomplete message, don't emit event yet
                    None
                }
            }
        });

        println!("🔧 ChunkMerger: Returning processed stream");
        std::io::stdout().flush().unwrap();
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
    async fn test_is_chunked_data() {
        assert!(ChunkMerger::is_chunked_data("Transfer-Encoding: chunked"));
        assert!(ChunkMerger::is_chunked_data("7c\r\nevent: content_block_delta"));
        assert!(ChunkMerger::is_chunked_data("0\r\n\r\n"));
        assert!(!ChunkMerger::is_chunked_data("regular text"));
    }

    #[tokio::test]
    async fn test_extract_chunks() {
        let chunked_data = "7c\r\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\"}\r\n0\r\n\r\n";
        let chunks = ChunkMerger::extract_chunks(chunked_data);
        assert!(chunks.is_some());
    }

    #[tokio::test]
    async fn test_chunk_merger_processes_events() {
        let mut merger = ChunkMerger::new();
        
        let test_event = Event::new("ssl".to_string(), json!({
            "comm": "test",
            "data": "7c\r\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\"}\r\n0\r\n\r\n",
            "function": "READ/RECV",
            "pid": 1234,
            "tid": 1234,
            "timestamp_ns": 1000000000
        }));
        
        let events = vec![test_event];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = merger.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        // Should have processed the event
        assert!(!collected.is_empty());
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
    async fn test_chunk_merger_ignores_non_chunked_ssl_events() {
        let mut merger = ChunkMerger::new();
        
        let test_event = Event::new("ssl".to_string(), json!({
            "comm": "test", 
            "data": "regular HTTP data without chunks",
            "function": "READ/RECV",
            "pid": 1234
        }));
        
        let events = vec![test_event.clone()];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = merger.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        // Should pass through non-chunked SSL events unchanged
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
        
        let connection_id = ChunkMerger::generate_connection_id(&event);
        // Should include PID, TID, and timestamp window
        assert!(connection_id.contains("1234"));
        assert!(connection_id.contains("5678"));
    }

    #[tokio::test]
    async fn test_chunk_merger_integration_with_real_chunked_data() {
        let mut merger = ChunkMerger::new();
        
        // Create simple test event that should pass through unchanged (not chunked)
        let non_chunked_event = Event::new("ssl".to_string(), json!({
            "comm": "claude",
            "data": "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\nHello World",
            "function": "READ/RECV",
            "pid": 61778,
            "tid": 61778,
            "timestamp_ns": 32616800319854i64
        }));

        // Create simple chunked data that should be processed
        let simple_chunked_event = Event::new("ssl".to_string(), json!({
            "comm": "claude",
            "data": "5\r\nhello\r\n0\r\n\r\n",
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

        let events = vec![non_chunked_event, simple_chunked_event, process_event];
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
        
        println!("✅ Integration test with real chunked data completed!");
    }

    #[tokio::test]
    async fn test_chunk_merger_with_mixed_event_types() {
        let mut merger = ChunkMerger::new();
        
        // Mix of SSL events: chunked, non-chunked, and non-SSL
        let ssl_chunked = Event::new("ssl".to_string(), json!({
            "data": "a\r\nhello world\r\n0\r\n\r\n",
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
        
        let events = vec![ssl_chunked, ssl_regular, process_event];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = merger.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        // Should preserve all event types
        assert_eq!(collected.len(), 3, "Should preserve all events");
        
        // Check event sources
        let ssl_events = collected.iter().filter(|e| e.source == "ssl").count();
        let process_events = collected.iter().filter(|e| e.source == "process").count();
        let chunk_events = collected.iter().filter(|e| e.source == "chunk_merger").count();
        
        assert_eq!(process_events, 1, "Should have 1 process event");
        assert!(ssl_events >= 1, "Should have at least 1 SSL event");
        
        println!("Mixed events test: {} SSL, {} process, {} chunk_merger", 
                ssl_events, process_events, chunk_events);
    }
} 