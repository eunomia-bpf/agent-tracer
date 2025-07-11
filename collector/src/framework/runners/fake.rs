use super::{Runner, EventStream, RunnerError};
use super::common::AnalyzerProcessor;
use crate::framework::core::Event;
use crate::framework::analyzers::Analyzer;
use async_trait::async_trait;
use uuid::Uuid;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};

/// Fake runner that generates simulated SSL events for testing
pub struct FakeRunner {
    id: String,
    analyzers: Vec<Box<dyn Analyzer>>,
    event_count: usize,
    delay_ms: u64,
}

impl FakeRunner {
    /// Create a new FakeRunner
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            analyzers: Vec::new(),
            event_count: 5, // Default to 5 pairs (10 events total)
            delay_ms: 100,   // 100ms delay between events
        }
    }

    /// Set custom event count (this will generate 2x events - request + response pairs)
    pub fn event_count(mut self, count: usize) -> Self {
        self.event_count = count;
        self
    }
    
    /// Set delay between events in milliseconds  
    pub fn delay_ms(mut self, delay: u64) -> Self {
        self.delay_ms = delay;
        self
    }

    /// Set a custom ID for this runner
    pub fn with_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    /// Add an analyzer to the chain
    pub fn add_analyzer(mut self, analyzer: Box<dyn Analyzer>) -> Self {
        self.analyzers.push(analyzer);
        self
    }

    /// Generate a realistic SSL request event
    fn generate_ssl_request(pair_id: usize) -> Event {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64; // Use milliseconds for HTTP analyzer compatibility
        
        let pid = 12345 + pair_id as u32;
        let tid = pid;
        
        // Generate realistic HTTP request data
        let request_data = format!(
            "POST /v1/chat/completions HTTP/1.1\r\n\
            Host: api.openai.com\r\n\
            Accept-Encoding: gzip, deflate\r\n\
            Connection: keep-alive\r\n\
            Accept: application/json\r\n\
            Content-Type: application/json\r\n\
            User-Agent: OpenAI/Python 1.59.6\r\n\
            Authorization: Bearer sk-test-key\r\n\
            Content-Length: 150\r\n\r\n\
            {{\"model\":\"gpt-4\",\"messages\":[{{\"role\":\"user\",\"content\":\"Test request {}\"}}]}}", 
            pair_id
        );

        Event {
            id: Uuid::new_v4().to_string(),
            source: "ssl".to_string(),
            timestamp: current_time,
            data: json!({
                "comm": "python",
                "data": request_data,
                "function": "WRITE/SEND",
                "is_handshake": false,
                "latency_ms": 0.214,
                "len": request_data.len(),
                "pid": pid,
                "tid": tid,
                "time_s": current_time as f64 / 1000.0, // Convert back to seconds for this field
                "timestamp_ns": current_time * 1_000_000, // Convert to nanoseconds
                "truncated": false,
                "uid": 1000
            }),
        }
    }

    /// Generate a realistic SSL response event  
    fn generate_ssl_response(pair_id: usize) -> Event {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64 + 500; // Response comes 500ms after request
        
        let pid = 12345 + pair_id as u32;
        let tid = pid;
        
        // Generate realistic HTTP response data
        let response_data = format!(
            "HTTP/1.1 200 OK\r\n\
            Content-Type: application/json\r\n\
            Content-Length: 120\r\n\
            Date: Fri, 11 Jul 2025 19:01:04 GMT\r\n\
            Connection: keep-alive\r\n\r\n\
            {{\"id\":\"chatcmpl-test{}\",\"object\":\"chat.completion\",\"choices\":[{{\"message\":{{\"content\":\"Test response {}\"}}}}]}}",
            pair_id, pair_id
        );

        Event {
            id: Uuid::new_v4().to_string(),
            source: "ssl".to_string(),
            timestamp: current_time,
            data: json!({
                "comm": "python",
                "data": response_data,
                "function": "READ/RECV",
                "is_handshake": false,
                "latency_ms": 45.2,
                "len": response_data.len(),
                "pid": pid,
                "tid": tid,
                "time_s": current_time as f64 / 1000.0, // Convert back to seconds for this field
                "timestamp_ns": current_time * 1_000_000, // Convert to nanoseconds  
                "truncated": false,
                "uid": 1000
            }),
        }
    }
}

impl Default for FakeRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Runner for FakeRunner {
    async fn run(&mut self) -> Result<EventStream, RunnerError> {
        eprintln!("FakeRunner: Starting to generate {} fake SSL event pairs with {}ms delay", 
            self.event_count, self.delay_ms);
        
        let event_count = self.event_count;
        let delay_ms = self.delay_ms;
        
        // Create event stream using a simple generator
        let event_stream = async_stream::stream! {
            for i in 0..event_count {
                eprintln!("FakeRunner: Generating request/response pair #{}", i + 1);
                
                // Generate and yield request
                let request_event = Self::generate_ssl_request(i);
                eprintln!("FakeRunner: Yielding request event #{} (PID: {})", 
                    i + 1, request_event.data["pid"].as_u64().unwrap_or(0));
                yield request_event;
                
                // Small delay between request and response
                sleep(Duration::from_millis(delay_ms / 4)).await;
                
                // Generate and yield response  
                let response_event = Self::generate_ssl_response(i);
                eprintln!("FakeRunner: Yielding response event #{} (PID: {})", 
                    i + 1, response_event.data["pid"].as_u64().unwrap_or(0));
                yield response_event;
                
                // Longer delay between pairs (except for the last pair)
                if i < event_count - 1 {
                    sleep(Duration::from_millis(delay_ms)).await;
                }
            }
            
            eprintln!("FakeRunner: Completed generating {} request/response pairs", event_count);
        };
        
        // Process through analyzer chain
        eprintln!("FakeRunner: Processing through {} analyzers", self.analyzers.len());
        AnalyzerProcessor::process_through_analyzers(Box::pin(event_stream), &mut self.analyzers).await
    }

    fn add_analyzer(mut self, analyzer: Box<dyn Analyzer>) -> Self 
    where 
        Self: Sized 
    {
        self.analyzers.push(analyzer);
        self
    }

    fn name(&self) -> &str {
        "fake"
    }

    fn id(&self) -> String {
        self.id.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::analyzers::{HttpAnalyzer, FileLogger};
    use futures::stream::StreamExt;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_fake_runner_basic() {
        let mut runner = FakeRunner::new()
            .with_id("test-fake".to_string())
            .event_count(3)
            .delay_ms(1); // Fast for testing

        assert_eq!(runner.name(), "fake");
        assert_eq!(runner.id(), "test-fake");

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        // Should have 6 events (3 request/response pairs)
        assert_eq!(events.len(), 6);
        
        // Check that events are SSL events
        for event in &events {
            assert_eq!(event.source, "ssl");
            assert!(event.data.get("data").is_some());
            assert!(event.data.get("pid").is_some());
        }
    }

    #[tokio::test]
    async fn test_fake_runner_with_http_analyzer() {
        let mut runner = FakeRunner::new()
            .with_id("test-http".to_string())
            .event_count(2)
            .delay_ms(1)
            .add_analyzer(Box::new(HttpAnalyzer::new()));

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        // Should have some HTTP pairs created by HttpAnalyzer
        let http_pairs: Vec<_> = events.iter()
            .filter(|e| e.source == "http_analyzer" 
                && e.data.get("type").and_then(|t| t.as_str()) == Some("http_request_response_pair"))
            .collect();
        
        assert!(!http_pairs.is_empty(), "Should have created HTTP request/response pairs");
        
        // Check pair structure
        for pair in &http_pairs {
            assert!(pair.data.get("request").is_some());
            assert!(pair.data.get("response").is_some());
            assert!(pair.data.get("thread_id").is_some());
        }
    }

    #[tokio::test]
    async fn test_fake_runner_with_file_logger() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_str().unwrap();
        
        let mut runner = FakeRunner::new()
            .with_id("test-file".to_string())
            .event_count(2)
            .delay_ms(1)
            .add_analyzer(Box::new(FileLogger::new_with_options(file_path, true, true).unwrap()));

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        // Should have events
        assert!(!events.is_empty());
        
        // Check that file was written to
        let file_contents = std::fs::read_to_string(file_path).unwrap();
        assert!(!file_contents.is_empty(), "Log file should have content");
        assert!(file_contents.contains("EVENT:"), "Should contain event logs");
    }

    #[tokio::test]
    async fn test_fake_runner_full_chain() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_str().unwrap();
        
        let mut runner = FakeRunner::new()
            .with_id("test-chain".to_string())
            .event_count(3)
            .delay_ms(1)
            .add_analyzer(Box::new(HttpAnalyzer::new()))
            .add_analyzer(Box::new(FileLogger::new_with_options(file_path, true, true).unwrap()));

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        // Should have both original SSL events and HTTP pairs
        let ssl_events: Vec<_> = events.iter().filter(|e| e.source == "ssl").collect();
        let http_events: Vec<_> = events.iter().filter(|e| e.source == "http_analyzer").collect();
        
        assert!(!ssl_events.is_empty(), "Should have SSL events");
        assert!(!http_events.is_empty(), "Should have HTTP analyzer events");
        
        // Check that file contains both types
        let file_contents = std::fs::read_to_string(file_path).unwrap();
        assert!(file_contents.contains("source=ssl"), "Should log SSL events");
        assert!(file_contents.contains("source=http_analyzer"), "Should log HTTP events");
    }
} 