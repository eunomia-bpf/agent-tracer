use super::{Analyzer, AnalyzerError};
use crate::framework::runners::EventStream;
use async_trait::async_trait;
use futures::stream::StreamExt;

/// Raw analyzer that outputs events as JSON to stdout and passes them through
pub struct RawAnalyzer {
    name: String,
    print_to_stdout: bool,
}

impl RawAnalyzer {
    /// Create a new RawAnalyzer that prints to stdout
    pub fn new() -> Self {
        Self {
            name: "raw".to_string(),
            print_to_stdout: true,
        }
    }

    /// Create a new RawAnalyzer with custom settings
    #[allow(dead_code)]
    pub fn new_with_options(print_to_stdout: bool) -> Self {
        Self {
            name: "raw".to_string(),
            print_to_stdout,
        }
    }
}

impl Default for RawAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Analyzer for RawAnalyzer {
    async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
        let print_to_stdout = self.print_to_stdout;
        
        let processed_stream = stream.map(move |event| {
            if print_to_stdout {
                // Print the event as raw JSON
                match event.to_json() {
                    Ok(json) => println!("{}", json),
                    Err(e) => eprintln!("Error serializing event to JSON: {}", e),
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

    #[tokio::test]
    async fn test_raw_analyzer_passthrough() {
        let mut analyzer = RawAnalyzer::new_with_options(false); // Don't print to stdout in tests
        
        let events = vec![
            Event::new("test".to_string(), json!({"data": 1})),
            Event::new("test".to_string(), json!({"data": 2})),
        ];
        
        let input_stream: EventStream = Box::pin(stream::iter(events.clone()));
        let output_stream = analyzer.process(input_stream).await.unwrap();
        
        let collected: Vec<_> = output_stream.collect().await;
        
        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0].data, json!({"data": 1}));
        assert_eq!(collected[1].data, json!({"data": 2}));
    }

    #[tokio::test]
    async fn test_raw_analyzer_name() {
        let analyzer = RawAnalyzer::new();
        assert_eq!(analyzer.name(), "raw");
    }
} 