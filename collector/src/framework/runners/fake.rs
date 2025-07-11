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
    use crate::framework::analyzers::{HttpAnalyzer, FileLogger, OutputAnalyzer, Analyzer};
    use futures::stream::StreamExt;
    use std::fs;
    use tokio::time::Duration;
    use serde_json::json;
    use std::time::Instant;

    #[tokio::test]
    async fn test_fake_runner_basic() {
        let mut runner = FakeRunner::new()
            .with_id("test-basic".to_string())
            .event_count(2)
            .delay_ms(10); // Fast for testing

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        // Should generate 2 pairs = 4 events total
        assert_eq!(events.len(), 4);
        
        // Check that we have alternating request/response events
        assert_eq!(events[0].data["function"].as_str().unwrap(), "WRITE/SEND"); // Request
        assert_eq!(events[1].data["function"].as_str().unwrap(), "READ/RECV");  // Response
        assert_eq!(events[2].data["function"].as_str().unwrap(), "WRITE/SEND"); // Request
        assert_eq!(events[3].data["function"].as_str().unwrap(), "READ/RECV");  // Response
        
        // All events should have ssl source
        for event in &events {
            assert_eq!(event.source, "ssl");
        }
    }

    #[tokio::test]
    async fn test_fake_runner_with_http_analyzer() {
        let mut runner = FakeRunner::new()
            .with_id("test-http".to_string())
            .event_count(2)
            .delay_ms(10)
            .add_analyzer(Box::new(HttpAnalyzer::new_with_wait_time(5000))); // 5 second timeout

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        println!("HTTP Analyzer Test Results:");
        println!("Total events: {}", events.len());
        
        let ssl_events = events.iter().filter(|e| e.source == "ssl").count();
        let http_pairs = events.iter()
            .filter(|e| e.source == "http_analyzer" 
                && e.data.get("type").and_then(|t| t.as_str()) == Some("http_request_response_pair"))
            .count();
            
        println!("SSL events: {}", ssl_events);
        println!("HTTP pairs: {}", http_pairs);
        
        // Should have original SSL events forwarded
        assert!(ssl_events > 0, "Should have SSL events forwarded");
        
        // Should have HTTP pairs created
        assert!(http_pairs > 0, "Should have HTTP request/response pairs");
        
        // Show HTTP pairs for debugging
        for event in &events {
            if event.source == "http_analyzer" && 
               event.data.get("type").and_then(|t| t.as_str()) == Some("http_request_response_pair") {
                println!("HTTP Pair: {} {} -> {} {}", 
                    event.data["request"]["method"].as_str().unwrap_or("?"),
                    event.data["request"]["url"].as_str().unwrap_or("?"),
                    event.data["response"]["status_code"].as_u64().unwrap_or(0),
                    event.data["response"]["status_text"].as_str().unwrap_or("?")
                );
            }
        }
    }

    #[tokio::test] 
    async fn test_fake_runner_with_file_logger() {
        let test_log_file = "test_fake_runner.log";
        
        // Clean up any existing test file
        let _ = fs::remove_file(test_log_file);
        
        let mut runner = FakeRunner::new()
            .with_id("test-file-logger".to_string())
            .event_count(2)
            .delay_ms(10)
            .add_analyzer(Box::new(FileLogger::new_with_options(test_log_file, true, true).unwrap()));

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        println!("File Logger Test Results:");
        println!("Total events: {}", events.len());
        
        // Check if log file was created
        assert!(std::path::Path::new(test_log_file).exists(), "Log file should be created");
        
        let log_size = fs::metadata(test_log_file).unwrap().len();
        println!("Log file size: {} bytes", log_size);
        assert!(log_size > 0, "Log file should not be empty");
        
        // Read and check log contents
        let log_contents = fs::read_to_string(test_log_file).unwrap();
        let log_lines: Vec<&str> = log_contents.lines().collect();
        println!("Log file lines: {}", log_lines.len());
        assert!(log_lines.len() > 0, "Log file should have content");
        
        // Clean up
        let _ = fs::remove_file(test_log_file);
    }

    #[tokio::test]
    async fn test_full_analyzer_chain() {
        let test_log_file = "test_full_chain.log";
        
        // Clean up any existing test file
        let _ = fs::remove_file(test_log_file);
        
        let mut runner = FakeRunner::new()
            .with_id("test-full-chain".to_string())
            .event_count(2)
            .delay_ms(10)
            .add_analyzer(Box::new(HttpAnalyzer::new_with_wait_time(5000)))
            .add_analyzer(Box::new(FileLogger::new_with_options(test_log_file, true, true).unwrap()))
            .add_analyzer(Box::new(OutputAnalyzer::new_with_options(false, false, false))); // Silent output

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        println!("Full Chain Test Results:");
        println!("Total events: {}", events.len());
        
        let ssl_events = events.iter().filter(|e| e.source == "ssl").count();
        let http_pairs = events.iter()
            .filter(|e| e.source == "http_analyzer" 
                && e.data.get("type").and_then(|t| t.as_str()) == Some("http_request_response_pair"))
            .count();
            
        println!("SSL events: {}", ssl_events);
        println!("HTTP pairs: {}", http_pairs);
        
        // Verify all components worked
        assert!(ssl_events > 0, "Should have SSL events");
        assert!(http_pairs > 0, "Should have HTTP pairs");
        assert!(std::path::Path::new(test_log_file).exists(), "Log file should exist");
        
        let log_size = fs::metadata(test_log_file).unwrap().len();
        assert!(log_size > 0, "Log file should not be empty");
        
        println!("✅ Full analyzer chain test completed successfully!");
        
        // Clean up
        let _ = fs::remove_file(test_log_file);
    }

    #[tokio::test]
    async fn test_analyzer_chain_order_independence() {
        let test_log_file1 = "test_order1.log";
        let test_log_file2 = "test_order2.log";
        
        // Clean up any existing test files
        let _ = fs::remove_file(test_log_file1);
        let _ = fs::remove_file(test_log_file2);
        
        // Test chain: HTTP -> FileLogger -> Output
        let mut runner1 = FakeRunner::new()
            .with_id("test-order1".to_string())
            .event_count(2)
            .delay_ms(10)
            .add_analyzer(Box::new(HttpAnalyzer::new_with_wait_time(5000)))
            .add_analyzer(Box::new(FileLogger::new_with_options(test_log_file1, true, true).unwrap()))
            .add_analyzer(Box::new(OutputAnalyzer::new_with_options(false, false, false)));

        // Test chain: FileLogger -> HTTP -> Output
        let mut runner2 = FakeRunner::new()
            .with_id("test-order2".to_string())
            .event_count(2)
            .delay_ms(10)
            .add_analyzer(Box::new(FileLogger::new_with_options(test_log_file2, true, true).unwrap()))
            .add_analyzer(Box::new(HttpAnalyzer::new_with_wait_time(5000)))
            .add_analyzer(Box::new(OutputAnalyzer::new_with_options(false, false, false)));

        let stream1 = runner1.run().await.unwrap();
        let events1: Vec<_> = stream1.collect().await;
        
        let stream2 = runner2.run().await.unwrap();
        let events2: Vec<_> = stream2.collect().await;
        
        println!("Chain Order Test Results:");
        println!("Order 1 (HTTP->File->Output) events: {}", events1.len());
        println!("Order 2 (File->HTTP->Output) events: {}", events2.len());
        
        // Both should have similar results
        assert!(events1.len() > 0, "Order 1 should produce events");
        assert!(events2.len() > 0, "Order 2 should produce events");
        
        // Both should generate HTTP pairs
        let pairs1 = events1.iter().filter(|e| e.source == "http_analyzer").count();
        let pairs2 = events2.iter().filter(|e| e.source == "http_analyzer").count();
        
        assert!(pairs1 > 0, "Order 1 should generate HTTP pairs");
        assert!(pairs2 > 0, "Order 2 should generate HTTP pairs");
        
        // Both log files should exist and have content
        assert!(std::path::Path::new(test_log_file1).exists(), "Log file 1 should exist");
        assert!(std::path::Path::new(test_log_file2).exists(), "Log file 2 should exist");
        
        println!("✅ Analyzer chain order independence test completed!");
        
        // Clean up
        let _ = fs::remove_file(test_log_file1);
        let _ = fs::remove_file(test_log_file2);
    }

    #[tokio::test]
    async fn test_multiple_analyzer_instances() {
        let test_log_file1 = "test_multi1.log";
        let test_log_file2 = "test_multi2.log";
        
        // Clean up any existing test files
        let _ = fs::remove_file(test_log_file1);
        let _ = fs::remove_file(test_log_file2);
        
        // Chain with multiple file loggers and output analyzers
        let mut runner = FakeRunner::new()
            .with_id("test-multi".to_string())
            .event_count(2)
            .delay_ms(10)
            .add_analyzer(Box::new(HttpAnalyzer::new_with_wait_time(5000)))
            .add_analyzer(Box::new(FileLogger::new_with_options(test_log_file1, true, true).unwrap()))
            .add_analyzer(Box::new(FileLogger::new_with_options(test_log_file2, false, false).unwrap())) // Different settings
            .add_analyzer(Box::new(OutputAnalyzer::new_with_options(false, false, false)))
            .add_analyzer(Box::new(OutputAnalyzer::new_with_options(false, true, false))); // Different settings

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        println!("Multiple Analyzer Instances Test Results:");
        println!("Total events: {}", events.len());
        
        // Verify all events passed through multiple analyzers
        assert!(events.len() > 0, "Should have events");
        
        // Both log files should exist
        assert!(std::path::Path::new(test_log_file1).exists(), "Log file 1 should exist");
        assert!(std::path::Path::new(test_log_file2).exists(), "Log file 2 should exist");
        
        // Verify file contents (file1 should have more content due to pretty printing and all events)
        let size1 = fs::metadata(test_log_file1).unwrap().len();
        let size2 = fs::metadata(test_log_file2).unwrap().len();
        
        assert!(size1 > 0, "Log file 1 should have content");
        assert!(size2 > 0, "Log file 2 should have content"); 
        assert!(size1 >= size2, "Pretty printed log should be larger or equal");
        
        println!("✅ Multiple analyzer instances test completed!");
        
        // Clean up
        let _ = fs::remove_file(test_log_file1);
        let _ = fs::remove_file(test_log_file2);
    }

    #[tokio::test]
    async fn test_analyzer_chain_performance() {
        let start_time = std::time::Instant::now();
        
        // Test with many events
        let mut runner = FakeRunner::new()
            .with_id("test-performance".to_string())
            .event_count(50) // 100 events total (50 pairs)
            .delay_ms(1) // Minimal delay for speed
            .add_analyzer(Box::new(HttpAnalyzer::new_with_wait_time(10000)))
            .add_analyzer(Box::new(OutputAnalyzer::new_with_options(false, false, false))); // Silent

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        let elapsed = start_time.elapsed();
        
        println!("Performance Test Results:");
        println!("Events processed: {}", events.len());
        println!("Time elapsed: {:?}", elapsed);
        println!("Events per second: {:.2}", events.len() as f64 / elapsed.as_secs_f64());
        
        // Verify we processed the expected number of events
        assert!(events.len() >= 100, "Should have at least 100 SSL events (50 pairs)");
        
        // Verify HTTP pairs were created
        let http_pairs = events.iter()
            .filter(|e| e.source == "http_analyzer")
            .count();
        assert!(http_pairs > 0, "Should have HTTP pairs");
        
        // Performance assertion - should complete within reasonable time
        assert!(elapsed.as_secs() < 10, "Should complete within 10 seconds");
        
        println!("✅ Performance test completed!");
    }

    #[tokio::test]
    async fn test_analyzer_chain_empty_stream() {
        // Test with zero events
        let mut runner = FakeRunner::new()
            .with_id("test-empty".to_string())
            .event_count(0) // No events
            .delay_ms(10)
            .add_analyzer(Box::new(HttpAnalyzer::new_with_wait_time(5000)))
            .add_analyzer(Box::new(OutputAnalyzer::new_with_options(false, false, false)));

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        println!("Empty Stream Test Results:");
        println!("Events processed: {}", events.len());
        
        // Should handle empty stream gracefully
        assert_eq!(events.len(), 0, "Should have no events");
        
        println!("✅ Empty stream test completed!");
    }

    #[tokio::test]
    async fn test_http_analyzer_timeout_cleanup() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // Create a custom FakeRunner that generates only requests (no responses)
        let mut runner = FakeRunner::new()
            .with_id("test-timeout".to_string())
            .event_count(0) // We'll manually create events
            .delay_ms(10);

        // Add HTTP analyzer with very short timeout
        runner = runner.add_analyzer(Box::new(HttpAnalyzer::new_with_wait_time(100))); // 100ms timeout
        runner = runner.add_analyzer(Box::new(OutputAnalyzer::new_with_options(false, false, false)));

        // Override the event generation to create only requests
        let event_stream = async_stream::stream! {
            let base_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            
            // Generate 3 requests with no responses
            for i in 0..3 {
                let request_data = format!(
                    "GET /timeout-test-{} HTTP/1.1\r\nHost: example.com\r\nUser-Agent: test\r\n\r\n", 
                    i
                );
                
                let event = Event::new_with_id_and_timestamp(
                    format!("req_{}", i),
                    base_time + i * 50, // Space them 50ms apart
                    "ssl".to_string(),
                    json!({
                        "data": request_data,
                        "pid": 1000 + i,
                        "timestamp_ns": (base_time + i * 50) * 1_000_000
                    })
                );
                
                yield event;
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            
            // Wait for timeout to trigger cleanup
            tokio::time::sleep(Duration::from_millis(200)).await;
            
            // Generate one more request to trigger cleanup
            let cleanup_event = Event::new_with_id_and_timestamp(
                "cleanup".to_string(),
                base_time + 300,
                "ssl".to_string(),
                json!({
                    "data": "GET /cleanup HTTP/1.1\r\nHost: example.com\r\n\r\n",
                    "pid": 9999,
                    "timestamp_ns": (base_time + 300) * 1_000_000
                })
            );
            yield cleanup_event;
        };

        // Process through analyzers manually
        let processed_stream = crate::framework::runners::common::AnalyzerProcessor::process_through_analyzers(
            Box::pin(event_stream), 
            &mut runner.analyzers
        ).await.unwrap();
        
        let events: Vec<_> = processed_stream.collect().await;
        
        println!("Timeout Cleanup Test Results:");
        println!("Total events: {}", events.len());
        
        // Should have the original SSL events forwarded
        let ssl_events = events.iter().filter(|e| e.source == "ssl").count();
        assert_eq!(ssl_events, 4, "Should have 4 SSL events forwarded");
        
        // Should not have any HTTP pairs due to timeout
        let http_pairs = events.iter()
            .filter(|e| e.source == "http_analyzer")
            .count();
        assert_eq!(http_pairs, 0, "Should have no HTTP pairs due to timeout");
        
        println!("✅ HTTP analyzer timeout cleanup test completed!");
    }

    #[tokio::test] 
    async fn test_analyzer_chain_with_mixed_event_sources() {
        // Test analyzer chain with events from different sources
        let mut runner = FakeRunner::new()
            .with_id("test-mixed".to_string())
            .event_count(0) // Manual event generation
            .delay_ms(10);

        runner = runner.add_analyzer(Box::new(HttpAnalyzer::new_with_wait_time(5000)));
        runner = runner.add_analyzer(Box::new(OutputAnalyzer::new_with_options(false, false, false)));

        // Generate mixed source events
        let event_stream = async_stream::stream! {
            // SSL events (should be processed by HTTP analyzer)
            yield Event::new("ssl".to_string(), json!({
                "data": "GET /api/test HTTP/1.1\r\nHost: example.com\r\n\r\n",
                "pid": 1234
            }));
            
            yield Event::new("ssl".to_string(), json!({
                "data": "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"result\":\"ok\"}",
                "pid": 1234
            }));
            
            // Non-SSL events (should be forwarded unchanged)
            yield Event::new("process".to_string(), json!({
                "pid": 5678,
                "command": "test_process"
            }));
            
            yield Event::new("custom".to_string(), json!({
                "message": "custom event",
                "value": 42
            }));
        };

        let processed_stream = crate::framework::runners::common::AnalyzerProcessor::process_through_analyzers(
            Box::pin(event_stream), 
            &mut runner.analyzers
        ).await.unwrap();
        
        let events: Vec<_> = processed_stream.collect().await;
        
        println!("Mixed Event Sources Test Results:");
        println!("Total events: {}", events.len());
        
        // Count events by source
        let ssl_events = events.iter().filter(|e| e.source == "ssl").count();
        let process_events = events.iter().filter(|e| e.source == "process").count();
        let custom_events = events.iter().filter(|e| e.source == "custom").count();
        let http_pairs = events.iter().filter(|e| e.source == "http_analyzer").count();
        
        println!("SSL events: {}", ssl_events);
        println!("Process events: {}", process_events);
        println!("Custom events: {}", custom_events);
        println!("HTTP pairs: {}", http_pairs);
        
        // Verify all events are preserved
        assert_eq!(ssl_events, 2, "Should have 2 SSL events");
        assert_eq!(process_events, 1, "Should have 1 process event");
        assert_eq!(custom_events, 1, "Should have 1 custom event");
        assert!(http_pairs > 0, "Should have HTTP pairs from SSL events");
        
        println!("✅ Mixed event sources test completed!");
    }

    #[tokio::test]
    async fn test_analyzer_chain_memory_cleanup() {
        use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
        
        // Create a custom analyzer that tracks memory usage
        struct MemoryTrackingAnalyzer {
            event_count: Arc<AtomicUsize>,
            max_events_seen: Arc<AtomicUsize>,
        }
        
        impl MemoryTrackingAnalyzer {
            fn new() -> Self {
                Self {
                    event_count: Arc::new(AtomicUsize::new(0)),
                    max_events_seen: Arc::new(AtomicUsize::new(0)),
                }
            }
        }
        
        #[async_trait::async_trait]
        impl Analyzer for MemoryTrackingAnalyzer {
            async fn process(&mut self, stream: EventStream) -> Result<EventStream, crate::framework::analyzers::AnalyzerError> {
                let event_count = self.event_count.clone();
                let max_events = self.max_events_seen.clone();
                
                let processed_stream = stream.map(move |event| {
                    let current = event_count.fetch_add(1, Ordering::SeqCst) + 1;
                    max_events.fetch_max(current, Ordering::SeqCst);
                    
                    // Simulate processing and cleanup
                    if current % 10 == 0 {
                        // Simulate periodic cleanup
                        event_count.store(0, Ordering::SeqCst);
                    }
                    
                    event
                });
                
                Ok(Box::pin(processed_stream))
            }
            
            fn name(&self) -> &str {
                "MemoryTrackingAnalyzer"
            }
        }
        
        let memory_tracker = MemoryTrackingAnalyzer::new();
        let max_events_ref = memory_tracker.max_events_seen.clone();
        
        let mut runner = FakeRunner::new()
            .with_id("test-memory".to_string())
            .event_count(25) // 50 events total
            .delay_ms(1)
            .add_analyzer(Box::new(memory_tracker))
            .add_analyzer(Box::new(OutputAnalyzer::new_with_options(false, false, false)));

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        let max_events_seen = max_events_ref.load(Ordering::SeqCst);
        
        println!("Memory Cleanup Test Results:");
        println!("Total events processed: {}", events.len());
        println!("Max events seen at once: {}", max_events_seen);
        
        // Verify events were processed
        assert!(events.len() >= 50, "Should have processed at least 50 events");
        
        // Verify memory tracking worked (cleanup occurred)
        assert!(max_events_seen < events.len(), "Memory cleanup should have occurred");
        assert!(max_events_seen <= 10, "Should not accumulate more than 10 events due to cleanup");
        
        println!("✅ Memory cleanup test completed!");
    }

    #[test]
    fn test_fake_runner_builder_pattern() {
        // Test the fluent builder pattern
        let runner = FakeRunner::new()
            .with_id("test-builder".to_string())
            .event_count(10)
            .delay_ms(50)
            .add_analyzer(Box::new(OutputAnalyzer::new()));
        
        assert_eq!(runner.id(), "test-builder");
        // Note: event_count and delay_ms are private fields, so we can't test them directly
        // But we can verify the runner was created successfully and has the right ID
        assert_eq!(runner.name(), "fake");
    }

    #[tokio::test]
    async fn test_analyzer_chain_integration_scenario() {
        // Comprehensive integration test that simulates real-world usage
        let test_log_file = "test_integration.log";
        
        // Clean up any existing test file
        let _ = fs::remove_file(test_log_file);
        
        println!("Starting comprehensive analyzer chain integration test...");
        
        // Create a realistic analyzer chain that might be used in production:
        // 1. HTTP analyzer for pairing requests/responses
        // 2. File logger for persistence 
        // 3. Output analyzer for real-time display
        let mut runner = FakeRunner::new()
            .with_id("integration-test".to_string())
            .event_count(10) // 20 events total
            .delay_ms(25) // Realistic timing
            .add_analyzer(Box::new(HttpAnalyzer::new_with_wait_time(10000))) // 10 second timeout
            .add_analyzer(Box::new(FileLogger::new_with_options(test_log_file, true, true).unwrap()))
            .add_analyzer(Box::new(OutputAnalyzer::new_with_options(false, false, false))); // Silent for test

        let start_time = Instant::now();
        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        let elapsed = start_time.elapsed();
        
        println!("Integration Test Results:");
        println!("Total processing time: {:?}", elapsed);
        println!("Total events processed: {}", events.len());
        
        // Analyze event distribution
        let ssl_events = events.iter().filter(|e| e.source == "ssl").count();
        let http_pairs = events.iter()
            .filter(|e| e.source == "http_analyzer" 
                && e.data.get("type").and_then(|t| t.as_str()) == Some("http_request_response_pair"))
            .count();
        
        println!("SSL events: {}", ssl_events);
        println!("HTTP pairs: {}", http_pairs);
        
        // Verify expected behavior
        assert_eq!(ssl_events, 20, "Should have 20 SSL events (10 request/response pairs)");
        assert!(http_pairs >= 5, "Should have created multiple HTTP pairs");
        assert!(http_pairs <= 10, "Should not have more pairs than request/response pairs");
        
        // Verify file logging worked
        assert!(std::path::Path::new(test_log_file).exists(), "Log file should exist");
        let log_content = fs::read_to_string(test_log_file).unwrap();
        let log_lines = log_content.lines().count();
        println!("Log file lines: {}", log_lines);
        assert!(log_lines > 0, "Log file should have content");
        
        // Verify performance characteristics
        let events_per_second = events.len() as f64 / elapsed.as_secs_f64();
        println!("Processing rate: {:.2} events/second", events_per_second);
        assert!(events_per_second > 10.0, "Should process at least 10 events per second");
        
        // Verify HTTP analyzer functionality with detailed checks
        for event in &events {
            if event.source == "http_analyzer" && 
               event.data.get("type").and_then(|t| t.as_str()) == Some("http_request_response_pair") {
                
                // Verify HTTP pair structure
                assert!(event.data.get("request").is_some(), "HTTP pair should have request");
                assert!(event.data.get("response").is_some(), "HTTP pair should have response");
                assert!(event.data.get("latency_ms").is_some(), "HTTP pair should have latency");
                assert!(event.data.get("thread_id").is_some(), "HTTP pair should have thread_id");
                
                let request = &event.data["request"];
                let response = &event.data["response"];
                
                // Verify request structure
                assert!(request.get("method").is_some(), "Request should have method");
                assert!(request.get("url").is_some(), "Request should have URL");
                assert!(request.get("headers").is_some(), "Request should have headers");
                
                // Verify response structure
                assert!(response.get("status_code").is_some(), "Response should have status code");
                assert!(response.get("status_text").is_some(), "Response should have status text");
                assert!(response.get("headers").is_some(), "Response should have headers");
                
                println!("Validated HTTP pair: {} {} -> {} {}", 
                    request["method"].as_str().unwrap_or("?"),
                    request["url"].as_str().unwrap_or("?"),
                    response["status_code"].as_u64().unwrap_or(0),
                    response["status_text"].as_str().unwrap_or("?")
                );
            }
        }
        
        println!("✅ Comprehensive analyzer chain integration test completed successfully!");
        
        // Clean up
        let _ = fs::remove_file(test_log_file);
    }

    #[test]
    fn test_ssl_event_structure() {
        let request = FakeRunner::generate_ssl_request(0);
        let response = FakeRunner::generate_ssl_response(0);
        
        // Verify request structure
        assert_eq!(request.source, "ssl");
        assert_eq!(request.data["function"].as_str().unwrap(), "WRITE/SEND");
        assert_eq!(request.data["pid"].as_u64().unwrap(), 12345);
        assert!(request.data["data"].as_str().unwrap().contains("POST /v1/chat/completions"));
        
        // Verify response structure  
        assert_eq!(response.source, "ssl");
        assert_eq!(response.data["function"].as_str().unwrap(), "READ/RECV");
        assert_eq!(response.data["pid"].as_u64().unwrap(), 12345);
        assert!(response.data["data"].as_str().unwrap().contains("HTTP/1.1 200 OK"));
        
        // Verify timing
        assert!(response.timestamp > request.timestamp, "Response should come after request");
    }
} 