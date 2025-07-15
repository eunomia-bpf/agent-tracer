use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Core event structure for the observability framework
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub timestamp: u64,
    pub source: String,
    pub data: serde_json::Value,
}

impl Event {

    /// Create a new event with custom timestamp
    pub fn new_with_timestamp(
        timestamp: u64,
        source: String,
        data: serde_json::Value,
    ) -> Self {
        Self {
            timestamp,
            source,
            data,
        }
    }

    /// Get the event timestamp as a DateTime<Utc>
    pub fn datetime(&self) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(self.timestamp as i64)
            .unwrap_or_else(|| Utc::now())
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize this event to pretty-printed JSON
    #[allow(dead_code)]
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize an event from JSON string
    #[allow(dead_code)]
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {}: {}",
            self.datetime().format("%Y-%m-%d %H:%M:%S%.3f"),
            self.source,
            self.data
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_event_creation() {
        let data = json!({"key": "value", "number": 42});
        let event = Event::new("test-source".to_string(), data.clone());

        assert!(event.timestamp > 0);
        assert_eq!(event.source, "test-source");
        assert_eq!(event.data, data);
    }

    #[test]
    fn test_event_with_custom_timestamp() {
        let data = json!({"test": true});
        let custom_timestamp = 1234567890u64;

        let event = Event::new_with_timestamp(
            custom_timestamp,
            "custom-source".to_string(),
            data.clone(),
        );

        assert_eq!(event.timestamp, custom_timestamp);
        assert_eq!(event.source, "custom-source");
        assert_eq!(event.data, data);
    }

    #[test]
    fn test_event_json_serialization() {
        let data = json!({"message": "hello world"});
        let event = Event::new_with_timestamp(
            1000,
            "test".to_string(),
            data,
        );

        let json_str = event.to_json().unwrap();
        let deserialized = Event::from_json(&json_str).unwrap();

        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_event_display() {
        let data = json!({"msg": "test"});
        let event = Event::new_with_timestamp(
            1609459200000, // 2021-01-01 00:00:00 UTC
            "test-source".to_string(),
            data,
        );

        let display_str = format!("{}", event);
        assert!(display_str.contains("test-source"));
        assert!(display_str.contains("2021"));
    }
} 