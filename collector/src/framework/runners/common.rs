use crate::framework::core::Event;
use crate::framework::analyzers::Analyzer;
use super::{EventStream, RunnerError};
use serde::de::DeserializeOwned;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use log::debug;

/// Trait for converting from specific event data types to framework Events
pub trait IntoFrameworkEvent {
    fn into_framework_event(self, source: &str) -> Event;
}

/// Common binary executor for runners - now supports streaming
pub struct BinaryExecutor {
    binary_path: String,
}

impl BinaryExecutor {
    pub fn new(binary_path: String) -> Self {
        Self { binary_path }
    }

    /// Execute binary and collect events as a stream (for real-time processing)
    pub async fn collect_events<T>(&self, source: &str) -> Result<EventStream, RunnerError>
    where
        T: DeserializeOwned + IntoFrameworkEvent + Send + 'static,
    {
        debug!("Starting {} binary: {}", source, self.binary_path);
        
        // Spawn the process with piped stdout
        let mut child = TokioCommand::new(&self.binary_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn {} binary: {}", source, e))?;

        let stdout = child.stdout.take()
            .ok_or_else(|| format!("Failed to capture stdout for {} binary", source))?;

        debug!("{} binary started with PID: {:?}", source, child.id());
        
        let source_name = source.to_string();
        
        // Create a stream that reads lines from stdout and converts them to events
        let event_stream = async_stream::stream! {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            let mut line_count = 0;
            
            debug!("Reading from {} binary stdout", source_name);
            
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        debug!("{} binary stdout closed (EOF)", source_name);
                        break;
                    }
                    Ok(_) => {
                        line_count += 1;
                        let trimmed = line.trim();
                        
                        if !trimmed.is_empty() {
                            debug!("Line {}: {}", line_count, 
                                if trimmed.len() > 100 { 
                                    format!("{}...", &trimmed[..100]) 
                                } else { 
                                    trimmed.to_string() 
                                }
                            );
                            
                            // Only process lines that are actual events (not config or other types)
                            if trimmed.contains("\"type\":\"event\"") {
                                match serde_json::from_str::<T>(trimmed) {
                                    Ok(event_data) => {
                                        let event = event_data.into_framework_event(&source_name);
                                        debug!("Parsed event: {} - {}", event.event_type, event.source);
                                        yield event;
                                    }
                                    Err(e) => {
                                        debug!("Failed to parse JSON on line {}: {} - Raw: {}", 
                                            line_count, e, trimmed);
                                    }
                                }
                            } else {
                                debug!("Skipping non-event line {}: {}", line_count, 
                                    if trimmed.contains("\"type\":\"config\"") { "config" } else { "unknown" });
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Error reading from {} binary: {}", source_name, e);
                        break;
                    }
                }
            }
            
            // Ensure child process is terminated
            debug!("Terminating {} binary process", source_name);
            let _ = child.kill().await;
            let _ = child.wait().await;
            debug!("{} binary process terminated", source_name);
        };

        Ok(Box::pin(event_stream))
    }
}

/// Common analyzer processor for runners
pub struct AnalyzerProcessor;

impl AnalyzerProcessor {
    /// Process events through a chain of analyzers
    pub async fn process_through_analyzers(
        mut stream: EventStream, 
        analyzers: &mut [Box<dyn Analyzer>]
    ) -> Result<EventStream, RunnerError> {
        // Process through each analyzer in sequence
        for analyzer in analyzers.iter_mut() {
            stream = analyzer.process(stream).await?;
        }
        
        Ok(stream)
    }
} 