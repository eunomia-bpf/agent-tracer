use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

/// Core event structure for the observability framework
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub id: String,
    pub timestamp: u64,
    pub source: String,
    pub data: serde_json::Value,
}

impl Event {
    /// Create a new event with auto-generated ID and current timestamp
    #[allow(dead_code)]
    pub fn new(source: String, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            source,
            data,
        }
    }

    /// Create a new event with custom ID and timestamp
    pub fn new_with_id_and_timestamp(
        id: String,
        timestamp: u64,
        source: String,
        data: serde_json::Value,
    ) -> Self {
        Self {
            id,
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
            "[{}] {} ({}): {}",
            self.datetime().format("%Y-%m-%d %H:%M:%S%.3f"),
            self.source,
            self.id,
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

        assert!(!event.id.is_empty());
        assert!(event.timestamp > 0);
        assert_eq!(event.source, "test-source");
        assert_eq!(event.data, data);
    }

    #[test]
    fn test_event_with_custom_id_and_timestamp() {
        let data = json!({"test": true});
        let custom_id = "custom-id-123".to_string();
        let custom_timestamp = 1234567890u64;

        let event = Event::new_with_id_and_timestamp(
            custom_id.clone(),
            custom_timestamp,
            "custom-source".to_string(),
            data.clone(),
        );

        assert_eq!(event.id, custom_id);
        assert_eq!(event.timestamp, custom_timestamp);
        assert_eq!(event.source, "custom-source");
        assert_eq!(event.data, data);
    }

    #[test]
    fn test_event_json_serialization() {
        let data = json!({"message": "hello world"});
        let event = Event::new_with_id_and_timestamp(
            "test-id".to_string(),
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
        let event = Event::new_with_id_and_timestamp(
            "test-id".to_string(),
            1609459200000, // 2021-01-01 00:00:00 UTC
            "test-source".to_string(),
            data,
        );

        let display_str = format!("{}", event);
        assert!(display_str.contains("test-source"));
        assert!(display_str.contains("test-id"));
        assert!(display_str.contains("2021"));
    }
} 