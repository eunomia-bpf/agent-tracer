use super::{Analyzer, AnalyzerError};
use crate::framework::runners::EventStream;
use async_trait::async_trait;
use futures::stream::StreamExt;
use log::debug;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// FileLogger analyzer that logs events to a specified file
pub struct FileLogger {
    name: String,
    file_path: String,
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
            file_handle: Arc::new(Mutex::new(file)),
        })
    }

    /// Create a new FileLogger with custom options (for backward compatibility)
    #[allow(dead_code)]
    pub fn new_with_options<P: AsRef<Path>>(
        file_path: P,
        _pretty_print: bool,  // Ignored - we always use raw JSON
        _log_all_events: bool, // Ignored - we always log all events
    ) -> Result<Self, std::io::Error> {
        Self::new(file_path)
    }

    /// Convert binary data to hex string
    fn data_to_string(data: &serde_json::Value) -> String {
        match data {
            serde_json::Value::String(s) => {
                // Check if string contains valid UTF-8
                if s.chars().all(|c| !c.is_control() || c == '\n' || c == '\r' || c == '\t') {
                    s.clone()
                } else {
                    // Convert to hex if it contains control characters (likely binary)
                    format!("HEX:{}", hex::encode(s.as_bytes()))
                }
            }
            serde_json::Value::Null => "null".to_string(),
            _ => data.to_string()
        }
    }
}

#[async_trait]
impl Analyzer for FileLogger {
    async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
        
        let file_handle = Arc::clone(&self.file_handle);
        let file_path = self.file_path.clone();
        
        // Process events using map instead of consuming the stream
        let processed_stream = stream.map(move |event| {
            debug!("FileLogger: Processing event: {:?}", event);
            // Log the event to file
            if let Ok(mut file) = file_handle.lock() {
                // Convert event to JSON, handling binary data in the "data" field
                let event_json = match event.to_json() {
                    Ok(json_str) => {
                        // Parse and fix data field if it contains binary
                        if let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(&json_str) {
                            if let Some(data_obj) = parsed.get_mut("data") {
                                if let Some(data_field) = data_obj.get_mut("data") {
                                    let data_str = Self::data_to_string(data_field);
                                    *data_field = serde_json::Value::String(data_str);
                                }
                            }
                            serde_json::to_string(&parsed).unwrap_or(json_str)
                        } else {
                            json_str
                        }
                    }
                    Err(e) => {
                        format!("{{\"error\":\"Failed to serialize event: {}\"}}", e)
                    }
                };
                
                // Write just the JSON without timestamp
                let log_entry = format!("{}\n", event_json);

                if let Err(e) = file.write_all(log_entry.as_bytes()) {
                    eprintln!("FileLogger: Failed to write to {}: {}", file_path, e);
                } else if let Err(e) = file.flush() {
                    eprintln!("FileLogger: Failed to flush {}: {}", file_path, e);
                }
            }
            
            // Pass the event through unchanged
            event
        });

        Ok(Box::pin(processed_stream))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::core::Event;
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
    }

    #[tokio::test]
    async fn test_file_logger_processes_events() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut logger = FileLogger::new(temp_file.path()).unwrap();
        
        let test_event = Event::new("test".to_string(), 1234, "test".to_string(), json!({
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

    #[tokio::test]
    async fn test_file_logger_with_binary_data() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut logger = FileLogger::new(temp_file.path()).unwrap();
        
        // Create an event with binary data
        let binary_data = String::from_utf8_lossy(&[0x00, 0x01, 0x02, 0xFF, 0xFE]).to_string();
        let test_event = Event::new("ssl".to_string(), 1234, "ssl".to_string(), json!({
            "data": binary_data,
            "len": 5
        }));
        
        let events = vec![test_event];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = logger.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        assert_eq!(collected.len(), 1);
        
        // Check that file was written with hex encoding
        let file_contents = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(file_contents.contains("HEX:"));
    }
} 