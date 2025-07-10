use super::{Analyzer, AnalyzerError};
use crate::framework::runners::EventStream;
use async_trait::async_trait;
use futures::stream::StreamExt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Output analyzer that provides real-time formatted event output
pub struct OutputAnalyzer {
    name: String,
    show_timestamps: bool,
    show_runner_id: bool,
    format_json: bool,
}

impl OutputAnalyzer {
    /// Create a new OutputAnalyzer with default formatting
    pub fn new() -> Self {
        Self {
            name: "output".to_string(),
            show_timestamps: true,
            show_runner_id: true,
            format_json: true,
        }
    }

    /// Create a new OutputAnalyzer with custom formatting options
    pub fn new_with_options(
        show_timestamps: bool,
        show_runner_id: bool,
        format_json: bool,
    ) -> Self {
        Self {
            name: "output".to_string(),
            show_timestamps,
            show_runner_id,
            format_json,
        }
    }

    /// Create a simple OutputAnalyzer that just prints raw JSON
    pub fn new_simple() -> Self {
        Self {
            name: "output".to_string(),
            show_timestamps: false,
            show_runner_id: false,
            format_json: false,
        }
    }

    fn format_timestamp(timestamp: u64) -> String {
        let datetime = SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(timestamp);
        match datetime.duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let secs = duration.as_secs();
                let micros = duration.subsec_micros();
                format!("{}.{:06}", secs, micros)
            }
            Err(_) => timestamp.to_string(),
        }
    }
}

impl Default for OutputAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Analyzer for OutputAnalyzer {
    async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
        let show_timestamps = self.show_timestamps;
        let show_runner_id = self.show_runner_id;
        let format_json = self.format_json;
        
        let processed_stream = stream.map(move |event| {
            // Build the output string
            let mut output_parts = Vec::new();
            
            // Add timestamp if enabled
            if show_timestamps {
                let timestamp_str = Self::format_timestamp(event.timestamp);
                output_parts.push(format!("[{}]", timestamp_str));
            }
            
            // Add runner ID if enabled
            if show_runner_id {
                output_parts.push(format!("[{}]", event.source));
            }
            
            // Add event type
            output_parts.push(format!("[{}]", event.event_type));
            
            // Format the main content
            let content = if format_json {
                match serde_json::to_string_pretty(&event.data) {
                    Ok(json) => json,
                    Err(e) => {
                        eprintln!("Error formatting event JSON: {}", e);
                        format!("{:?}", event.data)
                    }
                }
            } else {
                match event.to_json() {
                    Ok(json) => json,
                    Err(e) => {
                        eprintln!("Error serializing event to JSON: {}", e);
                        format!("{:?}", event)
                    }
                }
            };
            
            // Print the formatted output
            if output_parts.is_empty() {
                println!("{}", content);
            } else {
                println!("{} {}", output_parts.join(" "), content);
            }
            
            // Flush stdout to ensure immediate output
            use std::io::{self, Write};
            let _ = io::stdout().flush();
            
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

    #[tokio::test]
    async fn test_output_analyzer_passthrough() {
        let mut analyzer = OutputAnalyzer::new_simple(); // Simple format to avoid timestamp issues in tests
        
        let events = vec![
            Event::new("test-runner".to_string(), "event1".to_string(), json!({"data": 1})),
            Event::new("test-runner".to_string(), "event2".to_string(), json!({"data": 2})),
        ];
        
        let input_stream: EventStream = Box::pin(stream::iter(events.clone()));
        let output_stream = analyzer.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0].data, json!({"data": 1}));
        assert_eq!(collected[1].data, json!({"data": 2}));
    }

    #[tokio::test]
    async fn test_output_analyzer_name() {
        let analyzer = OutputAnalyzer::new();
        assert_eq!(analyzer.name(), "output");
    }
} 