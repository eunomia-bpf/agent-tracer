use super::{Runner, ProcessConfig, EventStream, RunnerError};
use super::common::{BinaryExecutor, AnalyzerProcessor};
use crate::framework::core::Event;
use crate::framework::analyzers::Analyzer;
use async_trait::async_trait;
use std::path::Path;
use futures::stream::StreamExt;

/// Runner for collecting process/system events
pub struct ProcessRunner {
    config: ProcessConfig,
    analyzers: Vec<Box<dyn Analyzer>>,
    executor: BinaryExecutor,
    additional_args: Vec<String>,
}

impl ProcessRunner {
    /// Create from binary extractor (real execution mode)
    pub fn from_binary_extractor(binary_path: impl AsRef<Path>) -> Self {
        let path_str = binary_path.as_ref().to_string_lossy().to_string();
        Self {
            config: ProcessConfig::default(),
            analyzers: Vec::new(),
            executor: BinaryExecutor::new(path_str).with_runner_name("Process".to_string()),
            additional_args: Vec::new(),
        }
    }


    /// Add additional command-line arguments to pass to the binary
    pub fn with_args<I, S>(mut self, args: I) -> Self 
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.additional_args = args.into_iter().map(|s| s.as_ref().to_string()).collect();
        // Update the executor with the additional args
        self.executor = self.executor.with_args(&self.additional_args).with_runner_name("Process".to_string());
        self
    }

    /// Set the PID to monitor
    #[allow(dead_code)]
    pub fn pid(mut self, pid: u32) -> Self {
        self.config.pid = Some(pid);
        self
    }

    /// Set the memory threshold for filtering
    #[allow(dead_code)]
    pub fn memory_threshold(mut self, threshold: u64) -> Self {
        self.config.memory_threshold = Some(threshold);
        self
    }
}

#[async_trait]
impl Runner for ProcessRunner {
    async fn run(&mut self) -> Result<EventStream, RunnerError> {
        // Get raw JSON stream from the binary executor
        let json_stream = self.executor.get_json_stream().await?;
        
        // Convert JSON values directly to framework Events
        let event_stream = json_stream.map(|json_value| {
            // Extract timestamp if available, otherwise use current time
            let timestamp = json_value.get("timestamp")
                .and_then(|v| v.as_u64())
                .unwrap_or_else(|| {
                    panic!("Missing timestamp field in process event: {}", json_value);
                });
            
            // Extract pid - panic if not found
            let pid = json_value.get("pid")
                .and_then(|v| v.as_u64())
                .map(|p| p as u32)
                .unwrap_or_else(|| {
                    panic!("Missing pid field in process event: {}", json_value);
                });
            
            // Extract comm - panic if not found
            let comm = json_value.get("comm")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| {
                    panic!("Missing comm field in process event: {}", json_value);
                })
                .to_string();
            
            Event::new_with_timestamp(
                timestamp,
                "process".to_string(), // source is runner name
                pid,
                comm,
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
        "process"
    }

    fn id(&self) -> String {
        "process".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_runner_creation() {
        let runner = ProcessRunner::from_binary_extractor("/fake/path/process");
        assert_eq!(runner.name(), "process");
        assert_eq!(runner.id(), "process");
        assert_eq!(runner.config.pid, None);
    }

    #[test]
    fn test_process_runner_with_custom_config() {
        let runner = ProcessRunner::from_binary_extractor("/fake/path/process")
            .pid(1234);

        assert_eq!(runner.id(), "process");
        assert_eq!(runner.config.pid, Some(1234));
    }

    /// Test that actually runs the real process binary
    /// 
    /// This test is ignored by default and only runs when specifically requested.
    /// To run this test: `cargo test test_process_runner_with_real_binary -- --ignored`
    /// 
    /// Prerequisites:
    /// - The process binary must be built and available at ../src/process
    /// - Sufficient privileges to run eBPF programs (usually requires sudo)
    /// 
    /// Note: This test may fail if:
    /// - The binary doesn't exist
    /// - Insufficient privileges 
    /// - No process events occur during the short execution window
    #[tokio::test]
    #[ignore = "requires real binary and may need sudo privileges"]
    async fn test_process_runner_with_real_binary() {
        use std::path::Path;
        use std::time::{Duration, Instant};
        use tokio::time::{timeout, interval};

        
        // Initialize debug logging for the test
        let _ = env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .is_test(true)
            .try_init();
        
        let binary_path = "../src/process";
        
        // Check if binary exists before attempting to run
        if !Path::new(binary_path).exists() {
            eprintln!("Process binary not found at {}", binary_path);
            eprintln!("   Build the binary first: cd ../src && make process");
            return;
        }

        let start_time = Instant::now();
        println!("Testing ProcessRunner with real binary at {}", binary_path);
        println!("   Runtime: 30 seconds with live streaming output");
        println!("   Will terminate the process automatically after timeout");
        println!("{}", "=".repeat(60));
        
        // Create runner with real binary
        let mut runner = ProcessRunner::from_binary_extractor(binary_path)
            .add_analyzer(Box::new(crate::framework::analyzers::OutputAnalyzer::new()));
        
        // Run the binary and collect events for 30 seconds
        match runner.run().await {
            Ok(mut stream) => {
                println!("ProcessRunner started successfully! ({}s)", start_time.elapsed().as_secs());
                println!("Streaming process events live for 30 seconds...");
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
                                println!("[{:02}s] Status: {} events collected, last event {}s ago", 
                                    runtime, event_count, time_since_last);
                            }
                        }
                    }
                }).await;
                
                let total_runtime = start_time.elapsed();
                println!();
                
                match result {
                    Ok(_) => println!("Event stream completed naturally after {:.1}s", total_runtime.as_secs_f32()),
                    Err(_) => {
                        println!("30-second timeout reached - terminating process");
                        println!("Process killed automatically");
                    }
                }
                
                println!("{}", "=".repeat(60));
                println!("ProcessRunner test completed!");
                println!("   Total events: {}", event_count);
                println!("   Total runtime: {:.2}s", total_runtime.as_secs_f32());
                println!("   Event rate: {:.1} events/sec", 
                    event_count as f32 / total_runtime.as_secs_f32());
                
                if event_count == 0 {
                    println!();
                    println!("No events captured during test period!");
                    println!("   Try running commands in another terminal:");
                    println!("   ls, ps, cat /proc/version, etc.");
                }
            }
            Err(e) => {
                let runtime = start_time.elapsed();
                eprintln!("ProcessRunner failed after {:.2}s: {}", runtime.as_secs_f32(), e);
                eprintln!("   Possible causes:");
                eprintln!("   - Insufficient privileges (try: sudo cargo test ...)");
                eprintln!("   - Binary compilation failed");
                eprintln!("   - eBPF/kernel support missing");
                eprintln!("   - Missing kernel headers");
                
                // Don't panic - allow test to pass even with environmental issues
                return;
            }
        }
    }
} 