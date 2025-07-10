use super::{Runner, SslConfig, EventStream, RunnerError};
use super::common::{BinaryExecutor, AnalyzerProcessor, IntoFrameworkEvent};
use crate::framework::core::Event;
use crate::framework::analyzers::Analyzer;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
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

impl IntoFrameworkEvent for SslEventData {
    fn into_framework_event(self, source: &str) -> Event {
        Event::new_with_id_and_timestamp(
            Uuid::new_v4().to_string(),
            self.timestamp,
            source.to_string(),
            self.event_type.clone(),
            serde_json::json!({
                "pid": self.pid,
                "comm": self.comm,
                "fd": self.fd,
                "data": self.data,
                "data_len": self.data_len,
                "event_type": self.event_type
            }),
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
    pub fn tls_version(mut self, version: String) -> Self {
        self.config.tls_version = Some(version);
        self
    }
}

#[async_trait]
impl Runner for SslRunner {
    async fn run(&mut self) -> Result<EventStream, RunnerError> {
        let events = self.executor.collect_events::<SslEventData>("ssl").await?;
        AnalyzerProcessor::process_through_analyzers(events, &mut self.analyzers).await
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

    #[test]
    fn test_ssl_event_into_framework_event() {
        let ssl_data = SslEventData {
            pid: 1234,
            comm: "curl".to_string(),
            fd: 3,
            timestamp: 1234567890,
            event_type: "ssl_read".to_string(),
            data: "test data".to_string(),
            data_len: 9,
        };

        let event = ssl_data.into_framework_event("ssl");
        assert_eq!(event.source, "ssl");
        assert_eq!(event.event_type, "ssl_read");
        assert_eq!(event.timestamp, 1234567890);
        assert!(event.data.get("pid").is_some());
        assert!(event.data.get("comm").is_some());
        assert!(event.data.get("fd").is_some());
    }
} 