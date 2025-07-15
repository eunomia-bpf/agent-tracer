#[cfg(test)]
mod sse_processor_tests {
    use super::super::sse_processor::SSEProcessor;
    use super::super::{Analyzer, AnalyzerError};
    use crate::framework::runners::EventStream;
    use crate::framework::core::Event;
    use async_trait::async_trait;
    use futures::stream::StreamExt;
    use serde_json::json;
    use futures::stream;

    #[tokio::test]
    async fn test_sse_processor_creation() {
        let processor = SSEProcessor::new();
        assert_eq!(processor.name(), "SSEProcessor");
    }

    #[tokio::test]
    async fn test_sse_processor_with_timeout() {
        let processor = SSEProcessor::new_with_timeout(5000);
        assert_eq!(processor.name(), "SSEProcessor");
    }

    #[tokio::test]
    async fn test_is_sse_data() {
        assert!(SSEProcessor::is_sse_data("event: content_block_delta\ndata: {\"type\":\"content_block_delta\"}\r\n0\r\n\r\n"));
        assert!(SSEProcessor::is_sse_data("event: message_start\ndata: {\"message\":{\"id\":\"123\"}}\r\n0\r\n\r\n"));
        assert!(SSEProcessor::is_sse_data("Transfer-Encoding: chunked\r\nevent: content_block_delta\r\ndata: {\"type\":\"content_block_delta\"}\r\n0\r\n\r\n"));
        assert!(SSEProcessor::is_sse_data("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n"));
        assert!(!SSEProcessor::is_sse_data("regular text"));
    }

    #[tokio::test]
    async fn test_parse_sse_events() {
        let sse_data = "event: content_block_delta\ndata: {\"type\":\"content_block_delta\"}\r\n0\r\n\r\n";
        let events = SSEProcessor::parse_sse_events(sse_data);
        assert!(!events.is_empty());
    }

    #[tokio::test]
    async fn test_sse_processor_processes_events() {
        let mut processor = SSEProcessor::new();
        
        let test_event = Event::new("ssl".to_string(), json!({
            "comm": "test",
            "data": "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"hello\"}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
            "function": "READ/RECV",
            "pid": 1234,
            "tid": 1234,
            "timestamp_ns": 1000000000
        }));
        
        let events = vec![test_event];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = processor.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        // Should have processed the event and completed due to message_stop
        assert!(!collected.is_empty());
        
        // Should be an sse_processor event
        if let Some(merged_event) = collected.first() {
            assert_eq!(merged_event.source, "sse_processor");
        }
    }

    #[tokio::test]
    async fn test_sse_processor_ignores_non_ssl_events() {
        let mut processor = SSEProcessor::new();
        
        let test_event = Event::new("process".to_string(), json!({
            "comm": "test",
            "data": "some data",
            "pid": 1234
        }));
        
        let events = vec![test_event.clone()];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = processor.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        // Should pass through non-SSL events unchanged
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].source, "process");
    }

    #[tokio::test]
    async fn test_sse_processor_ignores_non_sse_ssl_events() {
        let mut processor = SSEProcessor::new();
        
        let test_event = Event::new("ssl".to_string(), json!({
            "comm": "test", 
            "data": "regular HTTP data without SSE",
            "function": "READ/RECV",
            "pid": 1234
        }));
        
        let events = vec![test_event.clone()];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = processor.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        // Should pass through non-SSE SSL events unchanged
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].source, "ssl");
    }

    #[tokio::test]
    async fn test_enhanced_sse_detection() {
        // Test enhanced SSE detection like ssl_log_analyzer.py
        
        // Test with Content-Type header
        assert!(SSEProcessor::is_sse_data("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n"));
        
        // Test with chunked encoding + SSE events
        assert!(SSEProcessor::is_sse_data("Transfer-Encoding: chunked\r\n\r\n1a\r\nevent: message_start\r\n"));
        
        // Test with just data field
        assert!(SSEProcessor::is_sse_data("data: {\"message\": \"hello\"}\r\n\r\n"));
        
        // Test negative cases
        assert!(!SSEProcessor::is_sse_data("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"data\": \"value\"}"));
        assert!(!SSEProcessor::is_sse_data("Regular HTTP response body"));
    }

    #[tokio::test]
    async fn test_enhanced_chunked_content_cleaning() {
        // Test enhanced chunked content cleaning like ssl_log_analyzer.py
        
        let chunked_data = "1a\r\nevent: content_block_delta\r\n0\r\n\r\n";
        let cleaned = SSEProcessor::clean_chunked_content(chunked_data);
        assert!(cleaned.contains("event: content_block_delta"));
        assert!(!cleaned.contains("1a")); // Chunk size should be removed
        
        let multi_chunk_data = "10\r\nevent: message_start\r\n15\r\ndata: {\"id\": \"123\"}\r\n0\r\n\r\n";
        let cleaned_multi = SSEProcessor::clean_chunked_content(multi_chunk_data);
        assert!(cleaned_multi.contains("event: message_start"));
        assert!(cleaned_multi.contains("data: {\"id\": \"123\"}"));
        assert!(!cleaned_multi.contains("10")); // Chunk sizes should be removed
        assert!(!cleaned_multi.contains("15"));
    }

    #[tokio::test]
    async fn test_sse_processor_with_thinking_content() {
        // Test processing thinking deltas like in ssl_log_analyzer.py
        let mut processor = SSEProcessor::new();
        
        let test_event = Event::new("ssl".to_string(), json!({
            "comm": "claude",
            "data": "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"Let me think about this...\"}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
            "function": "READ/RECV",
            "pid": 1234,
            "tid": 1234,
            "timestamp_ns": 1000000000
        }));
        
        let events = vec![test_event];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = processor.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        // Should have processed the event and completed due to message_stop
        assert!(!collected.is_empty());
        
        // Should be an sse_processor event with thinking content
        if let Some(merged_event) = collected.first() {
            assert_eq!(merged_event.source, "sse_processor");
            let merged_content = merged_event.data.get("merged_content").and_then(|v| v.as_str()).unwrap_or("");
            assert!(merged_content.contains("Let me think about this..."));
        }
    }

    #[tokio::test]
    async fn test_sse_processor_timeline_behavior() {
        // Test timeline-like behavior similar to ssl_log_analyzer.py's group_by_timeline
        let mut processor = SSEProcessor::new();
        
        // Create a sequence of SSE events that should be merged
        let events = vec![
            Event::new("ssl".to_string(), json!({
                "comm": "claude",
                "data": "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_123\"}}\n\n",
                "function": "READ/RECV",
                "pid": 1234,
                "tid": 1234,
                "timestamp_ns": 1000000000
            })),
            Event::new("ssl".to_string(), json!({
                "comm": "claude", 
                "data": "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello \"}}\n\n",
                "function": "READ/RECV",
                "pid": 1234,
                "tid": 1234,
                "timestamp_ns": 1000000100
            })),
            Event::new("ssl".to_string(), json!({
                "comm": "claude",
                "data": "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"World!\"}}\n\n",
                "function": "READ/RECV", 
                "pid": 1234,
                "tid": 1234,
                "timestamp_ns": 1000000200
            })),
            Event::new("ssl".to_string(), json!({
                "comm": "claude",
                "data": "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
                "function": "READ/RECV",
                "pid": 1234,
                "tid": 1234,
                "timestamp_ns": 1000000300
            }))
        ];
        
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = processor.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        println!("Timeline test collected {} events", collected.len());
        
        // Should have one merged event (the message_stop should trigger completion)
        let sse_events = collected.iter().filter(|e| e.source == "sse_processor").count();
        assert!(sse_events >= 1, "Should have at least 1 merged SSE event");
        
        // Check that text was properly accumulated
        if let Some(merged_event) = collected.iter().find(|e| e.source == "sse_processor") {
            let merged_content = merged_event.data.get("merged_content").and_then(|v| v.as_str()).unwrap_or("");
            assert_eq!(merged_content, "Hello World!", "Should have accumulated all text deltas");
            assert_eq!(merged_event.data.get("message_id").and_then(|v| v.as_str()).unwrap_or(""), "msg_123");
        }
    }

    #[tokio::test]
    async fn test_sse_processor_with_partial_json() {
        // Test processing partial JSON like in ssl_log_analyzer.py
        let mut processor = SSEProcessor::new();
        
        let test_event = Event::new("ssl".to_string(), json!({
            "comm": "claude",
            "data": "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"message\\\":\"}}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"\\\"Hello World!\\\"}\"}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
            "function": "READ/RECV",
            "pid": 1234,
            "tid": 1234,
            "timestamp_ns": 1000000000
        }));
        
        let events = vec![test_event];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = processor.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        // Should have processed the event and completed due to message_stop
        assert!(!collected.is_empty());
        
        // Should be an sse_processor event with accumulated JSON
        if let Some(merged_event) = collected.first() {
            assert_eq!(merged_event.source, "sse_processor");
            let merged_content = merged_event.data.get("merged_content").and_then(|v| v.as_str()).unwrap_or("");
            assert!(merged_content.contains("Hello World!"));
            assert_eq!(merged_event.data.get("content_type").and_then(|v| v.as_str()).unwrap_or(""), "json");
        }
    }
}