use super::{Analyzer, AnalyzerError};
use crate::framework::runners::EventStream;
use crate::framework::core::Event;
use async_trait::async_trait;
use futures::stream::StreamExt;
use futures;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// HTTP Request/Response analyzer that pairs HTTP requests with their responses
pub struct HttpAnalyzer {
    name: String,
    pending_requests: HashMap<String, PendingRequest>,
    max_wait_time_ms: u64,
    buffer: Vec<Event>,
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
    /// Create a new HttpAnalyzer
    pub fn new() -> Self {
        Self {
            name: "http".to_string(),
            pending_requests: HashMap::new(),
            max_wait_time_ms: 5000, // 5 seconds max wait for response
            buffer: Vec::new(),
        }
    }

    /// Create a new HttpAnalyzer with custom wait time
    pub fn new_with_wait_time(max_wait_time_ms: u64) -> Self {
        Self {
            name: "http".to_string(),
            pending_requests: HashMap::new(),
            max_wait_time_ms,
            buffer: Vec::new(),
        }
    }

    /// Check if event contains HTTP request data
    fn is_http_request(data: &str) -> bool {
        let methods = ["GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "PATCH", "TRACE"];
        methods.iter().any(|method| data.starts_with(method))
    }

    /// Check if event contains HTTP response data
    fn is_http_response(data: &str) -> bool {
        data.starts_with("HTTP/")
    }

    /// Parse HTTP request from SSL data
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

        // Get connection identifier from event data
        let pid = event.data.get("pid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let connection_id = format!("{}_{}", pid, 
            event.data.get("timestamp_ns").and_then(|v| v.as_u64()).unwrap_or(0));

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

    /// Parse HTTP response from SSL data
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

    /// Process a single event and potentially produce paired events
    fn process_event(&mut self, event: Event) -> Vec<Event> {
        let mut result_events = Vec::new();

        // Only process SSL events
        if event.source != "ssl" {
            result_events.push(event);
            return result_events;
        }

        // Extract HTTP data from SSL event
        let data_str = if let Some(data) = event.data.get("data") {
            match data {
                Value::String(s) => s.clone(),
                _ => {
                    result_events.push(event);
                    return result_events;
                }
            }
        } else {
            result_events.push(event);
            return result_events;
        };

        let current_time = event.timestamp;
        self.cleanup_expired_requests(current_time);

        if Self::is_http_request(&data_str) {
            // Parse and store HTTP request
            if let Some(request) = Self::parse_http_request(&data_str, &event) {
                let key = format!("{}_{}", request.pid, request.url);
                self.pending_requests.insert(key, request);
                println!("HTTP Analyzer: Stored request {} {}", 
                    self.pending_requests.len(), 
                    data_str.lines().next().unwrap_or(""));
            }
            // Don't forward individual request events
        } else if Self::is_http_response(&data_str) {
            // Parse HTTP response and try to match with pending request
            if let Some(response) = Self::parse_http_response(&data_str) {
                let pid = event.data.get("pid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                
                // Try to find matching request by PID and reasonable timing
                let mut matched_key = None;
                let mut best_match_score = f64::MAX;
                
                for (key, request) in &self.pending_requests {
                    if request.pid == pid {
                        let time_diff = current_time.saturating_sub(request.timestamp) as f64;
                        if time_diff < self.max_wait_time_ms as f64 && time_diff < best_match_score {
                            best_match_score = time_diff;
                            matched_key = Some(key.clone());
                        }
                    }
                }

                if let Some(key) = matched_key {
                    if let Some(request) = self.pending_requests.remove(&key) {
                        let pair_event = Self::create_request_response_pair(&request, &response);
                        result_events.push(pair_event);
                        println!("HTTP Analyzer: Created request/response pair: {} {} -> {} {}", 
                            request.method, request.url, response.status_code, response.status_text);
                    }
                } else {
                    println!("HTTP Analyzer: No matching request found for response: {} {}", 
                        response.status_code, response.status_text);
                }
            }
            // Don't forward individual response events
        } else {
            // Forward non-HTTP SSL events unchanged
            result_events.push(event);
        }

        result_events
    }
}

impl Default for HttpAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Analyzer for HttpAnalyzer {
    async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
        eprintln!("HTTP Analyzer: Starting to process stream...");
        
        let mut pending_requests = self.pending_requests.clone();
        let max_wait_time_ms = self.max_wait_time_ms;
        
        let processed_stream = stream.map(move |event| {
            eprintln!("HTTP Analyzer: Processing event from source: {}", event.source);
            
            // Only process SSL events
            if event.source != "ssl" {
                eprintln!("HTTP Analyzer: Forwarding non-SSL event");
                return vec![event];
            }

            // Extract HTTP data from SSL event
            let data_str = if let Some(data) = event.data.get("data") {
                match data {
                    Value::String(s) => {
                        eprintln!("HTTP Analyzer: Got SSL data: {}", 
                            if s.len() > 100 { &s[..100] } else { s });
                        s.clone()
                    },
                    _ => {
                        eprintln!("HTTP Analyzer: SSL data is not a string");
                        return vec![event];
                    }
                }
            } else {
                eprintln!("HTTP Analyzer: No data field in SSL event");
                return vec![event];
            };

            let current_time = event.timestamp;
            
            // Clean up expired requests
            let expired_keys: Vec<String> = pending_requests
                .iter()
                .filter(|(_, req)| current_time.saturating_sub(req.timestamp) > max_wait_time_ms)
                .map(|(key, _)| key.clone())
                .collect();

            for key in expired_keys {
                if let Some(expired_request) = pending_requests.remove(&key) {
                    eprintln!("HTTP Analyzer: Request expired after {}ms: {} {}", 
                        max_wait_time_ms, expired_request.method, expired_request.url);
                }
            }

            if Self::is_http_request(&data_str) {
                eprintln!("HTTP Analyzer: Found HTTP request");
                // Parse and store HTTP request
                if let Some(request) = Self::parse_http_request(&data_str, &event) {
                    let key = format!("{}_{}", request.pid, request.url);
                    pending_requests.insert(key, request.clone());
                    eprintln!("HTTP Analyzer: Stored request {} {}", 
                        pending_requests.len(), 
                        data_str.lines().next().unwrap_or(""));
                    
                    // Return the original event for now, but mark it as processed
                    let mut modified_event = event.clone();
                    modified_event.data.as_object_mut().unwrap().insert(
                        "http_processed".to_string(), 
                        json!("request_stored")
                    );
                    vec![modified_event]
                } else {
                    eprintln!("HTTP Analyzer: Failed to parse HTTP request");
                    vec![event]
                }
            } else if Self::is_http_response(&data_str) {
                eprintln!("HTTP Analyzer: Found HTTP response");
                // Parse HTTP response and try to match with pending request
                if let Some(response) = Self::parse_http_response(&data_str) {
                    let pid = event.data.get("pid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    
                    // Try to find matching request by PID and reasonable timing
                    let mut matched_key = None;
                    let mut best_match_score = f64::MAX;
                    
                    for (key, request) in &pending_requests {
                        if request.pid == pid {
                            let time_diff = current_time.saturating_sub(request.timestamp) as f64;
                            if time_diff < max_wait_time_ms as f64 && time_diff < best_match_score {
                                best_match_score = time_diff;
                                matched_key = Some(key.clone());
                            }
                        }
                    }

                    if let Some(key) = matched_key {
                        if let Some(request) = pending_requests.remove(&key) {
                            let pair_event = Self::create_request_response_pair(&request, &response);
                            eprintln!("HTTP Analyzer: Created request/response pair: {} {} -> {} {}", 
                                request.method, request.url, response.status_code, response.status_text);
                            vec![pair_event]
                        } else {
                            eprintln!("HTTP Analyzer: Failed to remove matched request");
                            vec![event]
                        }
                    } else {
                        eprintln!("HTTP Analyzer: No matching request found for response: {} {}", 
                            response.status_code, response.status_text);
                        // Still return the response event with metadata
                        let mut modified_event = event.clone();
                        modified_event.data.as_object_mut().unwrap().insert(
                            "http_processed".to_string(), 
                            json!("unmatched_response")
                        );
                        vec![modified_event]
                    }
                } else {
                    eprintln!("HTTP Analyzer: Failed to parse HTTP response");
                    vec![event]
                }
            } else {
                // Forward non-HTTP SSL events unchanged
                eprintln!("HTTP Analyzer: Forwarding non-HTTP SSL event (first 50 chars): {}", 
                    if data_str.len() > 50 { &data_str[..50] } else { &data_str });
                vec![event]
            }
        }).flat_map(futures::stream::iter);

        Ok(Box::pin(processed_stream))
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
        assert_eq!(analyzer.name(), "http");
    }
} 