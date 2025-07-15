use super::{Analyzer, AnalyzerError};
use crate::framework::runners::EventStream;
use async_trait::async_trait;
use futures::stream::StreamExt;
use log::debug;

/// Output analyzer that provides real-time formatted event output
pub struct OutputAnalyzer {
    name: String,
}

impl OutputAnalyzer {
    /// Create a new OutputAnalyzer with default formatting
    pub fn new() -> Self {
        Self {
            name: "output".to_string(),
        }
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

impl Default for OutputAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Analyzer for OutputAnalyzer {
    async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
        let processed_stream = stream.map(move |event| {
            debug!("OutputAnalyzer: Processing event: {:?}", event);
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
            
            // Print the formatted output immediately
            println!("{}", event_json);
            
            // Flush stdout immediately to ensure real-time output
            use std::io::{self, Write};
            if let Err(e) = io::stdout().flush() {
                eprintln!("Warning: Failed to flush stdout: {}", e);
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

    #[tokio::test]
    async fn test_output_analyzer_passthrough() {
        let mut analyzer = OutputAnalyzer::new(); // Simple format to avoid timestamp issues in tests
        
        let events = vec![
            Event::new("test-runner".to_string(), 1234, "test-runner".to_string(), json!({"data": 1})),
            Event::new("test-runner".to_string(), 1234, "test-runner".to_string(), json!({"data": 2})),
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

    #[tokio::test]
    async fn test_output_analyzer_with_binary_data() {
        let mut analyzer = OutputAnalyzer::new();
        
        // Create an event with binary data
        let binary_data = String::from_utf8_lossy(&[0x00, 0x01, 0x02, 0xFF, 0xFE]).to_string();
        let test_event = Event::new("ssl".to_string(), 1234, "ssl".to_string(), json!({
            "data": binary_data,
            "len": 5
        }));
        
        let events = vec![test_event];
        let input_stream: EventStream = Box::pin(stream::iter(events));
        let output_stream = analyzer.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].source, "ssl");
    }
} 