use crate::framework::analyzers::Analyzer;
use super::{EventStream, RunnerError};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use log::debug;
use futures::stream::Stream;
use std::pin::Pin;



/// Type alias for JSON stream
pub type JsonStream = Pin<Box<dyn Stream<Item = serde_json::Value> + Send>>;

/// Common binary executor for runners - now supports streaming
pub struct BinaryExecutor {
    binary_path: String,
    additional_args: Vec<String>,
    runner_name: Option<String>,
}

impl BinaryExecutor {
    pub fn new(binary_path: String) -> Self {
        Self { 
            binary_path,
            additional_args: Vec::new(),
            runner_name: None,
        }
    }

    /// Add additional command-line arguments
    pub fn with_args(mut self, args: &[String]) -> Self {
        self.additional_args = args.to_vec();
        self
    }

    /// Set runner name for debugging purposes
    pub fn with_runner_name(mut self, name: String) -> Self {
        self.runner_name = Some(name);
        self
    }

    /// Execute binary and get raw JSON stream
    pub async fn get_json_stream(&self) -> Result<JsonStream, RunnerError> {
        // Log the actual exec command with all arguments
        if self.additional_args.is_empty() {
            log::info!("Executing binary: {}", self.binary_path);
        } else {
            log::info!("Executing binary: {} {}", self.binary_path, self.additional_args.join(" "));
        }
        
        let mut cmd = TokioCommand::new(&self.binary_path);
        cmd.stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        // Add additional arguments if any
        if !self.additional_args.is_empty() {
            cmd.args(&self.additional_args);
            debug!("Added arguments: {:?}", self.additional_args);
        }
        
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
        
        // Clone needed data for the stream
        let runner_name = self.runner_name.clone();
        let binary_path = self.binary_path.clone();
        
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
                            let runner_info = runner_name.as_ref()
                                .map(|name| format!("[{}] ", name))
                                .unwrap_or_else(|| format!("[{}] ", 
                                    std::path::Path::new(&binary_path)
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("unknown")
                                ));
                            
                            // Convert raw bytes to a printable format
                            let raw_data = line.as_bytes();
                            let printable_data = raw_data.iter()
                                .map(|&b| {
                                    if b.is_ascii_graphic() || b == b' ' {
                                        // Printable ASCII characters
                                        char::from(b).to_string()
                                    } else if b == b'\t' {
                                        "\\t".to_string()
                                    } else if b == b'\r' {
                                        "\\r".to_string()
                                    } else if b == b'\n' {
                                        "\\n".to_string()
                                    } else if b == b'\0' {
                                        "\\0".to_string()
                                    } else {
                                        // Non-printable characters as hex
                                        format!("\\x{:02x}", b)
                                    }
                                })
                                .collect::<String>();
                            
                            // Also show hex dump for debugging
                            let hex_dump = raw_data.iter()
                                .take(32) // Show first 32 bytes
                                .map(|b| format!("{:02x}", b))
                                .collect::<Vec<_>>()
                                .join(" ");
                            
                            log::warn!("{}Invalid UTF-8 data from binary at line {}, skipping line", 
                                runner_info, line_count + 1);
                            log::warn!("{}Raw data (printable): {}", runner_info, printable_data);
                            log::warn!("{}Raw data (hex): {}", runner_info, hex_dump);
                            
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