use super::{Runner, ProcessConfig, EventStream};
use crate::framework::core::Event;
use crate::framework::analyzers::Analyzer;
use async_trait::async_trait;
use futures::stream;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

/// Process event data structure from process binary (matches original ProcessEvent)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessEventData {
    pub pid: u32,
    pub ppid: u32,
    pub comm: String,
    pub filename: String,
    pub timestamp: u64,
    pub event_type: String,
}

/// Runner for collecting process/system events
pub struct ProcessRunner {
    id: String,
    config: ProcessConfig,
    analyzers: Vec<Box<dyn Analyzer>>,
    binary_path: Option<String>,
    use_simulation: bool,
}

impl ProcessRunner {
    /// Create a new ProcessRunner with default configuration (simulation mode)
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            config: ProcessConfig::default(),
            analyzers: Vec::new(),
            binary_path: None,
            use_simulation: true,
        }
    }

    /// Create a new ProcessRunner with a custom ID
    pub fn with_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    /// Set the PID to monitor
    pub fn pid(mut self, pid: u32) -> Self {
        self.config.pid = Some(pid);
        self
    }

    /// Set the process name filter
    pub fn name_filter(mut self, name: String) -> Self {
        self.config.name = Some(name);
        self
    }

    /// Set the CPU threshold for filtering
    pub fn cpu_threshold(mut self, threshold: f32) -> Self {
        self.config.cpu_threshold = Some(threshold);
        self
    }

    /// Set the memory threshold for filtering
    pub fn memory_threshold(mut self, threshold: u64) -> Self {
        self.config.memory_threshold = Some(threshold);
        self
    }

    /// Set the binary path for the process executable and enable real execution
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
            config: ProcessConfig::default(),
            analyzers: Vec::new(),
            binary_path: Some(binary_path.as_ref().to_string_lossy().to_string()),
            use_simulation: false,
        }
    }

    /// Collect process events from the real binary
    async fn collect_process_events_real(&self) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
        let binary_path = self.binary_path.as_ref()
            .ok_or("Binary path not set for real execution")?;

        let output = Command::new(binary_path)
            .output()
            .map_err(|e| format!("Failed to execute process binary: {}", e))?;

        if !output.status.success() {
            return Err(format!("Process binary failed with status: {}", output.status).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut events = Vec::new();

        for line in stdout.lines() {
            if let Ok(process_event) = serde_json::from_str::<ProcessEventData>(line) {
                // Convert ProcessEventData to framework Event
                let event = Event::new_with_id_and_timestamp(
                    Uuid::new_v4().to_string(),
                    process_event.timestamp,
                    "process".to_string(),
                    process_event.event_type.clone(),
                    serde_json::json!({
                        "pid": process_event.pid,
                        "ppid": process_event.ppid,
                        "comm": process_event.comm,
                        "filename": process_event.filename,
                        "event_type": process_event.event_type
                    }),
                );
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Collect process events from simulation (for testing/demo)
    async fn collect_process_events_simulated(&self) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
        let simulated_events = vec![
            Event::new(
                "process".to_string(),
                "exec".to_string(),
                serde_json::json!({
                    "pid": 5678,
                    "ppid": 1234,
                    "comm": "python3",
                    "filename": "/usr/bin/python3",
                    "event_type": "exec"
                }),
            ),
            Event::new(
                "process".to_string(),
                "exit".to_string(),
                serde_json::json!({
                    "pid": 5678,
                    "ppid": 1234,
                    "comm": "python3",
                    "filename": "/usr/bin/python3",
                    "event_type": "exit"
                }),
            ),
            Event::new(
                "process".to_string(),
                "open".to_string(),
                serde_json::json!({
                    "pid": 9999,
                    "ppid": 5678,
                    "comm": "cat",
                    "filename": "/etc/hosts",
                    "event_type": "open"
                }),
            ),
        ];

        // Add a small delay to simulate actual data collection
        sleep(Duration::from_millis(150)).await;
        
        Ok(simulated_events)
    }

    /// Collect process events (real or simulated based on configuration)
    async fn collect_process_events(&self) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
        if self.use_simulation {
            self.collect_process_events_simulated().await
        } else {
            self.collect_process_events_real().await
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

impl Default for ProcessRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Runner for ProcessRunner {
    async fn run(&mut self) -> Result<EventStream, Box<dyn std::error::Error>> {
        let events = self.collect_process_events().await?;
        self.process_through_analyzers(events).await
    }

    fn add_analyzer(mut self, analyzer: Box<dyn Analyzer>) -> Self {
        self.analyzers.push(analyzer);
        self
    }

    fn name(&self) -> &str {
        "process"
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
    async fn test_process_runner_creation() {
        let runner = ProcessRunner::new();
        assert_eq!(runner.name(), "process");
        assert!(!runner.id().is_empty());
        assert_eq!(runner.config.pid, None);
        assert!(runner.use_simulation);
    }

    #[tokio::test]
    async fn test_process_runner_with_custom_config() {
        let runner = ProcessRunner::new()
            .with_id("test-process".to_string())
            .pid(1234)
            .name_filter("python".to_string())
            .cpu_threshold(80.0);

        assert_eq!(runner.id(), "test-process");
        assert_eq!(runner.config.pid, Some(1234));
        assert_eq!(runner.config.name, Some("python".to_string()));
        assert_eq!(runner.config.cpu_threshold, Some(80.0));
    }

    #[tokio::test]
    async fn test_process_runner_simulation_mode() {
        let mut runner = ProcessRunner::new()
            .simulation(true)
            .add_analyzer(Box::new(RawAnalyzer::new_with_options(false)));

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;

        assert_eq!(events.len(), 3); // We simulate 3 events
        assert_eq!(events[0].source, "process");
        assert_eq!(events[0].event_type, "exec");
        assert_eq!(events[1].event_type, "exit");
        assert_eq!(events[2].event_type, "open");
    }

    #[tokio::test]
    async fn test_process_runner_from_binary_extractor() {
        let runner = ProcessRunner::from_binary_extractor("/fake/path/process")
            .simulation(true) // Force simulation for testing
            .add_analyzer(Box::new(RawAnalyzer::new_with_options(false)));

        assert_eq!(runner.name(), "process");
        assert!(runner.binary_path.is_some());
        assert_eq!(runner.binary_path.as_ref().unwrap(), "/fake/path/process");
    }

    #[tokio::test]
    async fn test_process_runner_event_data_structure() {
        let mut runner = ProcessRunner::new().simulation(true);
        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let first_event = &events[0];
        assert!(first_event.data.get("pid").is_some());
        assert!(first_event.data.get("ppid").is_some());
        assert!(first_event.data.get("comm").is_some());
        assert!(first_event.data.get("filename").is_some());
        assert!(first_event.data.get("event_type").is_some());
    }

    #[tokio::test]
    async fn test_process_runner_binary_path_configuration() {
        let runner = ProcessRunner::new()
            .binary_path("/custom/path/process".to_string());

        assert!(!runner.use_simulation); // Should disable simulation when binary path is set
        assert_eq!(runner.binary_path.as_ref().unwrap(), "/custom/path/process");
    }

    #[tokio::test]
    async fn test_process_runner_multiple_analyzers() {
        let mut runner = ProcessRunner::new()
            .simulation(true)
            .add_analyzer(Box::new(RawAnalyzer::new_with_options(false)))
            .add_analyzer(Box::new(RawAnalyzer::new_with_options(false)));

        let stream = runner.run().await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Events should pass through multiple analyzers
        assert_eq!(events.len(), 3);
        assert!(events.iter().all(|e| e.source == "process"));
    }

    #[tokio::test]
    async fn test_process_event_data_serialization() {
        let process_data = ProcessEventData {
            pid: 1234,
            ppid: 5678,
            comm: "test".to_string(),
            filename: "/test/path".to_string(),
            timestamp: 1234567890,
            event_type: "exec".to_string(),
        };

        let json = serde_json::to_string(&process_data).unwrap();
        let deserialized: ProcessEventData = serde_json::from_str(&json).unwrap();

        assert_eq!(process_data.pid, deserialized.pid);
        assert_eq!(process_data.ppid, deserialized.ppid);
        assert_eq!(process_data.comm, deserialized.comm);
        assert_eq!(process_data.filename, deserialized.filename);
        assert_eq!(process_data.event_type, deserialized.event_type);
    }
} 