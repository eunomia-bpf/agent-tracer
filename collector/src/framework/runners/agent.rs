use super::{Runner, EventStream, RunnerError};
use crate::framework::analyzers::Analyzer;
use async_trait::async_trait;
use futures::stream::select_all;

/// AgentRunner composes multiple runners into a single unified stream
/// with optional global analyzers applied to the merged stream
pub struct AgentRunner {
    runners: Vec<Box<dyn Runner>>,
    analyzers: Vec<Box<dyn Analyzer>>,
    name: String,
}

impl AgentRunner {
    /// Create a new AgentRunner with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            runners: Vec::new(),
            analyzers: Vec::new(),
            name: name.into(),
        }
    }
    
    /// Add a pre-configured runner with its analyzer chain
    pub fn add_runner(mut self, runner: Box<dyn Runner>) -> Self {
        self.runners.push(runner);
        self
    }
    
    /// Add multiple runners at once
    pub fn add_runners(mut self, runners: Vec<Box<dyn Runner>>) -> Self {
        self.runners.extend(runners);
        self
    }
    
    /// Add analyzer that will be applied to the merged stream
    pub fn add_global_analyzer(mut self, analyzer: Box<dyn Analyzer>) -> Self {
        self.analyzers.push(analyzer);
        self
    }
    
    /// Get the number of configured runners
    pub fn runner_count(&self) -> usize {
        self.runners.len()
    }
    
    /// Get the number of configured global analyzers
    pub fn analyzer_count(&self) -> usize {
        self.analyzers.len()
    }
}

#[async_trait]
impl Runner for AgentRunner {
    async fn run(&mut self) -> Result<EventStream, RunnerError> {
        if self.runners.is_empty() {
            return Err("No runners configured for AgentRunner".into());
        }
        
        // Start all runners concurrently and collect their streams
        let mut streams = Vec::new();
        for runner in &mut self.runners {
            let stream = runner.run().await?;
            streams.push(stream);
        }
        
        // Merge all streams into a single stream
        let merged_stream = select_all(streams);
        
        // Apply global analyzers to the merged stream
        let mut final_stream = Box::pin(merged_stream) as EventStream;
        for analyzer in &mut self.analyzers {
            final_stream = analyzer.process(final_stream).await
                .map_err(|e| format!("Global analyzer error: {}", e))?;
        }
        
        Ok(final_stream)
    }
    
    fn add_analyzer(mut self, analyzer: Box<dyn Analyzer>) -> Self {
        self.analyzers.push(analyzer);
        self
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn id(&self) -> String {
        format!("agent-{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::runners::FakeRunner;
    use crate::framework::analyzers::{OutputAnalyzer, SSEProcessor, HTTPParser, FileLogger};
    use futures::stream::StreamExt;
    use tempfile::NamedTempFile;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_agent_runner_basic_composition() {
        let fake_runner1 = FakeRunner::new()
            .event_count(2)
            .delay_ms(10)
            .add_analyzer(Box::new(OutputAnalyzer::new()));
        
        let fake_runner2 = FakeRunner::new()
            .event_count(3)
            .delay_ms(15)
            .add_analyzer(Box::new(OutputAnalyzer::new()));
        
        let mut agent = AgentRunner::new("test-agent")
            .add_runner(Box::new(fake_runner1))
            .add_runner(Box::new(fake_runner2));
        
        assert_eq!(agent.runner_count(), 2);
        assert_eq!(agent.analyzer_count(), 0);
        
        let stream = agent.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        // Should have events from both runners (4 + 6 = 10 total)
        // FakeRunner generates 2 events per event_count (request + response pairs)
        assert_eq!(events.len(), 10);
        
        // Verify events come from SSL source (FakeRunner only generates SSL events)
        let ssl_events = events.iter().filter(|e| e.source == "ssl").count();
        assert_eq!(ssl_events, 10); // All events are SSL events from FakeRunner
    }
    
    #[tokio::test]
    async fn test_agent_runner_with_global_analyzers() {
        let temp_file = NamedTempFile::new().unwrap();
        
        let fake_runner = FakeRunner::new()
            .event_count(2)
            .delay_ms(10);
        
        let mut agent = AgentRunner::new("test-with-analyzers")
            .add_runner(Box::new(fake_runner))
            .add_global_analyzer(Box::new(FileLogger::new(temp_file.path()).unwrap()))
            .add_global_analyzer(Box::new(OutputAnalyzer::new()));
        
        assert_eq!(agent.runner_count(), 1);
        assert_eq!(agent.analyzer_count(), 2);
        
        let stream = agent.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        // Should have events from runner (2 events per event_count)
        assert_eq!(events.len(), 4);
        
        // Verify file was written by FileLogger
        let file_size = std::fs::metadata(temp_file.path()).unwrap().len();
        assert!(file_size > 0, "Log file should have content");
    }
    
    #[tokio::test]
    async fn test_agent_runner_multiple_runners_with_analyzers() {
        let fake_runner1 = FakeRunner::new()
            .event_count(1)
            .delay_ms(10)
            .add_analyzer(Box::new(SSEProcessor::new_with_timeout(5000)));
        
        let fake_runner2 = FakeRunner::new()
            .event_count(1)
            .delay_ms(10)
            .add_analyzer(Box::new(HTTPParser::new()));
        
        let mut agent = AgentRunner::new("complex-agent")
            .add_runner(Box::new(fake_runner1))
            .add_runner(Box::new(fake_runner2))
            .add_global_analyzer(Box::new(OutputAnalyzer::new()));
        
        let stream = agent.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        
        // Should have events from both runners (2 events per event_count each)
        assert!(events.len() >= 4, "Should have at least 4 events");
        
        // Check for events from different sources (SSL from FakeRunner, potentially processed by analyzers)
        let sources: std::collections::HashSet<_> = events.iter().map(|e| &e.source).collect();
        assert!(sources.len() >= 1, "Should have events from at least one source");
    }
    
    #[tokio::test]
    async fn test_agent_runner_empty_runners() {
        let mut agent = AgentRunner::new("empty-agent");
        
        assert_eq!(agent.runner_count(), 0);
        
        let result = agent.run().await;
        assert!(result.is_err(), "Should fail with no runners configured");
        
        if let Err(e) = result {
            let error_msg = format!("{}", e);
            assert!(error_msg.contains("No runners configured"), "Should have descriptive error message");
        }
    }
    
    #[tokio::test]
    async fn test_agent_runner_streaming_behavior() {
        use std::sync::Arc;
        use tokio::sync::Mutex;
        use std::time::Instant;
        
        let event_timestamps = Arc::new(Mutex::new(Vec::new()));
        
        // Custom analyzer that records timestamps
        struct TimestampRecorder {
            timestamps: Arc<Mutex<Vec<Instant>>>,
        }
        
        impl TimestampRecorder {
            fn new(timestamps: Arc<Mutex<Vec<Instant>>>) -> Self {
                Self { timestamps }
            }
        }
        
        #[async_trait]
        impl Analyzer for TimestampRecorder {
            async fn process(&mut self, stream: EventStream) -> Result<EventStream, Box<dyn std::error::Error + Send + Sync>> {
                let timestamps = self.timestamps.clone();
                let recorded_stream = stream.map(move |event| {
                    let timestamps_clone = timestamps.clone();
                    tokio::spawn(async move {
                        let mut guard = timestamps_clone.lock().await;
                        guard.push(Instant::now());
                    });
                    event
                });
                Ok(Box::pin(recorded_stream))
            }
            
            fn name(&self) -> &str {
                "TimestampRecorder"
            }
        }
        
        let fake_runner = FakeRunner::new()
            .event_count(3)
            .delay_ms(50); // Longer delay to ensure streaming behavior
        
        let mut agent = AgentRunner::new("streaming-test")
            .add_runner(Box::new(fake_runner))
            .add_global_analyzer(Box::new(TimestampRecorder::new(Arc::clone(&event_timestamps))))
            .add_global_analyzer(Box::new(OutputAnalyzer::new()));
        
        let start_time = Instant::now();
        let stream = agent.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;
        let total_time = start_time.elapsed();
        
        // Wait for async timestamp recording to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        assert_eq!(events.len(), 6); // 3 event_count × 2 events per count (request + response)
        
        let timestamps_guard = event_timestamps.lock().await;
        assert!(timestamps_guard.len() >= 3, "Should have recorded multiple timestamps");
        
        // Should take some time due to delays, indicating streaming behavior
        assert!(total_time >= Duration::from_millis(100), "Should demonstrate streaming behavior");
    }
    
    #[tokio::test]
    async fn test_agent_runner_concurrent_processing() {
        // Test multiple agent runners running concurrently
        let mut handles = Vec::new();
        
        for i in 0..3 {
            let handle = tokio::spawn(async move {
                let fake_runner = FakeRunner::new()
                    .event_count(2)
                    .delay_ms(10)
                    .add_analyzer(Box::new(OutputAnalyzer::new()));
                
                let mut agent = AgentRunner::new(format!("concurrent-agent-{}", i))
                    .add_runner(Box::new(fake_runner));
                
                let stream = agent.run().await.unwrap();
                let events: Vec<_> = stream.collect().await;
                
                (i, events.len())
            });
            handles.push(handle);
        }
        
        // Wait for all agents to complete
        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await.unwrap());
        }
        
        // All agents should have processed events
        assert_eq!(results.len(), 3);
        for (agent_id, event_count) in results {
            assert_eq!(event_count, 4, "Agent {} should have 4 events (2 event_count × 2 events)", agent_id);
        }
    }
    
    #[tokio::test]
    async fn test_agent_runner_error_handling() {
        // Test that agent runner handles runner failures gracefully
        
        // Create a mock runner that fails
        struct FailingRunner;
        
        #[async_trait]
        impl Runner for FailingRunner {
            async fn run(&mut self) -> Result<EventStream, RunnerError> {
                Err("Simulated runner failure".into())
            }
            
            fn add_analyzer(self, _analyzer: Box<dyn Analyzer>) -> Self {
                self
            }
            
            fn name(&self) -> &str {
                "FailingRunner"
            }
            
            fn id(&self) -> String {
                "failing-runner".to_string()
            }
        }
        
        let mut agent = AgentRunner::new("error-test")
            .add_runner(Box::new(FailingRunner));
        
        let result = agent.run().await;
        assert!(result.is_err(), "Should propagate runner error");
        
        if let Err(e) = result {
            let error_msg = format!("{}", e);
            assert!(error_msg.contains("Simulated runner failure"), "Should contain original error message");
        }
    }
    
    #[tokio::test]
    async fn test_agent_runner_with_timeout() {
        // Test agent runner with timeout to ensure it doesn't hang
        let fake_runner = FakeRunner::new()
            .event_count(5)
            .delay_ms(10);
        
        let mut agent = AgentRunner::new("timeout-test")
            .add_runner(Box::new(fake_runner))
            .add_global_analyzer(Box::new(OutputAnalyzer::new()));
        
        let result = timeout(Duration::from_secs(5), async {
            let stream = agent.run().await.unwrap();
            let events: Vec<_> = stream.collect().await;
            events.len()
        }).await;
        
        assert!(result.is_ok(), "AgentRunner should complete within timeout");
        assert_eq!(result.unwrap(), 10, "Should process all events (5 event_count × 2 events)");
    }
    
    #[tokio::test]
    async fn test_agent_runner_name_and_id() {
        let agent = AgentRunner::new("my-test-agent");
        
        assert_eq!(agent.name(), "my-test-agent");
        assert_eq!(agent.id(), "agent-my-test-agent");
    }
    
    #[tokio::test]
    async fn test_agent_runner_fluent_interface() {
        // Test that the fluent interface works correctly
        let fake_runner1 = FakeRunner::new().event_count(1).delay_ms(10);
        let fake_runner2 = FakeRunner::new().event_count(1).delay_ms(10);
        
        let agent = AgentRunner::new("fluent-test")
            .add_runner(Box::new(fake_runner1))
            .add_runner(Box::new(fake_runner2))
            .add_global_analyzer(Box::new(OutputAnalyzer::new()))
            .add_analyzer(Box::new(OutputAnalyzer::new())); // Test inherited add_analyzer
        
        assert_eq!(agent.runner_count(), 2);
        assert_eq!(agent.analyzer_count(), 2); // Both global analyzers should be present
    }
}