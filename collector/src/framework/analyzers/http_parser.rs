use super::{Analyzer, AnalyzerError};
use crate::framework::runners::EventStream;
use crate::framework::core::Event;
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::io::Write;

/// HTTP Parser Analyzer that merges SSL traffic into complete HTTP requests/responses
pub struct HTTPParser {
    name: String,
    /// Store accumulating HTTP messages by TID
    http_buffers: Arc<Mutex<HashMap<u64, HTTPAccumulator>>>,
    /// Timeout for incomplete HTTP streams (in milliseconds)
    timeout_ms: u64,
    /// Enable debug output
    debug: bool,
}

/// Accumulator for HTTP messages belonging to the same TID
#[derive(Clone)]
struct HTTPAccumulator {
    tid: u64,
    accumulated_data: String,
    message_type: Option<HTTPMessageType>,
    is_complete: bool,
    last_update: u64,
    /// Track if message has body content
    has_body: bool,
    content_length: Option<usize>,
    is_chunked: bool,
    /// Store headers
    headers: HashMap<String, String>,
    /// Track first line (request line or status line)
    first_line: Option<String>,
    /// Track if this is an SSE response waiting for events
    is_sse_response: bool,
    /// Store accumulated SSE content
    sse_content: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum HTTPMessageType {
    Request,
    Response,
}

/// Parsed HTTP message
#[derive(Clone, Debug)]
pub struct HTTPMessage {
    pub message_type: HTTPMessageType,
    pub first_line: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub raw_data: String,
    // Request-specific fields
    pub method: Option<String>,
    pub path: Option<String>,
    pub protocol: Option<String>,
    // Response-specific fields
    pub status_code: Option<u16>,
    pub status_text: Option<String>,
}

impl HTTPParser {
    /// Create a new HTTPParser with default timeout (30 seconds)
    pub fn new() -> Self {
        Self::new_with_timeout(30_000)
    }

    /// Create a new HTTPParser with debug output enabled
    pub fn new_with_debug() -> Self {
        HTTPParser {
            name: "HTTPParser".to_string(),
            http_buffers: Arc::new(Mutex::new(HashMap::new())),
            timeout_ms: 30_000,
            debug: true,
        }
    }

    /// Create a new HTTPParser with custom timeout
    pub fn new_with_timeout(timeout_ms: u64) -> Self {
        HTTPParser {
            name: "HTTPParser".to_string(),
            http_buffers: Arc::new(Mutex::new(HashMap::new())),
            timeout_ms,
            debug: true,
        }
    }

    /// Debug print function - only prints if debug is enabled
    fn debug_print(&self, message: &str) {
        if self.debug {
            eprintln!("{}", message);
            std::io::stdout().flush().unwrap();
        }
    }

    /// Check if SSL data contains HTTP protocol data
    pub fn is_http_data(data: &str) -> bool {
        // Look for HTTP patterns
        let has_http_request = data.contains("HTTP/1.") && 
                              (data.contains("GET ") || data.contains("POST ") || 
                               data.contains("PUT ") || data.contains("DELETE ") ||
                               data.contains("HEAD ") || data.contains("OPTIONS ") ||
                               data.contains("PATCH "));
        
        let has_http_response = data.starts_with("HTTP/1.") || data.contains("\r\nHTTP/1.");
        
        // Look for common HTTP headers
        let has_http_headers = data.contains("Content-Type:") || 
                              data.contains("content-type:") ||
                              data.contains("Host:") ||
                              data.contains("host:") ||
                              data.contains("User-Agent:") ||
                              data.contains("user-agent:");

        has_http_request || has_http_response || has_http_headers
    }

    /// Parse HTTP message from accumulated data
    pub fn parse_http_message(data: &str) -> Option<HTTPMessage> {
        let lines: Vec<&str> = data.split("\r\n").collect();
        
        if lines.is_empty() {
            return None;
        }

        let first_line = lines[0];
        let mut headers = HashMap::new();
        let mut body_start = None;
        let mut message_type = HTTPMessageType::Request;
        let mut method = None;
        let mut path = None;
        let mut protocol = None;
        let mut status_code = None;
        let mut status_text = None;

        // Parse first line to determine message type
        if first_line.starts_with("HTTP/") {
            // Response
            message_type = HTTPMessageType::Response;
            let parts: Vec<&str> = first_line.splitn(3, ' ').collect();
            if parts.len() >= 2 {
                if let Ok(code) = parts[1].parse::<u16>() {
                    status_code = Some(code);
                }
                if parts.len() >= 3 {
                    status_text = Some(parts[2].to_string());
                }
                protocol = Some(parts[0].to_string());
            }
        } else {
            // Request
            let parts: Vec<&str> = first_line.splitn(3, ' ').collect();
            if parts.len() >= 3 {
                method = Some(parts[0].to_string());
                path = Some(parts[1].to_string());
                protocol = Some(parts[2].to_string());
            }
        }

        // Parse headers
        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.is_empty() {
                body_start = Some(i + 1);
                break;
            }
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_lowercase();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.insert(key, value);
            }
        }

        // Extract body if present
        let body = if let Some(start) = body_start {
            if start < lines.len() {
                let body_lines: Vec<&str> = lines[start..].to_vec();
                let body_content = body_lines.join("\r\n");
                if !body_content.trim().is_empty() {
                    Some(body_content)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        Some(HTTPMessage {
            message_type,
            first_line: first_line.to_string(),
            headers,
            body,
            raw_data: data.to_string(),
            method,
            path,
            protocol,
            status_code,
            status_text,
        })
    }

    /// Check if HTTP message is complete
    fn is_http_complete(accumulator: &HTTPAccumulator) -> bool {
        let data = &accumulator.accumulated_data;
        
        // Check if we have headers section complete (indicated by \r\n\r\n)
        if !data.contains("\r\n\r\n") {
            return false;
        }

        // If no body expected, message is complete
        if !accumulator.has_body {
            return true;
        }

        // Check content-length
        if let Some(expected_length) = accumulator.content_length {
            let header_end = data.find("\r\n\r\n").unwrap_or(0) + 4;
            let body_length = data.len().saturating_sub(header_end);
            return body_length >= expected_length;
        }

        // Check for chunked encoding completion
        if accumulator.is_chunked {
            return data.contains("0\r\n\r\n");
        }

        // For responses without content-length or chunked encoding,
        // consider complete if we have headers and some time has passed
        true
    }

    /// Accumulate HTTP data for a TID
    fn accumulate_http_data(accumulator: &mut HTTPAccumulator, data: &str, debug: bool) {
        accumulator.accumulated_data.push_str(data);
        
        // Parse headers if we haven't determined message type yet
        if accumulator.message_type.is_none() {
            let lines: Vec<&str> = accumulator.accumulated_data.split("\r\n").collect();
            if !lines.is_empty() {
                let first_line = lines[0];
                accumulator.first_line = Some(first_line.to_string());
                
                if first_line.starts_with("HTTP/") {
                    accumulator.message_type = Some(HTTPMessageType::Response);
                } else if first_line.contains("HTTP/") {
                    accumulator.message_type = Some(HTTPMessageType::Request);
                }

                if debug {
                    eprintln!("[DEBUG] HTTPParser: Detected message type: {:?}", accumulator.message_type);
                }
            }
        }

        // Parse headers to determine if body is expected
        if accumulator.accumulated_data.contains("\r\n\r\n") && !accumulator.has_body {
            let lines: Vec<&str> = accumulator.accumulated_data.split("\r\n").collect();
            
            for line in &lines[1..] {
                if line.is_empty() {
                    break;
                }
                if let Some(colon_pos) = line.find(':') {
                    let key = line[..colon_pos].trim().to_lowercase();
                    let value = line[colon_pos + 1..].trim().to_string();
                    accumulator.headers.insert(key.clone(), value.clone());
                    
                    match key.as_str() {
                        "content-length" => {
                            if let Ok(length) = value.parse::<usize>() {
                                accumulator.content_length = Some(length);
                                accumulator.has_body = length > 0;
                                if debug {
                                    eprintln!("[DEBUG] HTTPParser: Content-Length: {}", length);
                                }
                            }
                        }
                        "transfer-encoding" => {
                            if value.to_lowercase().contains("chunked") {
                                accumulator.is_chunked = true;
                                accumulator.has_body = true;
                                if debug {
                                    eprintln!("[DEBUG] HTTPParser: Chunked encoding detected");
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Create merged event from accumulated HTTP data
    fn create_http_event(
        tid: u64,
        accumulator: &HTTPAccumulator,
        original_event: &Event,
    ) -> Event {
        let mut parsed_message = Self::parse_http_message(&accumulator.accumulated_data)
            .unwrap_or_else(|| HTTPMessage {
                message_type: HTTPMessageType::Request,
                first_line: "UNKNOWN".to_string(),
                headers: HashMap::new(),
                body: None,
                raw_data: accumulator.accumulated_data.clone(),
                method: None,
                path: None,
                protocol: None,
                status_code: None,
                status_text: None,
            });

        // If this is an SSE response with content, merge the SSE content into the body
        if accumulator.is_sse_response && accumulator.sse_content.is_some() {
            parsed_message.body = accumulator.sse_content.clone();
        }

        let message_type_str = match parsed_message.message_type {
            HTTPMessageType::Request => "request",
            HTTPMessageType::Response => "response",
        };

        Event::new(
            "http_parser".to_string(),
            json!({
                "tid": tid,
                "message_type": message_type_str,
                "first_line": parsed_message.first_line,
                "method": parsed_message.method,
                "path": parsed_message.path,
                "protocol": parsed_message.protocol,
                "status_code": parsed_message.status_code,
                "status_text": parsed_message.status_text,
                "headers": parsed_message.headers,
                "body": parsed_message.body,
                "raw_data": parsed_message.raw_data,
                "total_size": accumulator.accumulated_data.len(),
                "has_body": accumulator.has_body,
                "is_chunked": accumulator.is_chunked,
                "content_length": accumulator.content_length,
                "is_sse_response": accumulator.is_sse_response,
                "sse_content_size": accumulator.sse_content.as_ref().map(|s| s.len()).unwrap_or(0),
                "original_source": "ssl",
                "comm": original_event.data.get("comm").unwrap_or(&json!("unknown")).as_str().unwrap_or("unknown"),
                "pid": original_event.data.get("pid").unwrap_or(&json!(0)),
                "timestamp_ns": original_event.data.get("timestamp_ns").unwrap_or(&json!(0)),
            })
        )
    }

    /// Handle SSL events (HTTP request/response data)
    async fn handle_ssl_event(
        event: Event,
        buffers: Arc<Mutex<HashMap<u64, HTTPAccumulator>>>,
        debug: bool,
        timeout_ms: u64,
    ) -> Option<Event> {
        let ssl_data = &event.data;
        
        // Skip handshake events
        if ssl_data.get("is_handshake").and_then(|v| v.as_bool()).unwrap_or(false) {
            if debug {
                eprintln!("[DEBUG] HTTPParser: Skipping handshake event");
            }
            return None;
        }

        let data_str = match ssl_data.get("data").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => return Some(event),
        };

        // Check if this is HTTP data
        if !Self::is_http_data(data_str) {
            return Some(event);
        }

        let tid = ssl_data.get("tid").and_then(|v| v.as_u64()).unwrap_or(0);
        let timestamp = event.timestamp;

        if debug {
            eprintln!("[DEBUG] HTTPParser: Processing HTTP data for TID {} at timestamp {}", 
                     tid, timestamp);
        }

        // Store/accumulate HTTP data for this TID
        let mut buffers_lock = buffers.lock().unwrap();
        
        let accumulator = buffers_lock.entry(tid).or_insert_with(|| HTTPAccumulator {
            tid,
            accumulated_data: String::new(),
            message_type: None,
            is_complete: false,
            last_update: timestamp,
            has_body: false,
            content_length: None,
            is_chunked: false,
            headers: HashMap::new(),
            first_line: None,
            is_sse_response: false,
            sse_content: None,
        });
        
        // Update last update time
        accumulator.last_update = timestamp;
        
        // Accumulate HTTP data
        Self::accumulate_http_data(accumulator, data_str, debug);
        
        // Check if this is an SSE response
        if accumulator.message_type == Some(HTTPMessageType::Response) {
            if let Some(content_type) = accumulator.headers.get("content-type") {
                if content_type.contains("text/event-stream") {
                    accumulator.is_sse_response = true;
                    if debug {
                        eprintln!("[DEBUG] HTTPParser: Detected SSE response for TID {}", tid);
                    }
                }
            }
        }
        
        // Check if message is complete (but for SSE responses, wait for SSE content)
        if Self::is_http_complete(accumulator) && (!accumulator.is_sse_response || accumulator.sse_content.is_some()) {
            if debug {
                eprintln!("[DEBUG] HTTPParser: HTTP message complete for TID {}", tid);
                eprintln!("[DEBUG] HTTPParser: Message type: {:?}", accumulator.message_type);
                eprintln!("[DEBUG] HTTPParser: Total size: {} bytes", accumulator.accumulated_data.len());
                eprintln!("[DEBUG] HTTPParser: Has body: {}", accumulator.has_body);
                eprintln!("[DEBUG] HTTPParser: Is SSE response: {}", accumulator.is_sse_response);
            }
            
            // Create merged HTTP event (including SSE content if available)
            let http_event = Self::create_http_event(tid, accumulator, &event);
            
            // Clear this accumulator
            buffers_lock.remove(&tid);
            drop(buffers_lock);
            
            Some(http_event)
        } else {
            // Check for timeout
            let time_since_start = timestamp - accumulator.last_update;
            if time_since_start > timeout_ms * 1_000_000 { // Convert ms to ns
                if debug {
                    eprintln!("[DEBUG] HTTPParser: Timeout reached for TID {}, creating partial message", tid);
                }
                
                // Create event with partial data
                let http_event = Self::create_http_event(tid, accumulator, &event);
                
                // Clear this accumulator
                buffers_lock.remove(&tid);
                drop(buffers_lock);
                
                Some(http_event)
            } else {
                // Message not complete yet, don't emit event
                if debug {
                    if accumulator.is_sse_response && accumulator.sse_content.is_none() {
                        eprintln!("[DEBUG] HTTPParser: SSE response for TID {} waiting for SSE content", tid);
                    } else {
                        eprintln!("[DEBUG] HTTPParser: HTTP message incomplete for TID {}, continuing accumulation", tid);
                    }
                }
                None
            }
        }
    }

    /// Handle SSE processor events (merged SSE content)
    async fn handle_sse_event(
        event: Event,
        buffers: Arc<Mutex<HashMap<u64, HTTPAccumulator>>>,
        debug: bool,
    ) -> Option<Event> {
        let sse_data = &event.data;
        let tid = sse_data.get("tid").and_then(|v| v.as_u64()).unwrap_or(0);
        
        if debug {
            eprintln!("[DEBUG] HTTPParser: Received SSE content for TID {}", tid);
        }

        let mut buffers_lock = buffers.lock().unwrap();
        
        // Look for a matching SSE response accumulator
        if let Some(accumulator) = buffers_lock.get_mut(&tid) {
            if accumulator.is_sse_response {
                // Extract merged content from SSE processor
                let sse_content = sse_data.get("merged_content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                accumulator.sse_content = Some(sse_content.to_string());
                
                if debug {
                    eprintln!("[DEBUG] HTTPParser: Added SSE content to TID {} ({} chars)", 
                             tid, sse_content.len());
                }
                
                // Check if we can now complete the HTTP response
                if Self::is_http_complete(accumulator) {
                    if debug {
                        eprintln!("[DEBUG] HTTPParser: Completing SSE HTTP response for TID {}", tid);
                    }
                    
                    // Create a dummy event for the HTTP response creation
                    let dummy_event = Event::new(
                        "ssl".to_string(),
                        json!({
                            "tid": tid,
                            "comm": sse_data.get("comm").unwrap_or(&json!("unknown")),
                            "pid": sse_data.get("pid").unwrap_or(&json!(0)),
                            "timestamp_ns": sse_data.get("timestamp_ns").unwrap_or(&json!(0)),
                        })
                    );
                    
                    let http_event = Self::create_http_event(tid, accumulator, &dummy_event);
                    
                    // Clear this accumulator
                    buffers_lock.remove(&tid);
                    drop(buffers_lock);
                    
                    return Some(http_event);
                }
            }
        }
        
        // Don't pass through SSE processor events - we've consumed them
        None
    }
}

#[async_trait]
impl Analyzer for HTTPParser {
    async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
        let http_buffers = Arc::clone(&self.http_buffers);

        self.debug_print("[DEBUG] HTTPParser: Starting HTTP message processing");
        
        let debug = self.debug;
        let timeout_ms = self.timeout_ms;
        
        let processed_stream = stream.filter_map(move |event| {
            let buffers = Arc::clone(&http_buffers);
            
            async move {
                // Handle both SSL and SSE processor events
                if event.source == "ssl" {
                    Self::handle_ssl_event(event, buffers, debug, timeout_ms).await
                } else if event.source == "sse_processor" {
                    Self::handle_sse_event(event, buffers, debug).await
                } else {
                    Some(event) // Pass through other events
                }
            }
        });

        Ok(Box::pin(processed_stream))
    }

    fn name(&self) -> &str {
        &self.name
    }
}