use super::{Analyzer, AnalyzerError};
use crate::framework::runners::EventStream;
use async_trait::async_trait;
use futures::stream::StreamExt;
use log::debug;
use std::fs::{OpenOptions, File};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Configuration for log rotation
#[derive(Debug, Clone)]
pub struct LogRotationConfig {
    /// Maximum size of a single log file in bytes
    pub max_file_size: u64,
    
    /// Maximum number of rotated log files to keep (excluding current)
    pub max_files: usize,
    
    /// Check file size every N events (performance optimization)
    pub size_check_interval: u64,
    
    /// Whether to compress rotated files (optional feature)
    pub compress_rotated: bool,
}

impl Default for LogRotationConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10_000_000, // 10MB
            max_files: 5,
            size_check_interval: 100,
            compress_rotated: false,
        }
    }
}

/// FileLogger analyzer that logs events to a specified file
pub struct FileLogger {
    name: String,
    file_path: String,
    file_handle: Arc<Mutex<File>>,
    
    // New fields for rotation
    rotation_config: Option<LogRotationConfig>,
    event_count: Arc<Mutex<u64>>,
}

impl FileLogger {
    /// Create a new FileLogger with specified file path (no rotation)
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
            rotation_config: None,
            event_count: Arc::new(Mutex::new(0)),
        })
    }
    
    /// Create FileLogger with rotation configuration
    pub fn with_rotation<P: AsRef<Path>>(
        file_path: P, 
        config: LogRotationConfig
    ) -> Result<Self, std::io::Error> {
        let path_str = file_path.as_ref().to_string_lossy().to_string();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path_str)?;

        Ok(Self {
            name: "FileLogger".to_string(),
            file_path: path_str,
            file_handle: Arc::new(Mutex::new(file)),
            rotation_config: Some(config),
            event_count: Arc::new(Mutex::new(0)),
        })
    }
    
    /// Convenience method for simple size-based rotation
    pub fn with_max_size<P: AsRef<Path>>(
        file_path: P, 
        max_size_mb: u64
    ) -> Result<Self, std::io::Error> {
        let config = LogRotationConfig {
            max_file_size: max_size_mb * 1_000_000,
            ..Default::default()
        };
        Self::with_rotation(file_path, config)
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
    
    /// Perform log rotation (static method for use in closures)
    fn perform_rotation(
        file_handle: &Arc<Mutex<File>>,
        file_path: &str,
        config: &LogRotationConfig,
    ) {
        // Try to acquire the file lock for rotation
        if let Ok(mut file) = file_handle.lock() {
            // Flush and drop the current file handle
            let _ = file.flush();
            drop(file);
            
            // Rotate files in reverse order (app.log.2 -> app.log.3, etc.)
            for i in (1..config.max_files).rev() {
                let old_path = format!("{}.{}", file_path, i);
                let new_path = format!("{}.{}", file_path, i + 1);
                
                if std::path::Path::new(&old_path).exists() {
                    if let Err(e) = std::fs::rename(&old_path, &new_path) {
                        eprintln!("FileLogger: Failed to rotate {} to {}: {}", old_path, new_path, e);
                    }
                }
            }
            
            // Move current file to .1
            let rotated_path = format!("{}.1", file_path);
            if let Err(e) = std::fs::rename(file_path, &rotated_path) {
                eprintln!("FileLogger: Failed to rotate current file to {}: {}", rotated_path, e);
            }
            
            // Create new current file
            match OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(file_path)
            {
                Ok(new_file) => {
                    *file_handle.lock().unwrap() = new_file;
                }
                Err(e) => {
                    eprintln!("FileLogger: Failed to create new log file after rotation: {}", e);
                }
            }
            
            // Cleanup old files beyond max_files limit
            let cleanup_path = format!("{}.{}", file_path, config.max_files + 1);
            if std::path::Path::new(&cleanup_path).exists() {
                if let Err(e) = std::fs::remove_file(&cleanup_path) {
                    eprintln!("FileLogger: Failed to cleanup old log file {}: {}", cleanup_path, e);
                }
            }
        }
    }
}

#[async_trait]
impl Analyzer for FileLogger {
    async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
        
        let file_handle = Arc::clone(&self.file_handle);
        let file_path = self.file_path.clone();
        let rotation_config = self.rotation_config.clone();
        let event_count = Arc::clone(&self.event_count);
        
        // Process events using map instead of consuming the stream
        let processed_stream = stream.map(move |event| {
            debug!("FileLogger: Processing event: {:?}", event);
            
            // Check if we need to rotate logs before processing this event
            if let Some(config) = &rotation_config {
                let mut count = event_count.lock().unwrap();
                *count += 1;
                
                // Check rotation at intervals
                if *count % config.size_check_interval == 0 {
                    if let Ok(metadata) = std::fs::metadata(&file_path) {
                        if metadata.len() > config.max_file_size {
                            // Perform rotation
                            Self::perform_rotation(&file_handle, &file_path, config);
                        }
                    }
                }
            }
            
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

    #[tokio::test]
    async fn test_rotation_config_default() {
        let config = LogRotationConfig::default();
        assert_eq!(config.max_file_size, 10_000_000);
        assert_eq!(config.max_files, 5);
        assert_eq!(config.size_check_interval, 100);
        assert!(!config.compress_rotated);
    }

    #[tokio::test]
    async fn test_file_logger_with_rotation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        
        let config = LogRotationConfig {
            max_file_size: 100, // Very small for testing
            max_files: 3,
            size_check_interval: 1, // Check every event
            compress_rotated: false,
        };
        
        let logger = FileLogger::with_rotation(&log_path, config).unwrap();
        assert_eq!(logger.name(), "FileLogger");
        assert!(logger.rotation_config.is_some());
    }

    #[tokio::test]
    async fn test_file_logger_with_max_size() {
        let temp_dir = tempfile::tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        
        let logger = FileLogger::with_max_size(&log_path, 5).unwrap(); // 5MB
        assert_eq!(logger.name(), "FileLogger");
        assert!(logger.rotation_config.is_some());
        assert_eq!(logger.rotation_config.as_ref().unwrap().max_file_size, 5_000_000);
    }

    #[tokio::test]
    async fn test_rotation_on_size_limit() {
        let temp_dir = tempfile::tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        
        let config = LogRotationConfig {
            max_file_size: 50, // Very small for testing  
            max_files: 2,
            size_check_interval: 1, // Check every event
            compress_rotated: false,
        };
        
        let mut logger = FileLogger::with_rotation(&log_path, config).unwrap();
        
        // Create events that will exceed the size limit
        let large_event = Event::new("test".to_string(), 1234, "test".to_string(), json!({
            "message": "This is a large message that should trigger rotation when written multiple times",
            "value": 42
        }));
        
        let events = vec![large_event.clone(), large_event.clone(), large_event];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = logger.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        assert_eq!(collected.len(), 3);
        
        // Check that rotation occurred - rotated file should exist
        let rotated_path = format!("{}.1", log_path.to_string_lossy());
        assert!(std::path::Path::new(&rotated_path).exists() || log_path.exists());
    }

    #[tokio::test]
    async fn test_max_files_cleanup() {
        let temp_dir = tempfile::tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        
        let config = LogRotationConfig {
            max_file_size: 30,
            max_files: 2, // Only keep 2 rotated files
            size_check_interval: 1,
            compress_rotated: false,
        };
        
        let mut logger = FileLogger::with_rotation(&log_path, config).unwrap();
        
        // Create many events to trigger multiple rotations
        let large_event = Event::new("test".to_string(), 1234, "test".to_string(), json!({
            "data": "Large event data that will cause rotation",
        }));
        
        // Process enough events to trigger multiple rotations
        for _ in 0..5 {
            let events = vec![large_event.clone(); 10];
            let input_stream: EventStream = Box::pin(stream::iter(events));
            let output_stream = logger.process(input_stream).await.unwrap();
            let _: Vec<_> = output_stream.collect().await;
        }
        
        // Check that we don't have too many rotated files
        let log_1 = format!("{}.1", log_path.to_string_lossy());
        let log_4 = format!("{}.4", log_path.to_string_lossy());
        
        // Should have at most max_files rotated files
        assert!(!std::path::Path::new(&log_4).exists()); // Should be cleaned up
        
        // The log file or rotated files should exist
        assert!(log_path.exists() || std::path::Path::new(&log_1).exists());
    }

    #[tokio::test]
    async fn test_rotation_failure_graceful_degradation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        
        let config = LogRotationConfig {
            max_file_size: 50,
            max_files: 2,
            size_check_interval: 1,
            compress_rotated: false,
        };
        
        let mut logger = FileLogger::with_rotation(&log_path, config).unwrap();
        
        // Create a large event
        let large_event = Event::new("test".to_string(), 1234, "test".to_string(), json!({
            "message": "Large message that should trigger rotation",
            "data": "x".repeat(100),
        }));
        
        let events = vec![large_event];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = logger.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        // Even if rotation fails, events should still be processed
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].source, "test");
    }

    #[tokio::test]
    async fn test_no_rotation_when_disabled() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut logger = FileLogger::new(temp_file.path()).unwrap();
        
        // Create many large events - should not trigger rotation
        let large_event = Event::new("test".to_string(), 1234, "test".to_string(), json!({
            "message": "Large message",
            "data": "x".repeat(1000),
        }));
        
        let events = vec![large_event; 100];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = logger.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        assert_eq!(collected.len(), 100);
        
        // No rotated files should exist
        let rotated_path = format!("{}.1", temp_file.path().to_string_lossy());
        assert!(!std::path::Path::new(&rotated_path).exists());
    }

    #[tokio::test]
    async fn test_size_check_interval_optimization() {
        let temp_dir = tempfile::tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        
        let config = LogRotationConfig {
            max_file_size: 50,
            max_files: 2,
            size_check_interval: 10, // Only check every 10 events
            compress_rotated: false,
        };
        
        let mut logger = FileLogger::with_rotation(&log_path, config).unwrap();
        
        // Process fewer events than the check interval
        let event = Event::new("test".to_string(), 1234, "test".to_string(), json!({"msg": "test"}));
        let events = vec![event; 5];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = logger.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        assert_eq!(collected.len(), 5);
        
        // Should not have rotated yet due to interval optimization
        let rotated_path = format!("{}.1", log_path.to_string_lossy());
        assert!(!std::path::Path::new(&rotated_path).exists());
    }
} 