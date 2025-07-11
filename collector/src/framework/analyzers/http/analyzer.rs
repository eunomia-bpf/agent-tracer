use super::types::{PendingRequest, HttpResponse};
use super::parser::HttpParser;
use crate::framework::analyzers::{Analyzer, AnalyzerError};
use crate::framework::core::Event;
use crate::framework::runners::EventStream;
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

/// HTTP Analyzer that pairs HTTP requests with responses
pub struct HttpAnalyzer {
    name: String,
    pending_requests: HashMap<String, PendingRequest>, // Key: "pid_url"
    thread_buffers: HashMap<u32, String>, // Buffer partial HTTP data per thread/PID
    max_wait_time_ms: u64,
}

impl HttpAnalyzer {
    /// Create a new HttpAnalyzer with default settings
    pub fn new() -> Self {
        Self {
            name: "HttpAnalyzer".to_string(),
            pending_requests: HashMap::new(),
            thread_buffers: HashMap::new(),
            max_wait_time_ms: 30000, // 30 seconds default
        }
    }

    /// Create a new HttpAnalyzer with custom wait time
    pub fn new_with_wait_time(max_wait_time_ms: u64) -> Self {
        Self {
            name: "HttpAnalyzer".to_string(),
            pending_requests: HashMap::new(),
            thread_buffers: HashMap::new(),
            max_wait_time_ms,
        }
    }

    /// Create a request/response pair event with debug information
    fn create_request_response_pair(
        request: &PendingRequest,
        response: &HttpResponse,
    ) -> Event {
        let pair_data = json!({
            "type": "http_request_response_pair",
            "thread_id": request.pid,
            "request": {
                "method": request.method,
                "url": request.url,
                "headers": request.headers,
                "body": request.body,
                "timestamp": request.timestamp,
                "original_json": request.original_json
            },
            "response": {
                "status_code": response.status_code,
                "status_text": response.status_text,
                "headers": response.headers,
                "body": response.body,
                "timestamp": response.timestamp,
                "original_json": response.original_json
            },
            "duration_ms": response.timestamp.saturating_sub(request.timestamp),
            "debug_info": {
                "request_stored_at": request.timestamp,
                "response_received_at": response.timestamp,
                "matched_by": "thread_id",
                "thread_id": request.pid
            }
        });

        Event::new("http_analyzer".to_string(), pair_data)
    }

    /// Clean up expired pending requests
    fn cleanup_expired_requests(&mut self, current_time: u64) {
        let expired_keys: Vec<String> = self.pending_requests
            .iter()
            .filter(|(_, req)| current_time.saturating_sub(req.timestamp) > self.max_wait_time_ms)
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            if let Some(expired_request) = self.pending_requests.remove(&key) {
                eprintln!("HTTP Analyzer: Request expired after {}ms: {} {} (thread: {})", 
                    self.max_wait_time_ms, expired_request.method, expired_request.url, expired_request.pid);
                eprintln!("HTTP Analyzer: Original JSON: {}", 
                    serde_json::to_string_pretty(&expired_request.original_json).unwrap_or_default());
            }
        }
    }

    /// Process HTTP request from SSL data
    fn process_request(&mut self, data_str: &str, event: &Event, pid: u32) {
        eprintln!("HTTP Analyzer: Found HTTP request in SSL data for thread {}", pid);
        if let Some(request) = HttpParser::parse_http_request(data_str, event) {
            let key = format!("{}_{}", request.pid, request.url);
            eprintln!("HTTP Analyzer: Storing request with key: {}", key);
            eprintln!("HTTP Analyzer: Request details: {} {} (thread: {})", 
                request.method, request.url, request.pid);
            
            self.pending_requests.insert(key, request.clone());
            eprintln!("HTTP Analyzer: Total pending requests: {}", self.pending_requests.len());
        } else {
            eprintln!("HTTP Analyzer: Failed to parse HTTP request");
        }
    }

    /// Process HTTP response from SSL data
    fn process_response(&mut self, data_str: &str, event: &Event, pid: u32, current_time: u64, tx: &mpsc::UnboundedSender<Event>) -> bool {
        eprintln!("HTTP Analyzer: Found HTTP response in SSL data for thread {}", pid);
        if let Some(response) = HttpParser::parse_http_response(data_str, event) {
            eprintln!("HTTP Analyzer: Response details: {} {} (thread: {})", 
                response.status_code, response.status_text, response.pid);
            
            // Try to find matching request by thread/PID
            let mut matched_key = None;
            let mut best_match_score = f64::MAX;
            
            eprintln!("HTTP Analyzer: Looking for request match for thread: {}", pid);
            for (key, request) in &self.pending_requests {
                eprintln!("HTTP Analyzer: Checking pending request: {} (thread: {})", 
                    key, request.pid);
                
                // Match by thread/PID first, then by timing
                if request.pid == pid {
                    let time_diff = current_time.saturating_sub(request.timestamp) as f64;
                    eprintln!("HTTP Analyzer: Thread match found! Time diff: {}ms", time_diff);
                    if time_diff < self.max_wait_time_ms as f64 && time_diff < best_match_score {
                        best_match_score = time_diff;
                        matched_key = Some(key.clone());
                        eprintln!("HTTP Analyzer: Best match so far: {} ({}ms)", key, time_diff);
                    }
                }
            }

            if let Some(key) = matched_key {
                if let Some(request) = self.pending_requests.remove(&key) {
                    let pair_event = Self::create_request_response_pair(&request, &response);
                    eprintln!("HTTP Analyzer: Created request/response pair!");
                    eprintln!("HTTP Analyzer: Request:  {} {} (thread: {})", 
                        request.method, request.url, request.pid);
                    eprintln!("HTTP Analyzer: Response: {} {} ({}ms latency)", 
                        response.status_code, response.status_text, best_match_score);
                    eprintln!("HTTP Analyzer: Matched by thread ID: {}", request.pid);
                    
                    if tx.send(pair_event).is_err() {
                        return false;
                    }
                }
            } else {
                eprintln!("HTTP Analyzer: No matching request found for response: {} {} (thread: {})", 
                    response.status_code, response.status_text, response.pid);
                eprintln!("HTTP Analyzer: Current pending requests by thread:");
                for (key, req) in &self.pending_requests {
                    eprintln!("HTTP Analyzer:   {}: {} {} (thread: {})", 
                        key, req.method, req.url, req.pid);
                }
            }
        } else {
            eprintln!("HTTP Analyzer: Failed to parse HTTP response");
        }
        true
    }

    /// Process buffered HTTP data
    fn process_buffered_data(&mut self, data_str: &str, event: &Event, pid: u32, current_time: u64, tx: &mpsc::UnboundedSender<Event>) -> bool {
        eprintln!("HTTP Analyzer: Data doesn't start with HTTP, buffering for thread {}", pid);
        
        // Buffer the SSL data for this thread/PID
        let current_buffer = self.thread_buffers.entry(pid).or_insert_with(String::new);
        current_buffer.push_str(data_str);

        eprintln!("HTTP Analyzer: Buffered {} chars for thread {}, data: {}", 
            current_buffer.len(), pid,
            if data_str.len() > 100 { &data_str[..100] } else { data_str });

        // Extract complete HTTP messages from buffer (clone to avoid borrow issues)
        let buffer_clone = current_buffer.clone();
        let messages = HttpParser::extract_http_messages(&buffer_clone);
        
        if !messages.is_empty() {
            eprintln!("HTTP Analyzer: Found {} complete HTTP messages for thread {}", 
                messages.len(), pid);
            
            // Process each complete message
            for (message, _) in &messages {
                eprintln!("HTTP Analyzer: Processing buffered message: {}", 
                    if message.len() > 200 { &message[..200] } else { message });
                
                if HttpParser::starts_with_http_request(message) {
                    eprintln!("HTTP Analyzer: Found complete HTTP request for thread {}", pid);
                    if let Some(request) = HttpParser::parse_http_request(message, event) {
                        let key = format!("{}_{}", request.pid, request.url);
                        eprintln!("HTTP Analyzer: Storing request with key: {}", key);
                        eprintln!("HTTP Analyzer: Request details: {} {} (thread: {})", 
                            request.method, request.url, request.pid);
                        
                        self.pending_requests.insert(key, request.clone());
                        eprintln!("HTTP Analyzer: Total pending requests: {}", self.pending_requests.len());
                    } else {
                        eprintln!("HTTP Analyzer: Failed to parse HTTP request");
                    }
                } else if HttpParser::starts_with_http_response(message) {
                    if !self.process_response(message, event, pid, current_time, tx) {
                        return false;
                    }
                }
            }

            // Now update the buffer (get the buffer reference again)
            if let Some(current_buffer) = self.thread_buffers.get_mut(&pid) {
                let last_message_end = messages.last().map(|(_, end)| *end).unwrap_or(0);
                *current_buffer = current_buffer[last_message_end..].to_string();
                eprintln!("HTTP Analyzer: Buffer after processing: {} chars remaining", current_buffer.len());

                // If buffer is getting too large without finding complete HTTP data, clear it
                if current_buffer.len() > 65536 { // 64KB limit
                    eprintln!("HTTP Analyzer: Buffer too large ({}), clearing for thread: {}", 
                        current_buffer.len(), pid);
                    self.thread_buffers.remove(&pid);
                }
            }
        }
        true
    }
}

impl Default for HttpAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Analyzer for HttpAnalyzer {
    async fn process(&mut self, mut stream: EventStream) -> Result<EventStream, AnalyzerError> {
        eprintln!("HTTP Analyzer: Starting to process stream...");
        
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        
        // Process events with thread-based matching
        while let Some(event) = stream.next().await {
            // Debug: Print original event JSON
            eprintln!("HTTP Analyzer: Received event from source: {}", event.source);
            eprintln!("HTTP Analyzer: Event JSON: {}", 
                serde_json::to_string_pretty(&event.data).unwrap_or_default());
            
            // Only process SSL events
            if event.source != "ssl" {
                eprintln!("HTTP Analyzer: Forwarding non-SSL event");
                if tx.send(event).is_err() {
                    break;
                }
                continue;
            }

            // Extract HTTP data from SSL event
            let data_str = if let Some(data) = event.data.get("data") {
                match data {
                    Value::String(s) => s.clone(),
                    _ => {
                        eprintln!("HTTP Analyzer: SSL data is not a string, forwarding");
                        if tx.send(event).is_err() {
                            break;
                        }
                        continue;
                    }
                }
            } else {
                eprintln!("HTTP Analyzer: No data field in SSL event, forwarding");
                if tx.send(event).is_err() {
                    break;
                }
                continue;
            };

            let current_time = event.timestamp;
            let pid = event.data.get("pid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            
            // Clean up expired requests periodically
            self.cleanup_expired_requests(current_time);

            // Try to parse this SSL data directly as a complete HTTP message
            if HttpParser::starts_with_http_request(&data_str) {
                self.process_request(&data_str, &event, pid);
            } else if HttpParser::starts_with_http_response(&data_str) {
                if !self.process_response(&data_str, &event, pid, current_time, &tx) {
                    break;
                }
            } else {
                // If it's not a complete HTTP message, try the old buffering approach
                if !self.process_buffered_data(&data_str, &event, pid, current_time, &tx) {
                    break;
                }
            }
            
            // Always forward the original SSL event to the next analyzer
            eprintln!("HTTP Analyzer: Forwarding original SSL event");
            if tx.send(event).is_err() {
                break;
            }
        }

        eprintln!("HTTP Analyzer: Stream processing completed");
        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    fn name(&self) -> &str {
        &self.name
    }
} 