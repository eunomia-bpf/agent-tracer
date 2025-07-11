use super::{Analyzer, AnalyzerError};
use crate::framework::runners::EventStream;
use crate::framework::core::Event;
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio_stream::wrappers::UnboundedReceiverStream;
use chrono::{DateTime, Utc};

/// FileLogger analyzer that logs events to a specified file
pub struct FileLogger {
    name: String,
    file_path: String,
    pretty_print: bool,
    log_all_events: bool,
    file_handle: Arc<Mutex<std::fs::File>>,
}

impl FileLogger {
    /// Create a new FileLogger with specified file path
    pub fn new<P: AsRef<Path>>(file_path: P) -> Result<Self, std::io::Error> {
        let path_str = file_path.as_ref().to_string_lossy().to_string();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path_str)?;

        Ok(Self {
            name: "FileLogger".to_string(),
            file_path: path_str,
            pretty_print: true,
            log_all_events: true,
            file_handle: Arc::new(Mutex::new(file)),
        })
    }

    /// Create a new FileLogger with custom options
    pub fn new_with_options<P: AsRef<Path>>(
        file_path: P,
        pretty_print: bool,
        log_all_events: bool,
    ) -> Result<Self, std::io::Error> {
        let path_str = file_path.as_ref().to_string_lossy().to_string();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path_str)?;

        Ok(Self {
            name: "FileLogger".to_string(),
            file_path: path_str,
            pretty_print,
            log_all_events,
            file_handle: Arc::new(Mutex::new(file)),
        })
    }

    /// Log an event to the file
    fn log_event(&self, event: &Event) {
        if let Ok(mut file) = self.file_handle.lock() {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC");
            
            let json_str = if self.pretty_print {
                serde_json::to_string_pretty(&event.data).unwrap_or_else(|_| "{}".to_string())
            } else {
                serde_json::to_string(&event.data).unwrap_or_else(|_| "{}".to_string())
            };

            let log_entry = format!(
                "[{}] EVENT: source={}, id={}, timestamp={}\n{}\n{}\n",
                timestamp,
                event.source,
                event.id,
                event.timestamp,
                json_str,
                "=".repeat(80)
            );

            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                eprintln!("FileLogger: Failed to write to {}: {}", self.file_path, e);
            } else if let Err(e) = file.flush() {
                eprintln!("FileLogger: Failed to flush {}: {}", self.file_path, e);
            } else {
                eprintln!("FileLogger: Successfully wrote event to {}", self.file_path);
            }
        } else {
            eprintln!("FileLogger: Failed to acquire file lock for {}", self.file_path);
        }
    }

    /// Log a formatted message with event details
    fn log_summary(&self, event: &Event, message: &str) {
        if let Ok(mut file) = self.file_handle.lock() {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC");
            
            let summary_entry = format!(
                "[{}] SUMMARY: {} (source={}, id={})\n",
                timestamp,
                message,
                event.source,
                event.id
            );

            if let Err(e) = file.write_all(summary_entry.as_bytes()) {
                eprintln!("FileLogger: Failed to write summary to {}: {}", self.file_path, e);
            } else if let Err(e) = file.flush() {
                eprintln!("FileLogger: Failed to flush {}: {}", self.file_path, e);
            }
        }
    }
}

#[async_trait]
impl Analyzer for FileLogger {
    async fn process(&mut self, mut stream: EventStream) -> Result<EventStream, AnalyzerError> {
        eprintln!("FileLogger: Starting to log events to '{}'", self.file_path);
        eprintln!("FileLogger: Config - pretty_print: {}, log_all_events: {}", self.pretty_print, self.log_all_events);
        
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let file_path = self.file_path.clone();
        let log_all_events = self.log_all_events;
        
        // Process events and log them
        while let Some(event) = stream.next().await {
            eprintln!("FileLogger: Processing event from source: {}", event.source);
            
            // Log the event to file
            if log_all_events {
                eprintln!("FileLogger: Logging all events - writing event to file");
                self.log_event(&event);
            } else {
                // Only log specific event types if not logging all
                match event.source.as_str() {
                    "http_analyzer" => {
                        if let Some(event_type) = event.data.get("type") {
                            match event_type.as_str() {
                                Some("http_request_response_pair") => {
                                    self.log_event(&event);
                                    let summary = format!(
                                        "HTTP Pair: {} {} -> {} {}",
                                        event.data["request"]["method"].as_str().unwrap_or("?"),
                                        event.data["request"]["url"].as_str().unwrap_or("?"),
                                        event.data["response"]["status_code"].as_u64().unwrap_or(0),
                                        event.data["response"]["status_text"].as_str().unwrap_or("?")
                                    );
                                    self.log_summary(&event, &summary);
                                }
                                _ => {
                                    // Log other HTTP analyzer events with summary only
                                    let summary = format!("HTTP Event: {}", event_type.as_str().unwrap_or("unknown"));
                                    self.log_summary(&event, &summary);
                                }
                            }
                        }
                    }
                    "ssl" => {
                        // For SSL events, log summary only unless log_all_events is true
                        let data_preview = if let Some(data) = event.data.get("data") {
                            match data {
                                Value::String(s) => {
                                    if s.len() > 50 { &s[..50] } else { s }
                                }
                                _ => "non-string data"
                            }
                        } else {
                            "no data"
                        };
                        let summary = format!("SSL Event: {} bytes, data: {}", 
                            event.data.get("len").and_then(|v| v.as_u64()).unwrap_or(0),
                            data_preview
                        );
                        self.log_summary(&event, &summary);
                    }
                    _ => {
                        // Log other events with basic summary
                        let summary = format!("Event from source: {}", event.source);
                        self.log_summary(&event, &summary);
                    }
                }
            }
            
            // Forward the event to the next analyzer
            if tx.send(event).is_err() {
                break;
            }
        }

        eprintln!("FileLogger: Completed logging to '{}'", file_path);
        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use serde_json::json;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_file_logger_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new(temp_file.path()).unwrap();
        assert_eq!(logger.name(), "FileLogger");
    }

    #[tokio::test]
    async fn test_file_logger_with_options() {
        let temp_file = NamedTempFile::new().unwrap();
        let logger = FileLogger::new_with_options(temp_file.path(), false, false).unwrap();
        assert_eq!(logger.name(), "FileLogger");
        assert!(!logger.pretty_print);
        assert!(!logger.log_all_events);
    }

    #[tokio::test]
    async fn test_file_logger_processes_events() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut logger = FileLogger::new(temp_file.path()).unwrap();
        
        let test_event = Event::new("test".to_string(), json!({
            "message": "test event",
            "value": 42
        }));
        
        let events = vec![test_event];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = logger.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        // Should have one event passed through
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].source, "test");
        
        // Check that file was written to
        let file_contents = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(file_contents.contains("test event"));
    }

    #[test]
    fn test_file_writing_direct() {
        use std::io::Write;
        
        let temp_file = NamedTempFile::new().unwrap();
        println!("Testing file writing to: {:?}", temp_file.path());
        
        // Test 1: Direct file writing
        {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(temp_file.path()).unwrap();
            
            writeln!(file, "Direct write test").unwrap();
            file.flush().unwrap();
            println!("Direct write completed");
        }
        
        // Test 2: Arc<Mutex> file writing (like FileLogger)
        {
            let file_handle = Arc::new(Mutex::new(OpenOptions::new()
                .create(true)
                .append(true)
                .open(temp_file.path()).unwrap()));
            
            if let Ok(mut file) = file_handle.lock() {
                writeln!(file, "Mutex write test").unwrap();
                file.flush().unwrap();
                println!("Mutex write completed");
            } else {
                panic!("Failed to acquire mutex lock");
            }
        }
        
        // Test 3: FileLogger's log_event method
        {
            let logger = FileLogger::new(temp_file.path()).unwrap();
            let test_event = Event::new("test".to_string(), json!({
                "message": "log_event test",
                "value": 123
            }));
            
            logger.log_event(&test_event);
            println!("log_event completed");
        }
        
        // Verify all writes
        let file_contents = std::fs::read_to_string(temp_file.path()).unwrap();
        println!("File contents:\n{}", file_contents);
        
        assert!(file_contents.contains("Direct write test"));
        assert!(file_contents.contains("Mutex write test"));
        assert!(file_contents.contains("log_event test"));
    }
} 