use crate::framework::core::Event;
use crate::framework::analyzers::Analyzer;
use super::{EventStream, RunnerError};
use serde::de::DeserializeOwned;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use log::debug;
use futures::stream::Stream;
use std::pin::Pin;

/// Trait for converting from specific event data types to framework Events
pub trait IntoFrameworkEvent {
    fn into_framework_event(self, source: &str) -> Event;
}

/// Type alias for JSON stream
pub type JsonStream = Pin<Box<dyn Stream<Item = serde_json::Value> + Send>>;

/// Common binary executor for runners - now supports streaming
pub struct BinaryExecutor {
    binary_path: String,
}

impl BinaryExecutor {
    pub fn new(binary_path: String) -> Self {
        Self { binary_path }
    }

    /// Execute binary and get raw JSON stream
    pub async fn get_json_stream(&self) -> Result<JsonStream, RunnerError> {
        debug!("Starting binary for JSON stream: {}", self.binary_path);
        
        let mut cmd = TokioCommand::new(&self.binary_path);
        cmd.stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let mut child = cmd.spawn()
            .map_err(|e| Box::new(std::io::Error::new(
                std::io::ErrorKind::Other, 
                format!("Failed to start binary: {}", e)
            )) as RunnerError)?;
            
        let stdout = child.stdout.take()
            .ok_or_else(|| Box::new(std::io::Error::new(
                std::io::ErrorKind::Other, 
                "Failed to get stdout"
            )) as RunnerError)?;
        
        if let Some(pid) = child.id() {
            debug!("Binary started with PID: Some({})", pid);
        }
        
        let stream = async_stream::stream! {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            let mut line_count = 0;
            
            debug!("Reading from binary stdout");
            
            loop {
                line.clear();
                
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        debug!("Binary stdout closed (EOF)");
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
                            
                            // Try to parse as JSON
                            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                                match serde_json::from_str::<serde_json::Value>(trimmed) {
                                    Ok(json_value) => {
                                        debug!("Parsed JSON value");
                                        yield json_value;
                                    }
                                    Err(e) => {
                                        log::warn!("Failed to parse JSON from line {}: {} - Line: {}", 
                                            line_count, e,
                                            if trimmed.len() > 200 { 
                                                format!("{}...", &trimmed[..200]) 
                                            } else { 
                                                trimmed.to_string() 
                                            }
                                        );
                                    }
                                }
                            } else {
                                log::warn!("Skipping non-JSON line {} from binary: {}", 
                                    line_count, 
                                    if trimmed.len() > 100 { 
                                        format!("{}...", &trimmed[..100]) 
                                    } else { 
                                        trimmed.to_string() 
                                    }
                                );
                            }
                        }
                    }
                    Err(e) => {
                        // Handle UTF-8 errors gracefully - don't terminate, just warn and continue
                        if e.kind() == std::io::ErrorKind::InvalidData {
                            log::warn!("Invalid UTF-8 data from binary at line {}, skipping line", line_count + 1);
                            // Try to read the next line 
                            continue;
                        } else {
                            debug!("Error reading from binary: {}", e);
                            break;
                        }
                    }
                }
            }
            
            debug!("Terminating binary process");
            
            // Terminate the child process
            if let Err(e) = child.kill().await {
                debug!("Failed to kill binary process: {}", e);
            }
            
            // Wait for process to finish
            match child.wait().await {
                Ok(status) => {
                    debug!("Binary process terminated with status: {}", status);
                }
                Err(e) => {
                    debug!("Error waiting for binary process: {}", e);
                }
            }
        };
        
        Ok(Box::pin(stream))
    }

    /// Execute binary and collect events as a stream (for real-time processing)
    /// This method is kept for compatibility with existing code
    pub async fn collect_events<T>(&self, source: &str) -> Result<EventStream, RunnerError>
    where
        T: DeserializeOwned + IntoFrameworkEvent + Send + 'static,
    {
        let source_name = source.to_string(); // Own the source name
        debug!("Starting {} binary: {}", source_name, self.binary_path);
        
        let mut cmd = TokioCommand::new(&self.binary_path);
        cmd.stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        let mut child = cmd.spawn()
            .map_err(|e| Box::new(std::io::Error::new(
                std::io::ErrorKind::Other, 
                format!("Failed to start {} binary: {}", source_name, e)
            )) as RunnerError)?;
            
        let stdout = child.stdout.take()
            .ok_or_else(|| Box::new(std::io::Error::new(
                std::io::ErrorKind::Other, 
                "Failed to get stdout"
            )) as RunnerError)?;
        
        if let Some(pid) = child.id() {
            debug!("{} binary started with PID: Some({})", source_name, pid);
        }
        
        let stream = async_stream::stream! {
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
                            
                            // Try to parse as JSON for any runner type
                            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                                match serde_json::from_str::<T>(trimmed) {
                                    Ok(event_data) => {
                                        debug!("Parsed event - {}", source_name);
                                        yield event_data.into_framework_event(&source_name);
                                    }
                                    Err(e) => {
                                        log::warn!("Failed to parse {} event from line {}: {} - Line: {}", 
                                            source_name, line_count, e,
                                            if trimmed.len() > 200 { 
                                                format!("{}...", &trimmed[..200]) 
                                            } else { 
                                                trimmed.to_string() 
                                            }
                                        );
                                    }
                                }
                            } else {
                                log::warn!("Skipping non-JSON line {} from {} binary: {}", 
                                    line_count, source_name, 
                                    if trimmed.len() > 100 { 
                                        format!("{}...", &trimmed[..100]) 
                                    } else { 
                                        trimmed.to_string() 
                                    }
                                );
                            }
                        }
                    }
                    Err(e) => {
                        // Handle UTF-8 errors gracefully - don't terminate, just warn and continue
                        if e.kind() == std::io::ErrorKind::InvalidData {
                            log::warn!("Invalid UTF-8 data from {} binary at line {}, skipping line", source_name, line_count + 1);
                            // Try to read the next line 
                            continue;
                        } else {
                            debug!("Error reading from {} binary: {}", source_name, e);
                            break;
                        }
                    }
                }
            }
            
            debug!("Terminating {} binary process", source_name);
            
            // Terminate the child process
            if let Err(e) = child.kill().await {
                debug!("Failed to kill {} binary process: {}", source_name, e);
            }
            
            // Wait for process to finish
            match child.wait().await {
                Ok(status) => {
                    debug!("{} binary process terminated with status: {}", source_name, status);
                }
                Err(e) => {
                    debug!("Error waiting for {} binary process: {}", source_name, e);
                }
            }
        };
        
        Ok(Box::pin(stream))
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