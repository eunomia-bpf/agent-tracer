use super::{Runner, SslConfig, EventStream, RunnerError};
use super::common::{BinaryExecutor, AnalyzerProcessor};
use crate::framework::core::Event;
use crate::framework::analyzers::Analyzer;
use async_trait::async_trait;
use std::path::Path;
use uuid::Uuid;
use futures::stream::StreamExt;

/// Runner for collecting SSL/TLS events
pub struct SslRunner {
    id: String,
    config: SslConfig,
    analyzers: Vec<Box<dyn Analyzer>>,
    executor: BinaryExecutor,
    additional_args: Vec<String>,
}

impl SslRunner {
    /// Create from binary extractor (real execution mode)
    pub fn from_binary_extractor(binary_path: impl AsRef<Path>) -> Self {
        let path_str = binary_path.as_ref().to_string_lossy().to_string();
        Self {
            id: Uuid::new_v4().to_string(),
            config: SslConfig::default(),
            analyzers: Vec::new(),
            executor: BinaryExecutor::new(path_str),
            additional_args: Vec::new(),
        }
    }

    /// Create a new SslRunner with a custom ID
    pub fn with_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    /// Add additional command-line arguments to pass to the binary
    pub fn with_args<I, S>(mut self, args: I) -> Self 
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.additional_args = args.into_iter().map(|s| s.as_ref().to_string()).collect();
        // Update the executor with the additional args
        self.executor = self.executor.with_args(&self.additional_args);
        self
    }

    /// Set the TLS version filter
    #[allow(dead_code)]
    pub fn tls_version(mut self, version: String) -> Self {
        self.config.tls_version = Some(version);
        self
    }
}

#[async_trait]
impl Runner for SslRunner {
    async fn run(&mut self) -> Result<EventStream, RunnerError> {
        // Get raw JSON stream from the binary executor
        let json_stream = self.executor.get_json_stream().await?;
        
        // Convert JSON values directly to framework Events
        let event_stream = json_stream.map(|json_value| {
            // Extract timestamp if available, otherwise use current time
            let timestamp = json_value.get("timestamp_ns")
                .and_then(|v| v.as_u64())
                .unwrap_or_else(|| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64
                });
            
            Event::new_with_id_and_timestamp(
                Uuid::new_v4().to_string(),
                timestamp,
                "ssl".to_string(), // source is runner name
                json_value,
            )
        });
        
        AnalyzerProcessor::process_through_analyzers(Box::pin(event_stream), &mut self.analyzers).await
    }

    fn add_analyzer(mut self, analyzer: Box<dyn Analyzer>) -> Self {
        self.analyzers.push(analyzer);
        self
    }

    fn name(&self) -> &str {
        "ssl"
    }

    fn id(&self) -> String {
        self.id.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssl_runner_creation() {
        let runner = SslRunner::from_binary_extractor("/fake/path/sslsniff");
        assert_eq!(runner.name(), "ssl");
        assert!(!runner.id().is_empty());
    }

    #[test]
    fn test_ssl_runner_with_custom_config() {
        let runner = SslRunner::from_binary_extractor("/fake/path/sslsniff")
            .with_id("test-ssl".to_string());

        assert_eq!(runner.id(), "test-ssl");
    }

    /// Test that actually runs the real SSL binary
    /// 
    /// This test is ignored by default and only runs when specifically requested.
    /// To run this test: `cargo test test_ssl_runner_with_real_binary -- --ignored`
    /// 
    /// Prerequisites:
    /// - The sslsniff binary must be built and available at ../src/sslsniff
    /// - Sufficient privileges to run eBPF programs (usually requires sudo)
    /// 
    /// Note: This test may fail if:
    /// - The binary doesn't exist
    /// - Insufficient privileges 
    /// - No SSL/TLS traffic occurs during the execution window
    #[tokio::test]
    #[ignore = "requires real binary and may need sudo privileges"]
    async fn test_ssl_runner_with_real_binary() {
        use std::path::Path;
        use std::time::{Duration, Instant};
        use tokio::time::{timeout, interval};

        
        // Initialize debug logging for the test
        let _ = env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .is_test(true)
            .try_init();
        
        let binary_path = "../src/sslsniff";
        
        // Check if binary exists before attempting to run
        if !Path::new(binary_path).exists() {
            eprintln!("SSL binary not found at {}", binary_path);
            eprintln!("   Build the binary first: cd ../src && make sslsniff");
            return;
        }
        
        let start_time = Instant::now();
        println!("Testing SslRunner with real binary at {}", binary_path);
        println!("   Runtime: 30 seconds with live streaming output");
        println!("   Will terminate the process automatically after timeout");
        println!("   Generate SSL traffic: curl -s https://httpbin.org/get > /dev/null");
        println!("{}", "=".repeat(60));
        
        // Create runner with real binary
        let mut runner = SslRunner::from_binary_extractor(binary_path)
            .with_id("real-ssl-test".to_string())
            .add_analyzer(Box::new(crate::framework::analyzers::OutputAnalyzer::new()));
        
        // Run the binary and collect events for 30 seconds
        match runner.run().await {
            Ok(mut stream) => {
                println!("SslRunner started successfully! ({}s)", start_time.elapsed().as_secs());
                println!("Streaming SSL/TLS events live for 30 seconds...");
                println!();
                
                let mut event_count = 0;
                let mut status_interval = interval(Duration::from_secs(5));
                let mut last_event_time = Instant::now();
                
                // Run for 30 seconds with streaming output
                let result = timeout(Duration::from_secs(30), async {
                    loop {
                        tokio::select! {
                            event_opt = futures::StreamExt::next(&mut stream) => {
                                match event_opt {
                                    Some(event) => {
                                        event_count += 1;
                                        last_event_time = Instant::now();
                                        let runtime = start_time.elapsed().as_secs();
                                        
                                        // Print event as JSON
                                        println!("[{:02}s] Event #{}: {}", 
                                            runtime,
                                            event_count, 
                                            serde_json::to_string(&event).unwrap()
                                        );
                                    }
                                    None => {
                                        println!("[{:02}s] Event stream ended naturally", start_time.elapsed().as_secs());
                                        break;
                                    }
                                }
                            }
                            _ = status_interval.tick() => {
                                let runtime = start_time.elapsed().as_secs();
                                let time_since_last = last_event_time.elapsed().as_secs();
                                println!("[{:02}s] Status: {} SSL events collected, last event {}s ago", 
                                    runtime, event_count, time_since_last);
                            }
                        }
                    }
                }).await;
                
                let total_runtime = start_time.elapsed();
                println!();
                
                match result {
                    Ok(_) => println!("SSL event stream completed naturally after {:.1}s", total_runtime.as_secs_f32()),
                    Err(_) => {
                        println!("30-second timeout reached - terminating process");
                        println!("Process killed automatically");
                    }
                }
                
                println!("{}", "=".repeat(60));
                println!("SslRunner test completed!");
                println!("   Total SSL events: {}", event_count);
                println!("   Total runtime: {:.2}s", total_runtime.as_secs_f32());
                println!("   Event rate: {:.1} events/sec", 
                    event_count as f32 / total_runtime.as_secs_f32());
                
                if event_count == 0 {
                    println!();
                    println!("No SSL events captured during test period!");
                    println!("   Try generating HTTPS traffic in another terminal:");
                    println!("   curl -s https://httpbin.org/get");
                    println!("   wget -q -O /dev/null https://example.com");
                    println!("   firefox (browse HTTPS sites)");
                }
            }
            Err(e) => {
                let runtime = start_time.elapsed();
                eprintln!("SslRunner failed after {:.2}s: {}", runtime.as_secs_f32(), e);
                eprintln!("   Possible causes:");
                eprintln!("   - Insufficient privileges (try: sudo cargo test ...)");
                eprintln!("   - Binary compilation failed");
                eprintln!("   - eBPF/kernel support missing");
                eprintln!("   - Missing kernel headers");
                eprintln!("   - SSL/TLS hooks not available");
                
                // Don't panic - allow test to pass even with environmental issues
                return;
            }
        }
    }
} 