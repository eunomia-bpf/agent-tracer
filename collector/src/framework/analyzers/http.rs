use super::{Analyzer, AnalyzerError};
use crate::framework::runners::EventStream;
use crate::framework::core::Event;
use async_trait::async_trait;
use futures::stream::StreamExt;
use futures;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_stream::wrappers::UnboundedReceiverStream;

/// HTTP Request/Response analyzer that pairs HTTP requests with their responses by thread/PID
pub struct HttpAnalyzer {
    name: String,
    pending_requests: HashMap<String, PendingRequest>, // Key: "pid_url"
    thread_buffers: HashMap<u32, String>, // Buffer partial HTTP data per thread/PID
    max_wait_time_ms: u64,
}

#[derive(Debug, Clone)]
struct PendingRequest {
    event: Event,
    method: String,
    url: String,
    headers: HashMap<String, String>,
    body: Option<String>,
    timestamp: u64,
    pid: u32,
    original_json: Value,
}

#[derive(Debug, Clone)]
struct HttpResponse {
    status_code: u16,
    status_text: String,
    headers: HashMap<String, String>,
    body: Option<String>,
    timestamp: u64,
    pid: u32,
    original_json: Value,
}

impl HttpAnalyzer {
    /// Create a new HTTP analyzer with default settings
    pub fn new() -> Self {
        Self {
            name: "HttpAnalyzer".to_string(),
            pending_requests: HashMap::new(),
            thread_buffers: HashMap::new(),
            max_wait_time_ms: 30000, // 30 seconds
        }
    }

    /// Create a new HTTP analyzer with custom wait time
    pub fn new_with_wait_time(max_wait_time_ms: u64) -> Self {
        Self {
            name: "HttpAnalyzer".to_string(),
            pending_requests: HashMap::new(),
            thread_buffers: HashMap::new(),
            max_wait_time_ms,
        }
    }

    /// Check if data starts with HTTP request
    fn starts_with_http_request(data: &str) -> bool {
        let first_line = data.lines().next().unwrap_or("");
        first_line.starts_with("GET ") || first_line.starts_with("POST ") ||
        first_line.starts_with("PUT ") || first_line.starts_with("DELETE ") ||
        first_line.starts_with("HEAD ") || first_line.starts_with("PATCH ") ||
        first_line.starts_with("OPTIONS ") || first_line.starts_with("CONNECT ")
    }

    /// Check if data starts with HTTP response  
    fn starts_with_http_response(data: &str) -> bool {
        let first_line = data.lines().next().unwrap_or("");
        first_line.starts_with("HTTP/1.") || first_line.starts_with("HTTP/2")
    }

    /// Extract complete HTTP messages from buffer
    fn extract_http_messages(buffer: &str) -> Vec<(String, usize)> {
        let mut messages = Vec::new();
        let mut current_pos = 0;
        let buffer_bytes = buffer.as_bytes();
        
        while current_pos < buffer.len() {
            let remaining = &buffer[current_pos..];
            
            // Skip non-HTTP data
            if !Self::starts_with_http_request(remaining) && !Self::starts_with_http_response(remaining) {
                current_pos += 1;
                continue;
            }
            
            // Find end of headers (double newline)
            let header_end = if let Some(pos) = remaining.find("\r\n\r\n") {
                pos + 4
            } else if let Some(pos) = remaining.find("\n\n") {
                pos + 2
            } else {
                // Headers not complete yet
                break;
            };
            
            let headers_part = &remaining[..header_end];
            let mut content_length = 0;
            let mut is_chunked = false;
            
            // Parse headers to get content length
            for line in headers_part.lines().skip(1) {
                if line.trim().is_empty() {
                    break;
                }
                if let Some(colon_pos) = line.find(':') {
                    let key = line[..colon_pos].trim().to_lowercase();
                    let value = line[colon_pos + 1..].trim().to_lowercase();
                    
                    if key == "content-length" {
                        content_length = value.parse::<usize>().unwrap_or(0);
                    } else if key == "transfer-encoding" && value.contains("chunked") {
                        is_chunked = true;
                    }
                }
            }
            
            let message_end = if is_chunked {
                // For chunked encoding, look for "0\r\n\r\n" or "0\n\n"
                if let Some(pos) = remaining.find("\r\n0\r\n\r\n") {
                    pos + 7
                } else if let Some(pos) = remaining.find("\n0\n\n") {
                    pos + 4
                } else {
                    // Chunked message not complete
                    break;
                }
            } else {
                // Use content-length
                header_end + content_length
            };
            
            if message_end <= remaining.len() {
                let message = remaining[..message_end].to_string();
                messages.push((message, current_pos + message_end));
                current_pos += message_end;
            } else {
                // Message not complete yet
                break;
            }
        }
        
        messages
    }

    /// Parse HTTP request from complete HTTP data
    fn parse_http_request(data: &str, event: &Event) -> Option<PendingRequest> {
        let lines: Vec<&str> = data.lines().collect();
        if lines.is_empty() {
            return None;
        }

        // Parse request line: "GET /path HTTP/1.1"
        let request_line_parts: Vec<&str> = lines[0].split_whitespace().collect();
        if request_line_parts.len() < 3 {
            return None;
        }

        let method = request_line_parts[0].to_string();
        let url = request_line_parts[1].to_string();

        // Parse headers
        let mut headers = HashMap::new();
        let mut header_end = 1;
        
        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.trim().is_empty() {
                header_end = i + 1;
                break;
            }
            
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_lowercase();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.insert(key, value);
            }
        }

        // Extract body if present
        let body = if header_end < lines.len() {
            let body_lines: Vec<&str> = lines[header_end..].iter().cloned().collect();
            let body_text = body_lines.join("\n").trim().to_string();
            if body_text.is_empty() { None } else { Some(body_text) }
        } else {
            None
        };

        let pid = event.data.get("pid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        Some(PendingRequest {
            event: event.clone(),
            method,
            url,
            headers,
            body,
            timestamp: event.timestamp,
            pid,
            original_json: event.data.clone(),
        })
    }

    /// Parse HTTP response from complete HTTP data
    fn parse_http_response(data: &str, event: &Event) -> Option<HttpResponse> {
        let lines: Vec<&str> = data.lines().collect();
        if lines.is_empty() {
            return None;
        }

        // Parse status line: "HTTP/1.1 200 OK"
        let status_line_parts: Vec<&str> = lines[0].split_whitespace().collect();
        if status_line_parts.len() < 2 {
            return None;
        }

        let status_code = status_line_parts[1].parse::<u16>().ok()?;
        let status_text = if status_line_parts.len() > 2 {
            status_line_parts[2..].join(" ")
        } else {
            String::new()
        };

        // Parse headers
        let mut headers = HashMap::new();
        let mut header_end = 1;
        
        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.trim().is_empty() {
                header_end = i + 1;
                break;
            }
            
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_lowercase();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.insert(key, value);
            }
        }

        // Extract body if present
        let body = if header_end < lines.len() {
            let body_lines: Vec<&str> = lines[header_end..].iter().cloned().collect();
            let body_text = body_lines.join("\n").trim().to_string();
            if body_text.is_empty() { None } else { Some(body_text) }
        } else {
            None
        };

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let pid = event.data.get("pid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        Some(HttpResponse {
            status_code,
            status_text,
            headers,
            body,
            timestamp,
            pid,
            original_json: event.data.clone(),
        })
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

            // Buffer the SSL data for this thread/PID
            let current_buffer = self.thread_buffers.entry(pid).or_insert_with(String::new);
            current_buffer.push_str(&data_str);

            eprintln!("HTTP Analyzer: Buffered {} chars for thread {}, data: {}", 
                current_buffer.len(), pid,
                if data_str.len() > 100 { &data_str[..100] } else { &data_str });

            // Extract complete HTTP messages from buffer
            let messages = Self::extract_http_messages(current_buffer);
            
            if !messages.is_empty() {
                eprintln!("HTTP Analyzer: Found {} complete HTTP messages for thread {}", 
                    messages.len(), pid);
                
                // Process each complete message
                for (message, _) in &messages {
                    eprintln!("HTTP Analyzer: Processing message: {}", 
                        if message.len() > 200 { &message[..200] } else { message });
                    
                    if Self::starts_with_http_request(message) {
                        eprintln!("HTTP Analyzer: Found complete HTTP request for thread {}", pid);
                        if let Some(request) = Self::parse_http_request(message, &event) {
                            let key = format!("{}_{}", request.pid, request.url);
                            eprintln!("HTTP Analyzer: Storing request with key: {}", key);
                            eprintln!("HTTP Analyzer: Request details: {} {} (thread: {})", 
                                request.method, request.url, request.pid);
                            
                            self.pending_requests.insert(key, request.clone());
                            eprintln!("HTTP Analyzer: Total pending requests: {}", self.pending_requests.len());
                        } else {
                            eprintln!("HTTP Analyzer: Failed to parse HTTP request");
                        }
                    } else if Self::starts_with_http_response(message) {
                        eprintln!("HTTP Analyzer: Found complete HTTP response for thread {}", pid);
                        if let Some(response) = Self::parse_http_response(message, &event) {
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
                                    eprintln!("HTTP Analyzer: Pair JSON: {}", 
                                        serde_json::to_string_pretty(&pair_event.data).unwrap_or_default());
                                    
                                    if tx.send(pair_event).is_err() {
                                        break;
                                    }
                                }
                            } else {
                                eprintln!("HTTP Analyzer: No matching request found for response: {} {} (thread: {})", 
                                    response.status_code, response.status_text, pid);
                                eprintln!("HTTP Analyzer: Current pending requests by thread:");
                                for (key, req) in &self.pending_requests {
                                    eprintln!("HTTP Analyzer:   - {}: {} {} (thread: {})", 
                                        key, req.method, req.url, req.pid);
                                }
                            }
                        } else {
                            eprintln!("HTTP Analyzer: Failed to parse HTTP response");
                        }
                    }
                }
                
                // Remove processed messages from buffer
                let last_end = messages.last().unwrap().1;
                *current_buffer = current_buffer[last_end..].to_string();
                eprintln!("HTTP Analyzer: Buffer after processing: {} chars remaining", current_buffer.len());
            }
            
            // If buffer is getting too large without finding complete HTTP data, clear it
            if current_buffer.len() > 65536 { // 64KB limit
                eprintln!("HTTP Analyzer: Buffer too large ({}), clearing for thread: {}", 
                    current_buffer.len(), pid);
                self.thread_buffers.remove(&pid);
            }
        }

        eprintln!("HTTP Analyzer: Stream processing completed");
        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use serde_json::json;

    #[tokio::test]
    async fn test_http_request_parsing() {
        let http_data = "GET /api/users HTTP/1.1\r\nHost: example.com\r\nUser-Agent: curl/7.68.0\r\n\r\n";
        let event = Event::new("ssl".to_string(), json!({
            "data": http_data,
            "pid": 1234,
            "timestamp_ns": 1234567890
        }));

        let request = HttpAnalyzer::parse_http_request(http_data, &event).unwrap();
        assert_eq!(request.method, "GET");
        assert_eq!(request.url, "/api/users");
        assert_eq!(request.headers.get("host"), Some(&"example.com".to_string()));
        assert_eq!(request.pid, 1234);
    }

    #[tokio::test]
    async fn test_http_response_parsing() {
        let http_data = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 13\r\n\r\n{\"status\":\"ok\"}";
        
        let response = HttpAnalyzer::parse_http_response(http_data, &Event::new("ssl".to_string(), json!({
            "data": http_data,
            "pid": 1234,
            "timestamp_ns": 1234567890
        }))).unwrap();
        assert_eq!(response.status_code, 200);
        assert_eq!(response.status_text, "OK");
        assert_eq!(response.headers.get("content-type"), Some(&"application/json".to_string()));
        assert_eq!(response.body, Some("{\"status\":\"ok\"}".to_string()));
        assert_eq!(response.pid, 1234);
    }

    #[tokio::test]
    async fn test_request_response_pairing() {
        let mut analyzer = HttpAnalyzer::new();
        
        let request_event = Event::new("ssl".to_string(), json!({
            "data": "GET /test HTTP/1.1\r\nHost: example.com\r\n\r\n",
            "pid": 1234,
            "timestamp_ns": 1000000000
        }));

        let response_event = Event::new_with_id_and_timestamp(
            "resp1".to_string(),
            1001, // 1ms later
            "ssl".to_string(), 
            json!({
                "data": "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello World",
                "pid": 1234,
                "timestamp_ns": 1001000000
            })
        );

        let events = vec![request_event, response_event];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = analyzer.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        // Should have one paired event
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].source, "http_analyzer");
        assert_eq!(collected[0].data["type"], "http_request_response_pair");
        assert_eq!(collected[0].data["request"]["method"], "GET");
        assert_eq!(collected[0].data["response"]["status_code"], 200);
        assert_eq!(collected[0].data["thread_id"], 1234);
    }

    #[tokio::test]
    async fn test_analyzer_name() {
        let analyzer = HttpAnalyzer::new();
        assert_eq!(analyzer.name(), "HttpAnalyzer");
    }
} 