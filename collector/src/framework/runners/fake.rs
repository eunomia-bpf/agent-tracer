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
    #[allow(dead_code)]
    pub fn event_count(mut self, count: usize) -> Self {
        self.event_count = count;
        self
    }
    
    /// Set delay between events in milliseconds  
    #[allow(dead_code)]
    pub fn delay_ms(mut self, delay: u64) -> Self {
        self.delay_ms = delay;
        self
    }

    /// Set a custom ID for this runner
    #[allow(dead_code)]
    pub fn with_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    /// Add an analyzer to the chain
    #[allow(dead_code)]
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
    use crate::framework::analyzers::{ChunkMerger, FileLogger, OutputAnalyzer, Analyzer};
    use futures::stream::StreamExt;
    use std::fs;

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
    async fn test_fake_runner_with_chunk_merger() {
        let mut runner = FakeRunner::new()
            .with_id("test-chunk-merger".to_string())
            .event_count(2)
            .delay_ms(10)
            .add_analyzer(Box::new(ChunkMerger::new_with_timeout(5000))); // 5 second timeout

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        println!("Chunk Merger Test Results:");
        println!("Total events: {}", events.len());
        
        let ssl_events = events.iter().filter(|e| e.source == "ssl").count();
        let chunk_events = events.iter().filter(|e| e.source == "chunk_merger").count();
            
        println!("SSL events: {}", ssl_events);
        println!("Chunk events: {}", chunk_events);
        
        // Should have exactly 4 SSL events (2 pairs = 4 events)
        assert_eq!(ssl_events, 4, "Should have exactly 4 SSL events (2 request/response pairs)");
        
        // Since fake events don't contain chunked data, ChunkMerger just passes them through
        // So we expect 0 chunk_merger events and all original SSL events preserved
        assert_eq!(chunk_events, 0, "Should have no chunk_merger events since fake data isn't chunked");
        assert_eq!(events.len(), 4, "All original events should be preserved");
        
        // Verify all events are SSL events
        for event in &events {
            assert_eq!(event.source, "ssl", "All events should have ssl source");
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
        
        // Should have exactly 4 events (2 pairs = 4 events)
        assert_eq!(events.len(), 4, "Should have exactly 4 events (2 request/response pairs)");
        
        // Check if log file was created
        assert!(std::path::Path::new(test_log_file).exists(), "Log file should be created");
        
        let log_size = fs::metadata(test_log_file).unwrap().len();
        println!("Log file size: {} bytes", log_size);
        assert!(log_size > 0, "Log file should not be empty");
        
        // Read and check log contents
        let log_contents = fs::read_to_string(test_log_file).unwrap();
        let log_lines: Vec<&str> = log_contents.lines().collect();
        println!("Log file lines: {}", log_lines.len());
        assert_eq!(log_lines.len(), 4, "Log file should have exactly 4 lines (one per event)");
        
        // Clean up
        let _ = fs::remove_file(test_log_file);
    }

    #[tokio::test]
    async fn test_chunk_merger_basic() {
        let mut runner = FakeRunner::new()
            .event_count(3) // Explicitly set to 3 pairs for clear validation
            .add_analyzer(Box::new(ChunkMerger::new_with_timeout(5000)));

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        // Should have SSL events, but no chunk events since fake data isn't chunked
        let ssl_events = events.iter().filter(|e| e.source == "ssl").count();
        let chunk_events = events.iter().filter(|e| e.source == "chunk_merger").count();
            
        // Should have exactly 6 SSL events (3 pairs = 6 events)
        assert_eq!(ssl_events, 6, "Should have exactly 6 SSL events (3 request/response pairs)");
        assert_eq!(chunk_events, 0, "Should have no chunk_merger events since fake data isn't chunked");
        assert_eq!(events.len(), 6, "All original events should be preserved");
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
            .add_analyzer(Box::new(ChunkMerger::new_with_timeout(5000)))
            .add_analyzer(Box::new(FileLogger::new_with_options(test_log_file1, true, true).unwrap()))
            .add_analyzer(Box::new(FileLogger::new_with_options(test_log_file2, false, false).unwrap())) // Different settings
            .add_analyzer(Box::new(OutputAnalyzer::new()))
            .add_analyzer(Box::new(OutputAnalyzer::new())); // Different settings

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        println!("Multiple Analyzer Instances Test Results:");
        println!("Total events: {}", events.len());
        
        // Verify all events passed through multiple analyzers - should be 4 events (2 pairs) + HTTP analyzer events
        assert!(events.len() >= 4, "Should have at least 4 SSL events (2 request/response pairs)");
        
        // Count SSL events specifically
        let ssl_events = events.iter().filter(|e| e.source == "ssl").count();
        assert_eq!(ssl_events, 4, "Should have exactly 4 SSL events (2 request/response pairs)");
        
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
    async fn test_analyzer_chain_empty_stream() {
        // Test with zero events
        let mut runner = FakeRunner::new()
            .with_id("test-empty".to_string())
            .event_count(0) // No events
            .delay_ms(10)
            .add_analyzer(Box::new(ChunkMerger::new_with_timeout(5000)))
            .add_analyzer(Box::new(OutputAnalyzer::new()));

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        println!("Empty Stream Test Results:");
        println!("Events processed: {}", events.len());
        
        // Should handle empty stream gracefully
        assert_eq!(events.len(), 0, "Should have no events");
        
        println!("✅ Empty stream test completed!");
    }



    #[tokio::test] 
    async fn test_analyzer_chain_with_mixed_event_sources() {
        // Test analyzer chain with events from different sources
        let mut runner = FakeRunner::new()
            .with_id("test-mixed".to_string())
            .event_count(0) // Manual event generation
            .delay_ms(10);

        runner = runner.add_analyzer(Box::new(ChunkMerger::new_with_timeout(5000)));
        runner = runner.add_analyzer(Box::new(OutputAnalyzer::new()));

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
        let chunk_events = events.iter().filter(|e| e.source == "chunk_merger").count();
        
        println!("SSL events: {}", ssl_events);
        println!("Process events: {}", process_events);
        println!("Custom events: {}", custom_events);
        println!("Chunk events: {}", chunk_events);
        
        // Verify all events are preserved
        assert_eq!(ssl_events, 2, "Should have 2 SSL events");
        assert_eq!(process_events, 1, "Should have 1 process event");
        assert_eq!(custom_events, 1, "Should have 1 custom event");
        // Note: chunk_events might be 0 if no chunked data was processed
        
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
            .add_analyzer(Box::new(OutputAnalyzer::new()));

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        let max_events_seen = max_events_ref.load(Ordering::SeqCst);
        
        println!("Memory Cleanup Test Results:");
        println!("Total events processed: {}", events.len());
        println!("Max events seen at once: {}", max_events_seen);
        
        // Verify events were processed - should be exactly 50 events (25 pairs)
        assert_eq!(events.len(), 50, "Should have processed exactly 50 events (25 request/response pairs)");
        
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
            .add_analyzer(Box::new(ChunkMerger::new_with_timeout(10000))) // 10 second timeout
            .add_analyzer(Box::new(FileLogger::new_with_options(test_log_file, true, true).unwrap()))
            .add_analyzer(Box::new(OutputAnalyzer::new())); // Silent for test

        let start_time = Instant::now();
        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        let elapsed = start_time.elapsed();
        
        println!("Integration Test Results:");
        println!("Total processing time: {:?}", elapsed);
        println!("Total events processed: {}", events.len());
        
        // Analyze event distribution
        let ssl_events = events.iter().filter(|e| e.source == "ssl").count();
        let chunk_events = events.iter().filter(|e| e.source == "chunk_merger").count();
        
        println!("SSL events: {}", ssl_events);
        println!("Chunk events: {}", chunk_events);
        
        // Verify expected behavior
        assert_eq!(ssl_events, 20, "Should have 20 SSL events (10 request/response pairs)");
        assert_eq!(chunk_events, 0, "Should have no chunk_merger events since fake data isn't chunked");
        assert_eq!(events.len(), 20, "All original events should be preserved");
        
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
        
        // Verify ChunkMerger functionality - since fake events don't contain chunked data,
        // ChunkMerger should pass all events through unchanged
        for event in &events {
            assert_eq!(event.source, "ssl", "All events should remain as SSL events");
            assert!(event.data.get("data").is_some(), "Events should have data field");
            assert!(event.data.get("pid").is_some(), "Events should have pid field");
            assert!(event.data.get("function").is_some(), "Events should have function field");
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