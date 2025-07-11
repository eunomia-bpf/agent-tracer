use crate::framework::core::Event;
use serde_json::Value;
use std::collections::HashMap;

/// Represents a pending HTTP request waiting for a response
#[derive(Clone, Debug)]
pub struct PendingRequest {
    pub event: Event,
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub timestamp: u64,
    pub pid: u32,
    pub original_json: Value,
}

/// Represents a parsed HTTP response
#[derive(Debug)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub timestamp: u64,
    pub pid: u32,
    pub original_json: Value,
} 