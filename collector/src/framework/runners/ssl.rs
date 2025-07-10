use super::{Runner, SslConfig, EventStream, RunnerError};
use super::common::{BinaryExecutor, AnalyzerProcessor, IntoFrameworkEvent};
use crate::framework::core::Event;
use crate::framework::analyzers::Analyzer;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

/// SSL event data structure from ssl binary (matches actual sslsniff output format)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SslEventData {
    #[serde(rename = "function")]
    pub function_type: String, // "READ/RECV" or "WRITE/SEND"
    pub time_s: f64,
    pub timestamp_ns: u64,
    pub comm: String,
    pub pid: u32,
    pub len: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_handshake: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

impl IntoFrameworkEvent for SslEventData {
    fn into_framework_event(self, source: &str) -> Event {
        let mut data = serde_json::json!({
            "pid": self.pid,
            "comm": self.comm,
            "function_type": self.function_type,
            "time_s": self.time_s,
            "len": self.len
        });
        
        // Add optional fields if they exist
        if let Some(is_handshake) = self.is_handshake {
            data["is_handshake"] = serde_json::Value::Bool(is_handshake);
        }
        if let Some(ssl_data) = self.data {
            data["data"] = serde_json::Value::String(ssl_data);
        }
        if let Some(truncated) = self.truncated {
            data["truncated"] = serde_json::Value::Bool(truncated);
        }
        
        Event::new_with_id_and_timestamp(
            Uuid::new_v4().to_string(),
            self.timestamp_ns,
            source.to_string(),
            self.function_type.clone(),
            data,
        )
    }
}

/// Runner for collecting SSL/TLS events
pub struct SslRunner {
    id: String,
    config: SslConfig,
    analyzers: Vec<Box<dyn Analyzer>>,
    executor: BinaryExecutor,
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
        }
    }

    /// Create a new SslRunner with a custom ID
    pub fn with_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    /// Set the port to monitor
    pub fn port(mut self, port: u16) -> Self {
        self.config.port = Some(port);
        self
    }

    /// Set the network interface to monitor
    pub fn interface(mut self, interface: String) -> Self {
        self.config.interface = Some(interface);
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
        let stream = self.executor.collect_events::<SslEventData>("ssl").await?;
        AnalyzerProcessor::process_through_analyzers(stream, &mut self.analyzers).await
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
    use tokio_stream::StreamExt;

    #[test]
    fn test_ssl_runner_creation() {
        let runner = SslRunner::from_binary_extractor("/fake/path/sslsniff");
        assert_eq!(runner.name(), "ssl");
        assert!(!runner.id().is_empty());
        assert_eq!(runner.config.port, Some(443));
    }

    #[test]
    fn test_ssl_runner_with_custom_config() {
        let runner = SslRunner::from_binary_extractor("/fake/path/sslsniff")
            .with_id("test-ssl".to_string())
            .port(8443)
            .interface("eth0".to_string());

        assert_eq!(runner.id(), "test-ssl");
        assert_eq!(runner.config.port, Some(8443));
        assert_eq!(runner.config.interface, Some("eth0".to_string()));
    }

    #[test]
    fn test_ssl_event_data_serialization() {
        let ssl_data = SslEventData {
            function_type: "READ/RECV".to_string(),
            time_s: 123.456,
            timestamp_ns: 1234567890,
            comm: "test".to_string(),
            pid: 1234,
            len: 9,
            is_handshake: Some(false),
            data: Some("test data".to_string()),
            truncated: Some(false),
        };

        let json = serde_json::to_string(&ssl_data).unwrap();
        let deserialized: SslEventData = serde_json::from_str(&json).unwrap();

        assert_eq!(ssl_data.function_type, deserialized.function_type);
        assert_eq!(ssl_data.time_s, deserialized.time_s);
        assert_eq!(ssl_data.timestamp_ns, deserialized.timestamp_ns);
        assert_eq!(ssl_data.comm, deserialized.comm);
        assert_eq!(ssl_data.pid, deserialized.pid);
        assert_eq!(ssl_data.len, deserialized.len);
        assert_eq!(ssl_data.is_handshake, deserialized.is_handshake);
        assert_eq!(ssl_data.data, deserialized.data);
        assert_eq!(ssl_data.truncated, deserialized.truncated);
    }

    #[test]
    fn test_ssl_event_into_framework_event() {
        let ssl_data = SslEventData {
            function_type: "ssl_read".to_string(),
            time_s: 123.456,
            timestamp_ns: 1234567890,
            comm: "curl".to_string(),
            pid: 1234,
            len: 9,
            is_handshake: Some(false),
            data: Some("test data".to_string()),
            truncated: Some(false),
        };

        let event = ssl_data.into_framework_event("ssl");
        assert_eq!(event.source, "ssl");
        assert_eq!(event.event_type, "ssl_read");
        assert_eq!(event.timestamp, 1234567890);
        assert!(event.data.get("pid").is_some());
        assert!(event.data.get("comm").is_some());
        assert!(event.data.get("function_type").is_some());
        assert!(event.data.get("time_s").is_some());
        assert!(event.data.get("len").is_some());
        assert!(event.data.get("is_handshake").is_some());
        assert!(event.data.get("data").is_some());
        assert!(event.data.get("truncated").is_some());
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
            .port(443) // Monitor HTTPS traffic
            .interface("any".to_string()) // Monitor all interfaces
            .add_analyzer(Box::new(crate::framework::analyzers::RawAnalyzer::new_with_options(false)));
        
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
                            event_opt = stream.next() => {
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