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

/// HTTP Request/Response analyzer that pairs HTTP requests with their responses
pub struct HttpAnalyzer {
    name: String,
    pending_requests: HashMap<String, PendingRequest>,
    connection_buffers: HashMap<String, String>, // Buffer partial HTTP data per connection
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
    connection_id: String,
}

#[derive(Debug, Clone)]
struct HttpResponse {
    status_code: u16,
    status_text: String,
    headers: HashMap<String, String>,
    body: Option<String>,
    timestamp: u64,
}

impl HttpAnalyzer {
    /// Create a new HTTP analyzer with default settings
    pub fn new() -> Self {
        Self {
            name: "HttpAnalyzer".to_string(),
            pending_requests: HashMap::new(),
            connection_buffers: HashMap::new(),
            max_wait_time_ms: 30000, // 30 seconds to allow for slow responses
        }
    }

    /// Create a new HTTP analyzer with custom wait time
    pub fn new_with_wait_time(max_wait_time_ms: u64) -> Self {
        Self {
            name: "HttpAnalyzer".to_string(),
            pending_requests: HashMap::new(),
            connection_buffers: HashMap::new(),
            max_wait_time_ms,
        }
    }

    /// Check if buffered data contains a complete HTTP request
    fn is_complete_http_request(data: &str) -> bool {
        let lines: Vec<&str> = data.lines().collect();
        if lines.is_empty() {
            return false;
        }
        
        // Check if first line is a valid HTTP request
        let first_line = lines[0];
        let is_request_line = first_line.starts_with("GET ") || first_line.starts_with("POST ") ||
            first_line.starts_with("PUT ") || first_line.starts_with("DELETE ") ||
            first_line.starts_with("HEAD ") || first_line.starts_with("PATCH ") ||
            first_line.starts_with("OPTIONS ") || first_line.starts_with("CONNECT ");
            
        if !is_request_line {
            return false;
        }
        
        // Find end of headers (empty line)
        let mut header_end = None;
        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.trim().is_empty() {
                header_end = Some(i);
                break;
            }
        }
        
        if let Some(header_end_idx) = header_end {
            // Check if we have Content-Length header
            let mut content_length = 0;
            for line in &lines[1..header_end_idx] {
                if let Some(colon_pos) = line.find(':') {
                    let key = line[..colon_pos].trim().to_lowercase();
                    if key == "content-length" {
                        if let Ok(len) = line[colon_pos + 1..].trim().parse::<usize>() {
                            content_length = len;
                            break;
                        }
                    }
                }
            }
            
            // If no body expected, request is complete
            if content_length == 0 {
                return true;
            }
            
            // Check if we have the complete body
            let body_start = header_end_idx + 1;
            if body_start < lines.len() {
                let body = lines[body_start..].join("\n");
                return body.len() >= content_length;
            }
        }
        
        false
    }

    /// Check if buffered data contains a complete HTTP response
    fn is_complete_http_response(data: &str) -> bool {
        let lines: Vec<&str> = data.lines().collect();
        if lines.is_empty() {
            return false;
        }
        
        // Check if first line is a valid HTTP response
        let first_line = lines[0];
        let is_response_line = first_line.starts_with("HTTP/1.") || first_line.starts_with("HTTP/2");
        
        if !is_response_line {
            return false;
        }
        
        // Find end of headers (empty line)
        let mut header_end = None;
        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.trim().is_empty() {
                header_end = Some(i);
                break;
            }
        }
        
        if let Some(header_end_idx) = header_end {
            // Check if we have Content-Length header
            let mut content_length = 0;
            let mut is_chunked = false;
            
            for line in &lines[1..header_end_idx] {
                if let Some(colon_pos) = line.find(':') {
                    let key = line[..colon_pos].trim().to_lowercase();
                    let value = line[colon_pos + 1..].trim().to_lowercase();
                    
                    if key == "content-length" {
                        if let Ok(len) = value.parse::<usize>() {
                            content_length = len;
                        }
                    } else if key == "transfer-encoding" && value.contains("chunked") {
                        is_chunked = true;
                    }
                }
            }
            
            // If no body expected, response is complete
            if content_length == 0 && !is_chunked {
                return true;
            }
            
            // For chunked encoding, look for "0\r\n\r\n" at the end
            if is_chunked {
                return data.ends_with("\r\n0\r\n\r\n") || data.ends_with("\n0\n\n");
            }
            
            // Check if we have the complete body based on Content-Length
            let body_start = header_end_idx + 1;
            if body_start < lines.len() {
                let body = lines[body_start..].join("\n");
                return body.len() >= content_length;
            }
        }
        
        false
    }

    /// Create connection identifier from event
    fn create_connection_id(event: &Event) -> String {
        let pid = event.data.get("pid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        
        // Try to use socket information for better connection tracking
        if let (Some(src_ip), Some(src_port), Some(dst_ip), Some(dst_port)) = (
            event.data.get("src_ip").and_then(|v| v.as_str()),
            event.data.get("src_port").and_then(|v| v.as_u64()),
            event.data.get("dst_ip").and_then(|v| v.as_str()),
            event.data.get("dst_port").and_then(|v| v.as_u64()),
        ) {
            format!("{}:{}->{}:{}", src_ip, src_port, dst_ip, dst_port)
        } else {
            // Fallback to PID-based connection ID
            format!("pid_{}", pid)
        }
    }

    /// Parse HTTP request from complete HTTP data
    fn parse_http_request(data: &str, event: &Event) -> Option<PendingRequest> {
        let lines: Vec<&str> = data.split('\n').collect();
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
        let connection_id = Self::create_connection_id(event);

        Some(PendingRequest {
            event: event.clone(),
            method,
            url,
            headers,
            body,
            timestamp: event.timestamp,
            pid,
            connection_id,
        })
    }

    /// Parse HTTP response from complete HTTP data
    fn parse_http_response(data: &str) -> Option<HttpResponse> {
        let lines: Vec<&str> = data.split('\n').collect();
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

        Some(HttpResponse {
            status_code,
            status_text,
            headers,
            body,
            timestamp,
        })
    }

    /// Create a request/response pair event
    fn create_request_response_pair(
        request: &PendingRequest,
        response: &HttpResponse,
    ) -> Event {
        let pair_data = json!({
            "type": "http_request_response_pair",
            "request": {
                "method": request.method,
                "url": request.url,
                "headers": request.headers,
                "body": request.body,
                "timestamp": request.timestamp,
                "pid": request.pid,
                "original_event_id": request.event.id
            },
            "response": {
                "status_code": response.status_code,
                "status_text": response.status_text,
                "headers": response.headers,
                "body": response.body,
                "timestamp": response.timestamp
            },
            "duration_ms": response.timestamp.saturating_sub(request.timestamp),
            "connection_id": request.connection_id
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
                eprintln!("HTTP Analyzer: Request expired after {}ms: {} {}", 
                    self.max_wait_time_ms, expired_request.method, expired_request.url);
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
        
        // Move the entire processing logic here with access to mutable self
        while let Some(event) = stream.next().await {
            // Only process SSL events
            if event.source != "ssl" {
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
                        if tx.send(event).is_err() {
                            break;
                        }
                        continue;
                    }
                }
            } else {
                if tx.send(event).is_err() {
                    break;
                }
                continue;
            };

            let current_time = event.timestamp;
            let connection_id = Self::create_connection_id(&event);
            
            // Clean up expired requests periodically
            self.cleanup_expired_requests(current_time);

            // Buffer the SSL data for this connection
            let current_buffer = self.connection_buffers.entry(connection_id.clone()).or_insert_with(String::new);
            current_buffer.push_str(&data_str);

            eprintln!("HTTP Analyzer: Buffered data for {}: {} chars", 
                connection_id, current_buffer.len());

            // Check if we have a complete HTTP request
            if Self::is_complete_http_request(current_buffer) {
                eprintln!("HTTP Analyzer: Found complete HTTP request");
                if let Some(request) = Self::parse_http_request(current_buffer, &event) {
                    let key = format!("{}_{}", request.connection_id, request.url);
                    self.pending_requests.insert(key, request.clone());
                    eprintln!("HTTP Analyzer: ✅ Stored request {} {} (total pending: {})", 
                        request.method, request.url, self.pending_requests.len());
                }
                // Clear the buffer after processing
                self.connection_buffers.remove(&connection_id);
            }
            // Check if we have a complete HTTP response
            else if Self::is_complete_http_response(current_buffer) {
                eprintln!("HTTP Analyzer: Found complete HTTP response");
                if let Some(response) = Self::parse_http_response(current_buffer) {
                    // Try to find matching request
                    let mut matched_key = None;
                    let mut best_match_score = f64::MAX;
                    
                    eprintln!("HTTP Analyzer: Looking for request match with connection_id: {}", connection_id);
                    for (key, request) in &self.pending_requests {
                        eprintln!("HTTP Analyzer: Checking pending request: {} (connection: {})", 
                            key, request.connection_id);
                        
                        if request.connection_id == connection_id {
                            let time_diff = current_time.saturating_sub(request.timestamp) as f64;
                            eprintln!("HTTP Analyzer: Connection match found, time_diff: {}ms", time_diff);
                            if time_diff < self.max_wait_time_ms as f64 && time_diff < best_match_score {
                                best_match_score = time_diff;
                                matched_key = Some(key.clone());
                            }
                        }
                    }

                    if let Some(key) = matched_key {
                        if let Some(request) = self.pending_requests.remove(&key) {
                            let pair_event = Self::create_request_response_pair(&request, &response);
                            eprintln!("HTTP Analyzer: ✅ Created request/response pair: {} {} -> {} {} ({}ms)", 
                                request.method, request.url, response.status_code, response.status_text, best_match_score);
                            if tx.send(pair_event).is_err() {
                                break;
                            }
                        }
                    } else {
                        eprintln!("HTTP Analyzer: ❌ No matching request found for response: {} {} (connection: {})", 
                            response.status_code, response.status_text, connection_id);
                    }
                }
                // Clear the buffer after processing
                self.connection_buffers.remove(&connection_id);
            }
            // If buffer is getting too large without finding complete HTTP data, clear it
            else if current_buffer.len() > 65536 { // 64KB limit
                eprintln!("HTTP Analyzer: Buffer too large, clearing for connection: {}", connection_id);
                self.connection_buffers.remove(&connection_id);
            }
            // For small fragments, just log and continue buffering
            else {
                eprintln!("HTTP Analyzer: Buffering fragment ({}): {}", 
                    current_buffer.len(),
                    if data_str.len() > 50 { &data_str[..50] } else { &data_str });
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
        
        let response = HttpAnalyzer::parse_http_response(http_data).unwrap();
        assert_eq!(response.status_code, 200);
        assert_eq!(response.status_text, "OK");
        assert_eq!(response.headers.get("content-type"), Some(&"application/json".to_string()));
        assert_eq!(response.body, Some("{\"status\":\"ok\"}".to_string()));
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
    }

    #[tokio::test]
    async fn test_analyzer_name() {
        let analyzer = HttpAnalyzer::new();
        assert_eq!(analyzer.name(), "HttpAnalyzer");
    }
} 