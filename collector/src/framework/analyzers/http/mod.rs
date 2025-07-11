//! HTTP Analyzer Module
//! 
//! This module provides HTTP request/response pairing functionality for SSL traffic.
//! It consists of:
//! - `types`: HTTP data structures (PendingRequest, HttpResponse)
//! - `parser`: HTTP parsing utilities
//! - `analyzer`: Main analyzer implementation

pub mod types;
pub mod parser;
pub mod analyzer;

// Re-export the main HttpAnalyzer for external use
pub use analyzer::HttpAnalyzer;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::core::Event;
    use futures::stream;
    use serde_json::json;
    use crate::framework::analyzers::Analyzer;
    use crate::framework::runners::EventStream;
    use futures::stream::StreamExt;

    #[tokio::test]
    async fn test_http_request_parsing() {
        let http_data = "GET /api/users HTTP/1.1\r\nHost: example.com\r\nUser-Agent: curl/7.68.0\r\n\r\n";
        let event = Event::new("ssl".to_string(), json!({
            "data": http_data,
            "pid": 1234,
            "timestamp_ns": 1234567890
        }));

        let request = parser::HttpParser::parse_http_request(http_data, &event).unwrap();
        assert_eq!(request.method, "GET");
        assert_eq!(request.url, "/api/users");
        assert_eq!(request.headers.get("host"), Some(&"example.com".to_string()));
        assert_eq!(request.pid, 1234);
    }

    #[tokio::test]
    async fn test_http_response_parsing() {
        let http_data = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 13\r\n\r\n{\"status\":\"ok\"}";
        
        let response = parser::HttpParser::parse_http_response(http_data, &Event::new("ssl".to_string(), json!({
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
        
        // Should have 3 events: 2 original SSL events + 1 HTTP pair 
        assert_eq!(collected.len(), 3);
        
        // Check that we have both original SSL events forwarded
        let ssl_events: Vec<_> = collected.iter().filter(|e| e.source == "ssl").collect();
        assert_eq!(ssl_events.len(), 2, "Should forward both original SSL events");
        
        // Check that we have one HTTP pair
        let http_pairs: Vec<_> = collected.iter()
            .filter(|e| e.source == "http_analyzer" 
                && e.data.get("type").and_then(|t| t.as_str()) == Some("http_request_response_pair"))
            .collect();
        assert_eq!(http_pairs.len(), 1, "Should have exactly one HTTP pair");
        
        // Check HTTP pair content
        let pair = &http_pairs[0];
        assert_eq!(pair.data["request"]["method"], "GET");
        assert_eq!(pair.data["response"]["status_code"], 200);
        assert_eq!(pair.data["thread_id"], 1234);
    }

    #[tokio::test]
    async fn test_analyzer_name() {
        let analyzer = HttpAnalyzer::new();
        assert_eq!(analyzer.name(), "HttpAnalyzer");
    }

    #[test]
    fn test_http_detection() {
        assert!(parser::HttpParser::starts_with_http_request("GET /test HTTP/1.1"));
        assert!(parser::HttpParser::starts_with_http_request("POST /api HTTP/1.1"));
        assert!(!parser::HttpParser::starts_with_http_request("invalid data"));
        
        assert!(parser::HttpParser::starts_with_http_response("HTTP/1.1 200 OK"));
        assert!(parser::HttpParser::starts_with_http_response("HTTP/1.0 404 Not Found"));
        assert!(!parser::HttpParser::starts_with_http_response("GET /test"));
    }

    #[test]
    fn test_message_extraction() {
        let buffer = "GET /test HTTP/1.1\r\nHost: example.com\r\n\r\nHTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nHello";
        let messages = parser::HttpParser::extract_http_messages(buffer);
        
        assert_eq!(messages.len(), 2);
        assert!(messages[0].0.starts_with("GET /test"));
        assert!(messages[1].0.starts_with("HTTP/1.1 200"));
    }
} 