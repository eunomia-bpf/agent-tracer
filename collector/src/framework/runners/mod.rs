use crate::framework::core::Event;
use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;

/// Type alias for event streams
pub type EventStream = Pin<Box<dyn Stream<Item = Event> + Send>>;

/// Type alias for errors that can be sent between threads
pub type RunnerError = Box<dyn std::error::Error + Send + Sync>;

/// Base trait for all runners that collect observability data
#[async_trait]
pub trait Runner: Send + Sync {
    /// Run the data collection and return a stream of events
    async fn run(&mut self) -> Result<EventStream, RunnerError>;
    
    /// Add an analyzer to this runner's processing chain
    fn add_analyzer(self, analyzer: Box<dyn crate::framework::analyzers::Analyzer>) -> Self
    where
        Self: Sized;
    
    /// Get the name of this runner
    fn name(&self) -> &str;
    
    /// Get a unique identifier for this runner instance
    fn id(&self) -> String;
}

/// Configuration for SSL/TLS monitoring
#[derive(Debug, Clone)]
pub struct SslConfig {
    pub port: Option<u16>,
    pub interface: Option<String>,
    pub tls_version: Option<String>,
}

impl Default for SslConfig {
    fn default() -> Self {
        Self {
            port: Some(443),
            interface: None,
            tls_version: None,
        }
    }
}

/// Configuration for process monitoring
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    pub pid: Option<u32>,
    pub name: Option<String>,
    pub cpu_threshold: Option<f32>,
    pub memory_threshold: Option<u64>,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            pid: None,
            name: None,
            cpu_threshold: None,
            memory_threshold: None,
        }
    }
}

pub mod common;
pub mod ssl;
pub mod process;

pub use ssl::SslRunner;
pub use process::ProcessRunner; 