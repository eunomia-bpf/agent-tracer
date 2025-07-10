use crate::framework::core::Event;
use crate::framework::analyzers::Analyzer;
use super::{EventStream, RunnerError};
use futures::stream;
use serde::de::DeserializeOwned;
use std::process::Command;

/// Trait for converting from specific event data types to framework Events
pub trait IntoFrameworkEvent {
    fn into_framework_event(self, source: &str) -> Event;
}

/// Common binary executor for runners
pub struct BinaryExecutor {
    binary_path: String,
}

impl BinaryExecutor {
    pub fn new(binary_path: String) -> Self {
        Self { binary_path }
    }

    /// Execute binary and collect events of a specific type
    pub async fn collect_events<T>(&self, source: &str) -> Result<Vec<Event>, RunnerError>
    where
        T: DeserializeOwned + IntoFrameworkEvent,
    {
        let output = Command::new(&self.binary_path)
            .output()
            .map_err(|e| format!("Failed to execute {} binary: {}", source, e))?;

        if !output.status.success() {
            return Err(format!("{} binary failed with status: {}", source, output.status).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut events = Vec::new();

        for line in stdout.lines() {
            if let Ok(event_data) = serde_json::from_str::<T>(line) {
                let event = event_data.into_framework_event(source);
                events.push(event);
            }
        }

        Ok(events)
    }
}

/// Common analyzer processor for runners
pub struct AnalyzerProcessor;

impl AnalyzerProcessor {
    /// Process events through a chain of analyzers
    pub async fn process_through_analyzers(
        events: Vec<Event>, 
        analyzers: &mut [Box<dyn Analyzer>]
    ) -> Result<EventStream, RunnerError> {
        let mut stream: EventStream = Box::pin(stream::iter(events));
        
        // Process through each analyzer in sequence
        for analyzer in analyzers.iter_mut() {
            stream = analyzer.process(stream).await?;
        }
        
        Ok(stream)
    }
} 