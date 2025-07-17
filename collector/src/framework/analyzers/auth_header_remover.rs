use super::{Analyzer, AnalyzerError};
use crate::framework::runners::EventStream;
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde_json::Value;

/// Authorization Header Remover Analyzer that removes authorization headers from HTTP events
/// This analyzer should be used after HTTPFilter to clean sensitive data from HTTP traffic
#[derive(Debug)]
pub struct AuthHeaderRemover {
    name: String,
    /// List of authorization header names to remove (case-insensitive)
    auth_headers: Vec<String>,
    /// Whether to log when headers are removed (for debugging)
    debug: bool,
}

impl AuthHeaderRemover {
    /// Create a new AuthHeaderRemover with default authorization headers
    pub fn new() -> Self {
        Self {
            name: "AuthHeaderRemover".to_string(),
            auth_headers: vec![
                "authorization".to_string(),
                "x-api-key".to_string(),
                "x-auth-token".to_string(),
                "bearer".to_string(),
                "token".to_string(),
                "x-access-token".to_string(),
                "x-session-token".to_string(),
                "cookie".to_string(),
                "set-cookie".to_string(),
            ],
            debug: false,
        }
    }

    /// Create a new AuthHeaderRemover with custom header names
    pub fn with_headers(headers: Vec<String>) -> Self {
        Self {
            name: "AuthHeaderRemover".to_string(),
            auth_headers: headers.into_iter().map(|h| h.to_lowercase()).collect(),
            debug: false,
        }
    }

    /// Enable debug logging for header removal
    pub fn with_debug(mut self) -> Self {
        self.debug = true;
        self
    }

    /// Remove authorization headers from HTTP event data
    fn remove_auth_headers(&self, mut event_data: Value) -> Value {
        // Only process HTTP parser events
        if event_data.get("message_type").and_then(|v| v.as_str()).is_none() {
            return event_data;
        }

        let mut headers_removed = Vec::new();

        // Process headers if they exist
        if let Some(headers_obj) = event_data.get_mut("headers").and_then(|v| v.as_object_mut()) {
            // Collect keys to remove (case-insensitive matching)
            let keys_to_remove: Vec<String> = headers_obj
                .keys()
                .filter(|key| {
                    self.auth_headers.iter().any(|auth_header| {
                        key.to_lowercase() == auth_header.to_lowercase()
                    })
                })
                .cloned()
                .collect();

            // Remove the matching headers
            for key in keys_to_remove {
                if headers_obj.remove(&key).is_some() {
                    headers_removed.push(key);
                }
            }
        }

        // Log removed headers if debug is enabled
        if self.debug && !headers_removed.is_empty() {
            eprintln!("[AuthHeaderRemover DEBUG] Removed headers: {:?}", headers_removed);
        }

        event_data
    }
}

impl Default for AuthHeaderRemover {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Analyzer for AuthHeaderRemover {
    async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
        let auth_headers = self.auth_headers.clone();
        let debug = self.debug;

        let processed_stream = stream.map(move |mut event| {
            // Only process events from http_parser
            if event.source == "http_parser" {
                event.data = AuthHeaderRemover {
                    name: "AuthHeaderRemover".to_string(),
                    auth_headers: auth_headers.clone(),
                    debug,
                }.remove_auth_headers(event.data);
            }
            event
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
    async fn test_auth_header_removal() {
        let mut analyzer = AuthHeaderRemover::new();
        
        let event_data = json!({
            "message_type": "request",
            "method": "GET",
            "path": "/api/test",
            "headers": {
                "authorization": "Bearer token123",
                "content-type": "application/json",
                "x-api-key": "secret-key",
                "user-agent": "test-client"
            }
        });
        
        let test_event = Event::new("http_parser".to_string(), 1234, "http_parser".to_string(), event_data);
        let events = vec![test_event];
        
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = analyzer.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        assert_eq!(collected.len(), 1);
        let headers = collected[0].data.get("headers").and_then(|v| v.as_object()).unwrap();
        
        // Authorization headers should be removed
        assert!(!headers.contains_key("authorization"));
        assert!(!headers.contains_key("x-api-key"));
        
        // Other headers should remain
        assert!(headers.contains_key("content-type"));
        assert!(headers.contains_key("user-agent"));
    }

    #[tokio::test]
    async fn test_non_http_events_passthrough() {
        let mut analyzer = AuthHeaderRemover::new();
        
        let event_data = json!({
            "type": "process",
            "pid": 1234,
            "command": "test"
        });
        
        let test_event = Event::new("process".to_string(), 1234, "process".to_string(), event_data.clone());
        let events = vec![test_event];
        
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = analyzer.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].data, event_data);
    }

    #[tokio::test]
    async fn test_custom_headers() {
        let mut analyzer = AuthHeaderRemover::with_headers(vec!["custom-auth".to_string()]);
        
        let event_data = json!({
            "message_type": "request",
            "headers": {
                "authorization": "Bearer token123",
                "custom-auth": "secret",
                "content-type": "application/json"
            }
        });
        
        let test_event = Event::new("http_parser".to_string(), 1234, "http_parser".to_string(), event_data);
        let events = vec![test_event];
        
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = analyzer.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        let headers = collected[0].data.get("headers").and_then(|v| v.as_object()).unwrap();
        
        // Only custom-auth should be removed (authorization not in custom list)
        assert!(headers.contains_key("authorization"));
        assert!(!headers.contains_key("custom-auth"));
        assert!(headers.contains_key("content-type"));
    }

    #[tokio::test]
    async fn test_case_insensitive_matching() {
        let mut analyzer = AuthHeaderRemover::new();
        
        let event_data = json!({
            "message_type": "request",
            "headers": {
                "Authorization": "Bearer token123",
                "X-API-KEY": "secret-key",
                "Content-Type": "application/json"
            }
        });
        
        let test_event = Event::new("http_parser".to_string(), 1234, "http_parser".to_string(), event_data);
        let events = vec![test_event];
        
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = analyzer.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        let headers = collected[0].data.get("headers").and_then(|v| v.as_object()).unwrap();
        
        // Authorization headers should be removed regardless of case
        assert!(!headers.contains_key("Authorization"));
        assert!(!headers.contains_key("X-API-KEY"));
        
        // Other headers should remain
        assert!(headers.contains_key("Content-Type"));
    }

    #[tokio::test]
    async fn test_no_headers_field() {
        let mut analyzer = AuthHeaderRemover::new();
        
        let event_data = json!({
            "message_type": "request",
            "method": "GET",
            "path": "/api/test"
        });
        
        let test_event = Event::new("http_parser".to_string(), 1234, "http_parser".to_string(), event_data.clone());
        let events = vec![test_event];
        
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = analyzer.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        // Should not crash and should pass through unchanged
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].data, event_data);
    }
}