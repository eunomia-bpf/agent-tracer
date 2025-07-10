use super::{Runner, SslConfig, EventStream};
use crate::framework::core::Event;
use crate::framework::analyzers::Analyzer;
use async_trait::async_trait;
use futures::stream;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

/// SSL/TLS event data structure from sslsniff binary (matches original SslEvent)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SslEventData {
    pub pid: u32,
    pub comm: String,
    pub fd: i32,
    pub timestamp: u64,
    pub event_type: String,
    pub data: String,
    pub data_len: usize,
}

/// Runner for collecting SSL/TLS events
pub struct SslRunner {
    id: String,
    config: SslConfig,
    analyzers: Vec<Box<dyn Analyzer>>,
    binary_path: Option<String>,
    use_simulation: bool,
}

impl SslRunner {
    /// Create a new SslRunner with default configuration (simulation mode)
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            config: SslConfig::default(),
            analyzers: Vec::new(),
            binary_path: None,
            use_simulation: true,
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
    pub fn tls_version(mut self, version: String) -> Self {
        self.config.tls_version = Some(version);
        self
    }

    /// Set the binary path for the sslsniff executable and enable real execution
    pub fn binary_path(mut self, path: String) -> Self {
        self.binary_path = Some(path);
        self.use_simulation = false;
        self
    }

    /// Enable or disable simulation mode
    pub fn simulation(mut self, enabled: bool) -> Self {
        self.use_simulation = enabled;
        self
    }

    /// Create from binary extractor (real execution mode)
    pub fn from_binary_extractor(binary_path: impl AsRef<Path>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            config: SslConfig::default(),
            analyzers: Vec::new(),
            binary_path: Some(binary_path.as_ref().to_string_lossy().to_string()),
            use_simulation: false,
        }
    }

    /// Collect SSL events from the real binary
    async fn collect_ssl_events_real(&self) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
        let binary_path = self.binary_path.as_ref()
            .ok_or("Binary path not set for real execution")?;

        let output = Command::new(binary_path)
            .output()
            .map_err(|e| format!("Failed to execute sslsniff binary: {}", e))?;

        if !output.status.success() {
            return Err(format!("SSLSniff binary failed with status: {}", output.status).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut events = Vec::new();

        for line in stdout.lines() {
            if let Ok(ssl_event) = serde_json::from_str::<SslEventData>(line) {
                // Convert SslEventData to framework Event
                let event = Event::new_with_id_and_timestamp(
                    Uuid::new_v4().to_string(),
                    ssl_event.timestamp,
                    "ssl".to_string(),
                    ssl_event.event_type.clone(),
                    serde_json::json!({
                        "pid": ssl_event.pid,
                        "comm": ssl_event.comm,
                        "fd": ssl_event.fd,
                        "data": ssl_event.data,
                        "data_len": ssl_event.data_len,
                        "event_type": ssl_event.event_type
                    }),
                );
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Collect SSL events from simulation (for testing/demo)
    async fn collect_ssl_events_simulated(&self) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
        let simulated_events = vec![
            Event::new(
                "ssl".to_string(),
                "ssl_read".to_string(),
                serde_json::json!({
                    "pid": 1234,
                    "comm": "curl",
                    "fd": 3,
                    "data": "GET / HTTP/1.1\r\nHost: example.com\r\n",
                    "data_len": 32,
                    "event_type": "ssl_read"
                }),
            ),
            Event::new(
                "ssl".to_string(),
                "ssl_write".to_string(),
                serde_json::json!({
                    "pid": 1234,
                    "comm": "curl",
                    "fd": 3,
                    "data": "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n",
                    "data_len": 38,
                    "event_type": "ssl_write"
                }),
            ),
        ];

        // Add a small delay to simulate actual data collection
        sleep(Duration::from_millis(100)).await;
        
        Ok(simulated_events)
    }

    /// Collect SSL events (real or simulated based on configuration)
    async fn collect_ssl_events(&self) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
        if self.use_simulation {
            self.collect_ssl_events_simulated().await
        } else {
            self.collect_ssl_events_real().await
        }
    }

    /// Process events through the analyzer chain
    async fn process_through_analyzers(&mut self, events: Vec<Event>) -> Result<EventStream, Box<dyn std::error::Error>> {
        let mut stream: EventStream = Box::pin(stream::iter(events));
        
        // Process through each analyzer in sequence
        for analyzer in &mut self.analyzers {
            stream = analyzer.process(stream).await?;
        }
        
        Ok(stream)
    }
}

impl Default for SslRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Runner for SslRunner {
    async fn run(&mut self) -> Result<EventStream, Box<dyn std::error::Error>> {
        let events = self.collect_ssl_events().await?;
        self.process_through_analyzers(events).await
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
    use crate::framework::analyzers::RawAnalyzer;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test_ssl_runner_creation() {
        let runner = SslRunner::new();
        assert_eq!(runner.name(), "ssl");
        assert!(!runner.id().is_empty());
        assert_eq!(runner.config.port, Some(443));
        assert!(runner.use_simulation);
    }

    #[tokio::test]
    async fn test_ssl_runner_with_custom_config() {
        let runner = SslRunner::new()
            .with_id("test-ssl".to_string())
            .port(8443)
            .interface("eth0".to_string());

        assert_eq!(runner.id(), "test-ssl");
        assert_eq!(runner.config.port, Some(8443));
        assert_eq!(runner.config.interface, Some("eth0".to_string()));
    }

    #[tokio::test]
    async fn test_ssl_runner_simulation_mode() {
        let mut runner = SslRunner::new()
            .simulation(true)
            .add_analyzer(Box::new(RawAnalyzer::new_with_options(false)));

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;

        assert_eq!(events.len(), 2); // We simulate 2 events
        assert_eq!(events[0].source, "ssl");
        assert_eq!(events[0].event_type, "ssl_read");
        assert_eq!(events[1].event_type, "ssl_write");
    }

    #[tokio::test]
    async fn test_ssl_runner_from_binary_extractor() {
        let runner = SslRunner::from_binary_extractor("/fake/path/sslsniff")
            .simulation(true) // Force simulation for testing
            .add_analyzer(Box::new(RawAnalyzer::new_with_options(false)));

        assert_eq!(runner.name(), "ssl");
        assert!(runner.binary_path.is_some());
        assert_eq!(runner.binary_path.as_ref().unwrap(), "/fake/path/sslsniff");
    }

    #[tokio::test]
    async fn test_ssl_runner_event_data_structure() {
        let mut runner = SslRunner::new().simulation(true);
        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let first_event = &events[0];
        assert!(first_event.data.get("pid").is_some());
        assert!(first_event.data.get("comm").is_some());
        assert!(first_event.data.get("fd").is_some());
        assert!(first_event.data.get("data").is_some());
        assert!(first_event.data.get("data_len").is_some());
        assert!(first_event.data.get("event_type").is_some());
    }

    #[tokio::test]
    async fn test_ssl_runner_binary_path_configuration() {
        let runner = SslRunner::new()
            .binary_path("/custom/path/sslsniff".to_string());

        assert!(!runner.use_simulation); // Should disable simulation when binary path is set
        assert_eq!(runner.binary_path.as_ref().unwrap(), "/custom/path/sslsniff");
    }

    #[tokio::test]
    async fn test_ssl_event_data_serialization() {
        let ssl_data = SslEventData {
            pid: 1234,
            comm: "test".to_string(),
            fd: 3,
            timestamp: 1234567890,
            event_type: "ssl_read".to_string(),
            data: "test data".to_string(),
            data_len: 9,
        };

        let json = serde_json::to_string(&ssl_data).unwrap();
        let deserialized: SslEventData = serde_json::from_str(&json).unwrap();

        assert_eq!(ssl_data.pid, deserialized.pid);
        assert_eq!(ssl_data.comm, deserialized.comm);
        assert_eq!(ssl_data.event_type, deserialized.event_type);
    }
} 