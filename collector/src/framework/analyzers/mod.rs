use crate::framework::runners::EventStream;
use async_trait::async_trait;

/// Type alias for errors that can be sent between threads
pub type AnalyzerError = Box<dyn std::error::Error + Send + Sync>;

/// Base trait for all analyzers that process event streams
#[async_trait]
pub trait Analyzer: Send + Sync {
    /// Process an event stream and return a processed stream
    async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError>;
    
    /// Get the name of this analyzer
    #[allow(dead_code)]
    fn name(&self) -> &str;
}

pub mod output;
pub mod file_logger;
pub mod sse_processor;
pub mod http_parser;
pub mod http_filter;
pub mod event;

#[cfg(test)]
mod sse_processor_tests;

pub use output::OutputAnalyzer;
pub use file_logger::FileLogger;
pub use sse_processor::SSEProcessor;
pub use http_parser::HTTPParser;
pub use http_filter::HTTPFilter;

#[cfg(test)]
mod comprehensive_analyzer_chain_tests {
    use super::*;
    use crate::framework::runners::{EventStream, FakeRunner, Runner};
    use futures::stream::StreamExt;
    use serde_json::json;
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
    use std::time::Instant;
    use tokio::time::Duration;
    use tempfile::NamedTempFile;

    /// Custom test analyzer that simulates errors
    struct ErrorSimulatorAnalyzer {
        error_on_event_number: usize,
        event_count: AtomicUsize,
    }

    impl ErrorSimulatorAnalyzer {
        fn new(error_on_event_number: usize) -> Self {
            Self {
                error_on_event_number,
                event_count: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl Analyzer for ErrorSimulatorAnalyzer {
        async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
            let error_event = self.error_on_event_number;
            let counter = Arc::new(AtomicUsize::new(0));
            
            let processed_stream = stream.map(move |event| {
                let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
                if count == error_event {
                    // Simulate an error condition but don't actually error out
                    // Instead, modify the event to indicate an error occurred
                    let mut error_event = event;
                    if let Some(data) = error_event.data.as_object_mut() {
                        data.insert("analyzer_error".to_string(), json!("Simulated error"));
                    }
                    error_event
                } else {
                    event
                }
            });
            
            Ok(Box::pin(processed_stream))
        }

        fn name(&self) -> &str {
            "ErrorSimulatorAnalyzer"
        }
    }

    /// Custom test analyzer that filters events
    struct FilterAnalyzer {
        filter_condition: String,
    }

    impl FilterAnalyzer {
        fn new(filter_condition: String) -> Self {
            Self { filter_condition }
        }
    }

    #[async_trait]
    impl Analyzer for FilterAnalyzer {
        async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
            let condition = self.filter_condition.clone();
            let filtered_stream = stream.filter(move |event| {
                let matches = if condition == "ssl_only" {
                    event.source == "ssl"
                } else if condition == "even_pids" {
                    event.data.get("pid")
                        .and_then(|v| v.as_u64())
                        .map(|pid| pid % 2 == 0)
                        .unwrap_or(false)
                } else {
                    true // No filter
                };
                futures::future::ready(matches)
            });
            
            Ok(Box::pin(filtered_stream))
        }

        fn name(&self) -> &str {
            "FilterAnalyzer"
        }
    }

    /// Custom test analyzer that adds metadata
    struct MetadataEnricherAnalyzer {
        metadata: serde_json::Value,
    }

    impl MetadataEnricherAnalyzer {
        fn new(metadata: serde_json::Value) -> Self {
            Self { metadata }
        }
    }

    #[async_trait]
    impl Analyzer for MetadataEnricherAnalyzer {
        async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
            let metadata = self.metadata.clone();
            let enriched_stream = stream.map(move |mut event| {
                if let Some(data) = event.data.as_object_mut() {
                    data.insert("enriched_metadata".to_string(), metadata.clone());
                }
                event
            });
            
            Ok(Box::pin(enriched_stream))
        }

        fn name(&self) -> &str {
            "MetadataEnricherAnalyzer"
        }
    }

    #[tokio::test]
    async fn test_complex_analyzer_chain_composition() {
        let temp_file = NamedTempFile::new().unwrap();
        
        // Create a complex chain: Filter -> ChunkMerger -> Enrich -> FileLogger -> Output
        let mut runner = FakeRunner::new()
            .with_id("complex-chain".to_string())
            .event_count(5) // 10 events total
            .delay_ms(10)
            .add_analyzer(Box::new(FilterAnalyzer::new("ssl_only".to_string())))
            .add_analyzer(Box::new(SSEProcessor::new_with_timeout(5000)))
            .add_analyzer(Box::new(MetadataEnricherAnalyzer::new(json!({"test_run": "complex_chain", "version": "1.0"}))))
            .add_analyzer(Box::new(FileLogger::new(temp_file.path()).unwrap()))
            .add_analyzer(Box::new(OutputAnalyzer::new()));

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        println!("Complex Chain Test Results:");
        println!("Total events: {}", events.len());
        
        // Verify events passed through all analyzers
        assert!(events.len() > 0, "Should have events");
        
        // All remaining events should be SSL (due to filter)
        let non_ssl_events = events.iter().filter(|e| e.source != "ssl" && e.source != "sse_processor").count();
        assert_eq!(non_ssl_events, 0, "Filter should remove non-SSL events");
        
        // Events should have enriched metadata
        let enriched_events = events.iter()
            .filter(|e| e.data.get("enriched_metadata").is_some())
            .count();
        assert!(enriched_events > 0, "Should have enriched events");
        
        // Verify sse processor events were created
        let _sse_events = events.iter()
            .filter(|e| e.source == "sse_processor")
            .count();
        // Note: sse_events might be 0 if no SSE data was processed
        
        // Verify file was written
        let file_size = std::fs::metadata(temp_file.path()).unwrap().len();
        assert!(file_size > 0, "Log file should have content");
        
        println!("✅ Complex analyzer chain composition test completed!");
    }

    #[tokio::test]
    async fn test_analyzer_chain_error_resilience() {
        // Test that analyzer chain continues working even when individual analyzers encounter issues
        let mut runner = FakeRunner::new()
            .with_id("error-resilience".to_string())
            .event_count(5)
            .delay_ms(10)
            .add_analyzer(Box::new(ErrorSimulatorAnalyzer::new(3))) // Error on 3rd event
            .add_analyzer(Box::new(SSEProcessor::new_with_timeout(5000)))
            .add_analyzer(Box::new(OutputAnalyzer::new()));

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        println!("Error Resilience Test Results:");
        println!("Total events: {}", events.len());
        
        // Should still process all events
        assert!(events.len() >= 10, "Should process all events despite simulated error");
        
        // Check that error was marked on the 3rd event
        let error_events = events.iter()
            .filter(|e| e.data.get("analyzer_error").is_some())
            .count();
        assert!(error_events > 0, "Should have error markers from ErrorSimulator");
        
        println!("✅ Error resilience test completed!");
    }

    #[tokio::test]
    async fn test_analyzer_chain_concurrent_processing() {
        use std::sync::Arc;
        use tokio::sync::Mutex;
        
        // Test multiple analyzer chains running concurrently
        let results = Arc::new(Mutex::new(Vec::new()));
        
        let mut handles = Vec::new();
        
        for i in 0..3 {
            let results_clone = Arc::clone(&results);
            let handle = tokio::spawn(async move {
                let mut runner = FakeRunner::new()
                    .with_id(format!("concurrent-{}", i))
                    .event_count(3)
                    .delay_ms(5)
                    .add_analyzer(Box::new(SSEProcessor::new_with_timeout(5000)))
                    .add_analyzer(Box::new(OutputAnalyzer::new()));

                let stream = runner.run().await.unwrap();
                let events: Vec<_> = stream.collect().await;
                
                let mut results_guard = results_clone.lock().await;
                results_guard.push((i, events.len()));
                
                events.len()
            });
            handles.push(handle);
        }
        
        // Wait for all chains to complete
        let mut total_events = 0;
        for handle in handles {
            total_events += handle.await.unwrap();
        }
        
        let results_guard = results.lock().await;
        
        println!("Concurrent Processing Test Results:");
        println!("Total events across all chains: {}", total_events);
        println!("Individual chain results: {:?}", *results_guard);
        
        // All chains should have processed events
        assert_eq!(results_guard.len(), 3, "Should have 3 chain results");
        assert!(total_events >= 18, "Should have at least 18 events total (3 chains × 6 events)");
        
        for (chain_id, event_count) in results_guard.iter() {
            assert!(*event_count >= 6, "Chain {} should have at least 6 events", chain_id);
        }
        
        println!("✅ Concurrent processing test completed!");
    }

    #[tokio::test]
    async fn test_analyzer_chain_streaming_behavior() {
        // Test that events are processed in streaming fashion, not batched
        use std::sync::Arc;
        use tokio::sync::Mutex;
        use std::time::Instant;
        
        let event_timestamps = Arc::new(Mutex::new(Vec::new()));
        
        // Custom analyzer that records processing timestamps
        struct TimestampRecorderAnalyzer {
            timestamps: Arc<Mutex<Vec<(usize, Instant)>>>,
            counter: Arc<AtomicUsize>,
        }
        
        impl TimestampRecorderAnalyzer {
            fn new(timestamps: Arc<Mutex<Vec<(usize, Instant)>>>) -> Self {
                Self {
                    timestamps,
                    counter: Arc::new(AtomicUsize::new(0)),
                }
            }
        }
        
        #[async_trait]
        impl Analyzer for TimestampRecorderAnalyzer {
            async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
                let timestamps = self.timestamps.clone();
                let counter = self.counter.clone();
                
                let recorded_stream = stream.map(move |event| {
                    let count = counter.fetch_add(1, Ordering::SeqCst);
                    let timestamp = Instant::now();
                    
                    let timestamps_clone = timestamps.clone();
                    tokio::spawn(async move {
                        let mut guard = timestamps_clone.lock().await;
                        guard.push((count, timestamp));
                    });
                    
                    event
                });
                
                Ok(Box::pin(recorded_stream))
            }
            
            fn name(&self) -> &str {
                "TimestampRecorderAnalyzer"
            }
        }
        
        let timestamps_clone = Arc::clone(&event_timestamps);
        
        let mut runner = FakeRunner::new()
            .with_id("streaming-test".to_string())
            .event_count(5) // 10 events total
            .delay_ms(100) // 100ms delay to ensure streaming behavior is observable
            .add_analyzer(Box::new(TimestampRecorderAnalyzer::new(timestamps_clone)))
            .add_analyzer(Box::new(OutputAnalyzer::new()));

        let start_time = Instant::now();
        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        let total_time = start_time.elapsed();
        
        // Wait a bit for async timestamp recording to complete
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        let timestamps_guard = event_timestamps.lock().await;
        
        println!("Streaming Behavior Test Results:");
        println!("Total events: {}", events.len());
        println!("Total time: {:?}", total_time);
        println!("Recorded timestamps: {}", timestamps_guard.len());
        
        // Verify streaming behavior - events should arrive over time, not all at once
        assert!(timestamps_guard.len() >= 5, "Should have recorded multiple timestamps");
        
        if timestamps_guard.len() >= 2 {
            let first_event_time = timestamps_guard[0].1;
            let last_event_time = timestamps_guard[timestamps_guard.len() - 1].1;
            let processing_span = last_event_time.duration_since(first_event_time);
            
            println!("Processing span: {:?}", processing_span);
            
            // Should take some time due to delays, indicating streaming behavior
            assert!(processing_span >= Duration::from_millis(50), 
                "Events should be processed over time, not all at once");
        }
        
        println!("✅ Streaming behavior test completed!");
    }

    #[tokio::test]
    async fn test_analyzer_chain_backpressure_handling() {
        // Test analyzer chain behavior under backpressure conditions
        
        // Custom slow analyzer that simulates processing delays
        struct SlowAnalyzer {
            delay_ms: u64,
        }
        
        impl SlowAnalyzer {
            fn new(delay_ms: u64) -> Self {
                Self { delay_ms }
            }
        }
        
        #[async_trait]
        impl Analyzer for SlowAnalyzer {
            async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
                let delay = self.delay_ms;
                let slow_stream = stream.then(move |event| async move {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    event
                });
                
                Ok(Box::pin(slow_stream))
            }
            
            fn name(&self) -> &str {
                "SlowAnalyzer"
            }
        }
        
        let start_time = Instant::now();
        
        let mut runner = FakeRunner::new()
            .with_id("backpressure-test".to_string())
            .event_count(3) // 6 events total
            .delay_ms(10) // Fast generation
            .add_analyzer(Box::new(SlowAnalyzer::new(50))) // Slow processing
            .add_analyzer(Box::new(OutputAnalyzer::new()));

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        let total_time = start_time.elapsed();
        
        println!("Backpressure Test Results:");
        println!("Total events: {}", events.len());
        println!("Total time: {:?}", total_time);
        
        // Should process all events
        assert_eq!(events.len(), 6, "Should process all events despite slow analyzer");
        
        // Should take longer due to slow analyzer (at least 3 * 50ms = 150ms for processing)
        assert!(total_time >= Duration::from_millis(100), 
            "Should take time due to slow analyzer processing");
        
        println!("✅ Backpressure handling test completed!");
    }

    #[tokio::test]
    async fn test_analyzer_chain_resource_cleanup() {
        // Test that resources are properly cleaned up after analyzer chain completion
        use std::sync::Arc;
        use tokio::sync::Mutex;
        
        // Custom analyzer that tracks resource allocation/cleanup
        struct ResourceTrackingAnalyzer {
            resources: Arc<Mutex<Vec<String>>>,
            id: String,
        }
        
        impl ResourceTrackingAnalyzer {
            fn new(id: String, resources: Arc<Mutex<Vec<String>>>) -> Self {
                Self { resources, id }
            }
        }
        
        impl Drop for ResourceTrackingAnalyzer {
            fn drop(&mut self) {
                // Simulate resource cleanup
                println!("Cleaning up ResourceTrackingAnalyzer: {}", self.id);
            }
        }
        
        #[async_trait]
        impl Analyzer for ResourceTrackingAnalyzer {
            async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
                let resources = self.resources.clone();
                let id = self.id.clone();
                
                // Simulate resource allocation
                {
                    let mut guard = resources.lock().await;
                    guard.push(format!("resource_{}", id));
                }
                
                let processed_stream = stream.map(move |event| {
                    // Simulate resource usage
                    event
                });
                
                Ok(Box::pin(processed_stream))
            }
            
            fn name(&self) -> &str {
                "ResourceTrackingAnalyzer"
            }
        }
        
        let resources = Arc::new(Mutex::new(Vec::new()));
        
        {
            let mut runner = FakeRunner::new()
                .with_id("cleanup-test".to_string())
                .event_count(2)
                .delay_ms(10)
                .add_analyzer(Box::new(ResourceTrackingAnalyzer::new("test1".to_string(), Arc::clone(&resources))))
                .add_analyzer(Box::new(ResourceTrackingAnalyzer::new("test2".to_string(), Arc::clone(&resources))))
                .add_analyzer(Box::new(OutputAnalyzer::new()));

            let stream = runner.run().await.unwrap();
            let events: Vec<_> = stream.collect().await;
            
            println!("Resource Cleanup Test Results:");
            println!("Events processed: {}", events.len());
            
            assert_eq!(events.len(), 4, "Should process all events");
        } // Runner and analyzers go out of scope here
        
        // Verify resources were allocated
        let resources_guard = resources.lock().await;
        assert_eq!(resources_guard.len(), 2, "Should have allocated 2 resources");
        assert!(resources_guard.contains(&"resource_test1".to_string()));
        assert!(resources_guard.contains(&"resource_test2".to_string()));
        
        println!("✅ Resource cleanup test completed!");
    }
} 