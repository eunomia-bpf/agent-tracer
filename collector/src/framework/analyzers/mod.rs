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
    fn name(&self) -> &str;
}

pub mod raw;
pub mod output;

pub use raw::RawAnalyzer;
pub use output::OutputAnalyzer; 