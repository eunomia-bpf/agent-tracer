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
                                // Check if this might be a stderr message or debug output
                                if trimmed.contains("error") || trimmed.contains("warn") || 
                                   trimmed.contains("failed") || trimmed.contains("Error:") {
                                    log::warn!("Possible error message from binary at line {}: {}", 
                                        line_count, trimmed);
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
                            
                            // Try to recover partial data up to the invalid UTF-8 sequence
                            let raw_bytes = line.as_bytes();
                            let valid_up_to = String::from_utf8_lossy(raw_bytes);
                            
                            // If we have a partial JSON object, try to parse it
                            if valid_up_to.trim_start().starts_with('{') {
                                // Find the position of the invalid UTF-8
                                let mut valid_len = 0;
                                for i in 0..raw_bytes.len() {
                                    if std::str::from_utf8(&raw_bytes[0..=i]).is_ok() {
                                        valid_len = i + 1;
                                    } else {
                                        break;
                                    }
                                }
                                
                                if valid_len > 0 {
                                    if let Ok(valid_str) = std::str::from_utf8(&raw_bytes[0..valid_len]) {
                                        log::debug!("Recovered {} valid UTF-8 bytes before error", valid_len);
                                        // Try to parse the valid portion
                                        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(valid_str.trim()) {
                                            log::info!("Successfully recovered partial JSON despite UTF-8 error");
                                            yield json_value;
                                            continue;
                                        }
                                    }
                                }
                            }
                            
                            // Log detailed error information
                            let hex_preview = raw_bytes.iter()
                                .take(64) // Show more context
                                .map(|b| format!("{:02x}", b))
                                .collect::<Vec<_>>()
                                .join(" ");
                            
                            log::warn!(
                                "{}Invalid UTF-8 at line {} (attempted recovery failed). Hex preview: {}",
                                runner_info, line_count + 1, hex_preview
                            );
                            
                            // Clear the line buffer and continue
                            line.clear();
                            continue;
                        } else if e.kind() == std::io::ErrorKind::UnexpectedEof {
                            // Handle partial reads at EOF gracefully
                            if !line.is_empty() {
                                let trimmed = line.trim();
                                if trimmed.starts_with('{') && trimmed.ends_with('}') {
                                    // Try to parse incomplete JSON at EOF
                                    match serde_json::from_str::<serde_json::Value>(trimmed) {
                                        Ok(json_value) => {
                                            log::debug!("Parsed final JSON line at EOF");
                                            yield json_value;
                                        }
                                        Err(e) => {
                                            log::warn!("Failed to parse final line at EOF: {}", e);
                                        }
                                    }
                                }
                            }
                            log::debug!("Reached EOF while reading");
                            break;
                        } else if e.kind() == std::io::ErrorKind::Interrupted {
                            // Retry on interrupted system calls
                            log::debug!("Read interrupted, retrying...");
                            continue;
                        } else {
                            log::warn!("Error reading from binary: {} (kind: {:?})", e, e.kind());
                            break;
                        }
                    }
                }
            }
            
            log::info!("Terminating binary process");
            
            // Terminate the child process
            if let Err(e) = child.kill().await {
                log::warn!("Failed to kill binary process: {}", e);
            }
            
            // Wait for process to finish
            match child.wait().await {
                Ok(status) => {
                    debug!("Binary process terminated with status: {}", status);
                }
                Err(e) => {
                    log::warn!("Error waiting for binary process: {}", e);
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