use super::{Analyzer, AnalyzerError};
use super::event::HTTPEvent;
use crate::framework::runners::EventStream;
use crate::framework::core::Event;
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde_json::json;
use std::collections::HashMap;

/// HTTP Parser Analyzer that parses SSL traffic into HTTP requests/responses
pub struct HTTPParser {
    name: String,
    /// Flag to include raw data in parsed events (default: true)
    include_raw_data: bool,
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
    /// Create a new HTTPParser with default settings (raw data included)
    pub fn new() -> Self {
        HTTPParser {
            name: "HTTPParser".to_string(),
            include_raw_data: true,
        }
    }


    /// Disable raw data inclusion
    pub fn disable_raw_data(mut self) -> Self {
        self.include_raw_data = false;
        self
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

    /// Create HTTP event from parsed message
    fn create_http_event(
        tid: u64,
        parsed_message: HTTPMessage,
        original_event: &Event,
        include_raw_data: bool,
    ) -> Event {
        let message_type_str = match parsed_message.message_type {
            HTTPMessageType::Request => "request",
            HTTPMessageType::Response => "response",
        };

        // Determine content properties
        let content_length = parsed_message.headers.get("content-length")
            .and_then(|v| v.parse::<usize>().ok());
        let is_chunked = parsed_message.headers.get("transfer-encoding")
            .map(|v| v.to_lowercase().contains("chunked"))
            .unwrap_or(false);
        let has_body = parsed_message.body.is_some();

        // Calculate total size from parsed components
        let total_size = parsed_message.first_line.len() +
            parsed_message.headers.iter().map(|(k, v)| k.len() + v.len() + 4).sum::<usize>() + // +4 for ": \r\n"
            parsed_message.body.as_ref().map(|b| b.len()).unwrap_or(0) +
            4; // +4 for \r\n\r\n separator

        let mut http_event = HTTPEvent::new(
            tid,
            message_type_str.to_string(),
            parsed_message.first_line,
            parsed_message.method,
            parsed_message.path,
            parsed_message.protocol,
            parsed_message.status_code,
            parsed_message.status_text,
            parsed_message.headers,
            parsed_message.body,
            total_size,
            has_body,
            is_chunked,
            content_length,
            "ssl".to_string(),
            original_event.data.get("comm").unwrap_or(&json!("unknown")).as_str().unwrap_or("unknown").to_string(),
            original_event.data.get("pid").unwrap_or(&json!(0)).as_u64().unwrap_or(0),
            original_event.data.get("timestamp_ns").unwrap_or(&json!(0)).as_u64().unwrap_or(0),
        );

        // Include raw data if requested
        if include_raw_data {
            http_event = http_event.with_raw_data(parsed_message.raw_data);
        }

        http_event.to_event()
    }

    /// Handle SSL events (HTTP request/response data)
    fn handle_ssl_event(
        event: Event,
        include_raw_data: bool,
    ) -> Option<Event> {
        let ssl_data = &event.data;
        
        let data_str = match ssl_data.get("data").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => return Some(event),
        };

        // Only process if it's HTTP data AND can be parsed as a complete HTTP message
        if Self::is_http_data(data_str) {
            if let Some(parsed_message) = Self::parse_http_message(data_str) {
                let tid = ssl_data.get("tid").and_then(|v| v.as_u64()).unwrap_or(0);
                return Some(Self::create_http_event(tid, parsed_message, &event, include_raw_data));
            }
        }

        // If not parseable as HTTP, pass through original event
        Some(event)
    }
}

#[async_trait]
impl Analyzer for HTTPParser {
    async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
        let include_raw_data = self.include_raw_data;
        
        let processed_stream = stream.filter_map(move |event| {
            async move {
                // Only process SSL events
                if event.source == "ssl" {
                    Self::handle_ssl_event(event, include_raw_data)
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