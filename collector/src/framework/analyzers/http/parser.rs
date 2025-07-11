use super::types::{PendingRequest, HttpResponse};
use crate::framework::core::Event;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// HTTP parsing utilities
pub struct HttpParser;

impl HttpParser {
    /// Check if data starts with an HTTP request
    pub fn starts_with_http_request(data: &str) -> bool {
        let methods = ["GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "PATCH", "TRACE"];
        methods.iter().any(|&method| {
            data.trim_start().starts_with(&format!("{} ", method))
        })
    }

    /// Check if data starts with an HTTP response
    pub fn starts_with_http_response(data: &str) -> bool {
        data.trim_start().starts_with("HTTP/")
    }

    /// Find the next UTF-8 character boundary from the given position
    fn next_char_boundary(s: &str, mut pos: usize) -> usize {
        if pos >= s.len() {
            return s.len();
        }
        
        // Move forward to find the next valid character boundary
        while pos < s.len() && !s.is_char_boundary(pos) {
            pos += 1;
        }
        pos
    }

    /// Extract complete HTTP messages from a buffer
    pub fn extract_http_messages(buffer: &str) -> Vec<(String, usize)> {
        let mut messages = Vec::new();
        let mut current_pos = 0;
        let _buffer_bytes = buffer.as_bytes();
        
        while current_pos < buffer.len() {
            // Ensure current_pos is at a valid UTF-8 character boundary
            current_pos = Self::next_char_boundary(buffer, current_pos);
            if current_pos >= buffer.len() {
                break;
            }
            
            let remaining = &buffer[current_pos..];
            
            // Skip non-HTTP data
            if !Self::starts_with_http_request(remaining) && !Self::starts_with_http_response(remaining) {
                // Move to the next character boundary
                current_pos = Self::next_char_boundary(buffer, current_pos + 1);
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
                current_pos = Self::next_char_boundary(buffer, current_pos + message_end);
            } else {
                // Message not complete yet
                break;
            }
        }
        
        messages
    }

    /// Parse HTTP request from complete HTTP data
    pub fn parse_http_request(data: &str, event: &Event) -> Option<PendingRequest> {
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
    pub fn parse_http_response(data: &str, event: &Event) -> Option<HttpResponse> {
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
} 