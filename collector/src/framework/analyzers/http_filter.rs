use super::{Analyzer, AnalyzerError};
use crate::framework::runners::EventStream;
use crate::framework::core::Event;
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde_json::Value;

/// HTTP Filter Analyzer that filters HTTP parser events based on configurable expressions
/// Similar to Python filter_expression.py but integrated into the Rust framework
pub struct HTTPFilter {
    name: String,
    /// Filter expressions to exclude events
    exclude_patterns: Vec<String>,
    /// Compiled filter expressions
    filters: Vec<FilterExpression>,
    /// Debug mode
    debug: bool,
    /// Metrics (shared atomic counters for thread safety)
    total_events_processed: std::sync::Arc<std::sync::atomic::AtomicU64>,
    filtered_events_count: std::sync::Arc<std::sync::atomic::AtomicU64>,
    passed_events_count: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

/// A single filter expression that can evaluate HTTP events
#[derive(Debug, Clone)]
pub struct FilterExpression {
    /// Original expression string
    expression: String,
    /// Parsed expression tree
    parsed: FilterNode,
}

/// Node in the filter expression tree
#[derive(Debug, Clone)]
pub enum FilterNode {
    /// Logical AND operation
    And(Vec<FilterNode>),
    /// Logical OR operation
    Or(Vec<FilterNode>),
    /// Single condition
    Condition {
        target: String,      // "request" or "response"
        field: String,       // "method", "path", "status_code", etc.
        operator: String,    // "=", "contains", "prefix", etc.
        value: String,       // Expected value
    },
    /// Empty filter (matches nothing)
    Empty,
}

impl HTTPFilter {
    /// Create a new HTTP filter with no patterns (passes everything through)
    pub fn new() -> Self {
        HTTPFilter {
            name: "HTTPFilter".to_string(),
            exclude_patterns: Vec::new(),
            filters: Vec::new(),
            debug: false,
            total_events_processed: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            filtered_events_count: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            passed_events_count: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Create a new HTTP filter with exclude patterns
    pub fn with_patterns(patterns: Vec<String>) -> Self {
        let mut filter = HTTPFilter::new();
        filter.exclude_patterns = patterns.clone();
        filter.filters = patterns.into_iter()
            .map(|p| FilterExpression::parse(&p))
            .collect();
        filter
    }


    /// Check if an HTTP parser event should be filtered out
    fn should_filter_event(&self, event: &Event) -> bool {
        if self.filters.is_empty() {
            return false;
        }

        let data = &event.data;
        
        // Only filter http_parser events
        if event.source != "http_parser" {
            return false;
        }

        // Evaluate each filter expression
        for filter in &self.filters {
            if filter.evaluate(data) {
                if self.debug {
                    eprintln!("[HTTPFilter DEBUG] Event filtered by: {}", filter.expression);
                }
                return true;
            }
        }

        false
    }

    /// Get filtering metrics
    pub fn get_metrics(&self) -> FilterMetrics {
        FilterMetrics {
            total_events_processed: self.total_events_processed.load(std::sync::atomic::Ordering::Relaxed),
            filtered_events_count: self.filtered_events_count.load(std::sync::atomic::Ordering::Relaxed),
            passed_events_count: self.passed_events_count.load(std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// Reset metrics counters
    pub fn reset_metrics(&self) {
        self.total_events_processed.store(0, std::sync::atomic::Ordering::Relaxed);
        self.filtered_events_count.store(0, std::sync::atomic::Ordering::Relaxed);
        self.passed_events_count.store(0, std::sync::atomic::Ordering::Relaxed);
    }

    /// Print current metrics to stderr
    pub fn print_metrics(&self) {
        let metrics = self.get_metrics();
        eprintln!("[HTTPFilter Metrics] Total: {}, Filtered: {}, Passed: {}", 
                  metrics.total_events_processed, 
                  metrics.filtered_events_count, 
                  metrics.passed_events_count);
    }

    /// Enable debug mode
    pub fn with_debug(mut self) -> Self {
        self.debug = true;
        self
    }
}

/// Metrics for HTTP filtering
#[derive(Debug, Clone)]
pub struct FilterMetrics {
    pub total_events_processed: u64,
    pub filtered_events_count: u64,
    pub passed_events_count: u64,
}

impl FilterMetrics {
    /// Calculate the filter rate as a percentage
    pub fn filter_rate(&self) -> f64 {
        if self.total_events_processed == 0 {
            0.0
        } else {
            (self.filtered_events_count as f64 / self.total_events_processed as f64) * 100.0
        }
    }

    /// Calculate the pass rate as a percentage
    pub fn pass_rate(&self) -> f64 {
        if self.total_events_processed == 0 {
            0.0
        } else {
            (self.passed_events_count as f64 / self.total_events_processed as f64) * 100.0
        }
    }
}

impl FilterExpression {
    /// Parse a filter expression string
    pub fn parse(expression: &str) -> Self {
        let trimmed = expression.trim();
        if trimmed.is_empty() {
            return FilterExpression {
                expression: expression.to_string(),
                parsed: FilterNode::Empty,
            };
        }

        let parsed = Self::parse_or_expression(trimmed);
        FilterExpression {
            expression: expression.to_string(),
            parsed,
        }
    }

    /// Parse OR expressions (lowest precedence)
    fn parse_or_expression(expr: &str) -> FilterNode {
        let or_parts: Vec<&str> = expr.split('|').map(|s| s.trim()).collect();
        
        if or_parts.len() > 1 {
            let conditions: Vec<FilterNode> = or_parts.into_iter()
                .map(|part| Self::parse_and_expression(part))
                .collect();
            FilterNode::Or(conditions)
        } else {
            Self::parse_and_expression(expr)
        }
    }

    /// Parse AND expressions (higher precedence)
    fn parse_and_expression(expr: &str) -> FilterNode {
        let and_parts: Vec<&str> = expr.split('&').map(|s| s.trim()).collect();
        
        if and_parts.len() > 1 {
            let conditions: Vec<FilterNode> = and_parts.into_iter()
                .map(|part| Self::parse_condition(part))
                .collect();
            FilterNode::And(conditions)
        } else {
            Self::parse_condition(expr)
        }
    }

    /// Parse a single condition
    fn parse_condition(condition: &str) -> FilterNode {
        let condition = condition.trim();
        
        if !condition.contains('=') {
            // Simple path containment (legacy)
            return FilterNode::Condition {
                target: "request".to_string(),
                field: "path".to_string(),
                operator: "contains".to_string(),
                value: condition.to_string(),
            };
        }

        let parts: Vec<&str> = condition.splitn(2, '=').collect();
        if parts.len() != 2 {
            return FilterNode::Empty;
        }

        let key = parts[0].trim();
        let value = parts[1].trim();

        // Parse dot notation (request.path, response.status_code)
        if key.contains('.') {
            let key_parts: Vec<&str> = key.splitn(2, '.').collect();
            if key_parts.len() == 2 {
                let target = key_parts[0].trim();
                let field = key_parts[1].trim();
                
                let (target, operator) = if target == "request" || target == "req" {
                    let op = match field {
                        "path_prefix" | "path_starts_with" => "prefix",
                        "path_contains" | "path_includes" => "contains",
                        "path" | "path_exact" => "exact",
                        _ => "exact",
                    };
                    ("request", op)
                } else if target == "response" || target == "resp" || target == "res" {
                    ("response", "exact")
                } else {
                    ("request", "exact")
                };

                return FilterNode::Condition {
                    target: target.to_string(),
                    field: field.to_string(),
                    operator: operator.to_string(),
                    value: value.to_string(),
                };
            }
        }

        // Legacy format (assume request)
        let operator = match key {
            "path_prefix" | "path_starts_with" => "prefix",
            "path_contains" | "path_includes" => "contains",
            "path" | "path_exact" => "exact",
            _ => "exact",
        };

        FilterNode::Condition {
            target: "request".to_string(),
            field: key.to_string(),
            operator: operator.to_string(),
            value: value.to_string(),
        }
    }

    /// Evaluate the filter expression against event data
    pub fn evaluate(&self, data: &Value) -> bool {
        self.evaluate_node(&self.parsed, data)
    }

    /// Evaluate a filter node
    fn evaluate_node(&self, node: &FilterNode, data: &Value) -> bool {
        match node {
            FilterNode::Empty => false,
            FilterNode::And(conditions) => {
                conditions.iter().all(|c| self.evaluate_node(c, data))
            }
            FilterNode::Or(conditions) => {
                conditions.iter().any(|c| self.evaluate_node(c, data))
            }
            FilterNode::Condition { target, field, operator, value } => {
                self.evaluate_condition(target, field, operator, value, data)
            }
        }
    }

    /// Evaluate a single condition
    fn evaluate_condition(&self, target: &str, field: &str, operator: &str, value: &str, data: &Value) -> bool {
        let message_type = data.get("message_type").and_then(|v| v.as_str()).unwrap_or("");
        
        // Check if the data type matches the target
        let matches_target = match target {
            "request" => message_type == "request",
            "response" => message_type == "response",
            _ => false,
        };

        if !matches_target {
            return false;
        }

        if target == "request" {
            self.evaluate_request_condition(field, operator, value, data)
        } else if target == "response" {
            self.evaluate_response_condition(field, operator, value, data)
        } else {
            false
        }
    }

    /// Evaluate request conditions
    fn evaluate_request_condition(&self, field: &str, operator: &str, value: &str, data: &Value) -> bool {
        match field {
            "method" | "verb" => {
                let method = data.get("method").and_then(|v| v.as_str()).unwrap_or("");
                method.to_uppercase() == value.to_uppercase()
            }
            "path" | "path_exact" => {
                let path = data.get("path").and_then(|v| v.as_str()).unwrap_or("");
                match operator {
                    "prefix" => path.starts_with(value),
                    "contains" => path.contains(value),
                    "exact" | _ => path == value,
                }
            }
            "path_prefix" | "path_starts_with" => {
                let path = data.get("path").and_then(|v| v.as_str()).unwrap_or("");
                path.starts_with(value)
            }
            "path_contains" | "path_includes" => {
                let path = data.get("path").and_then(|v| v.as_str()).unwrap_or("");
                path.contains(value)
            }
            "host" | "hostname" => {
                let empty_map = serde_json::Map::new();
                let headers = data.get("headers").and_then(|v| v.as_object()).unwrap_or(&empty_map);
                let host = headers.get("host").and_then(|v| v.as_str()).unwrap_or("");
                host == value
            }
            "body" | "body_contains" => {
                let body = data.get("body").and_then(|v| v.as_str()).unwrap_or("");
                body.contains(value)
            }
            _ => {
                // Try as query parameter
                let path = data.get("path").and_then(|v| v.as_str()).unwrap_or("");
                if let Some(query_start) = path.find('?') {
                    let query = &path[query_start + 1..];
                    let param_pattern = format!("{}={}", field, value);
                    query.contains(&param_pattern)
                } else {
                    false
                }
            }
        }
    }

    /// Evaluate response conditions
    fn evaluate_response_condition(&self, field: &str, _operator: &str, value: &str, data: &Value) -> bool {
        match field {
            "status_code" | "status" | "code" => {
                let status_code = data.get("status_code").and_then(|v| v.as_u64()).unwrap_or(0);
                if let Ok(target_code) = value.parse::<u64>() {
                    status_code == target_code
                } else {
                    false
                }
            }
            "status_text" | "status_message" => {
                let status_text = data.get("status_text").and_then(|v| v.as_str()).unwrap_or("");
                status_text.to_lowercase().contains(&value.to_lowercase())
            }
            "content_type" | "content-type" => {
                let empty_map = serde_json::Map::new();
                let headers = data.get("headers").and_then(|v| v.as_object()).unwrap_or(&empty_map);
                let content_type = headers.get("content-type").and_then(|v| v.as_str()).unwrap_or("");
                content_type.contains(value)
            }
            "server" => {
                let empty_map = serde_json::Map::new();
                let headers = data.get("headers").and_then(|v| v.as_object()).unwrap_or(&empty_map);
                let server = headers.get("server").and_then(|v| v.as_str()).unwrap_or("");
                server.contains(value)
            }
            "body" | "body_contains" => {
                let body = data.get("body").and_then(|v| v.as_str()).unwrap_or("");
                body.contains(value)
            }
            _ => {
                // Try as response header
                let empty_map = serde_json::Map::new();
                let headers = data.get("headers").and_then(|v| v.as_object()).unwrap_or(&empty_map);
                let header_value = headers.get(field).and_then(|v| v.as_str()).unwrap_or("");
                header_value.contains(value)
            }
        }
    }
}

#[async_trait]
impl Analyzer for HTTPFilter {
    async fn process(&mut self, stream: EventStream) -> Result<EventStream, AnalyzerError> {
        let filters = self.filters.clone();
        let debug = self.debug;
        
        // Clone the shared atomic counters for use in the stream
        let total_counter = self.total_events_processed.clone();
        let filtered_counter = self.filtered_events_count.clone();
        let passed_counter = self.passed_events_count.clone();
        
        let filtered_stream = stream.filter_map(move |event| {
            let filters = filters.clone();
            let total_counter = total_counter.clone();
            let filtered_counter = filtered_counter.clone();
            let passed_counter = passed_counter.clone();
            
            async move {
                // Increment total events processed
                total_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                
                // Check if this is an HTTP parser event and should be filtered
                let should_filter = if filters.is_empty() {
                    false
                } else if event.source != "http_parser" {
                    false
                } else {
                    // Evaluate each filter expression
                    let mut filtered = false;
                    for filter in &filters {
                        if filter.evaluate(&event.data) {
                            if debug {
                                eprintln!("[HTTPFilter DEBUG] Event filtered by: {}", filter.expression);
                            }
                            filtered = true;
                            break;
                        }
                    }
                    filtered
                };

                if should_filter {
                    // Increment filtered counter
                    filtered_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    None // Filter out
                } else {
                    // Increment passed counter  
                    passed_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Some(event) // Pass through
                }
            }
        });

        Ok(Box::pin(filtered_stream))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_filter_expression_parsing() {
        let expr = FilterExpression::parse("request.path=/health");
        match expr.parsed {
            FilterNode::Condition { target, field, operator, value } => {
                assert_eq!(target, "request");
                assert_eq!(field, "path");
                assert_eq!(operator, "exact");
                assert_eq!(value, "/health");
            }
            _ => panic!("Expected single condition"),
        }
    }

    #[test]
    fn test_request_filtering() {
        let filter = FilterExpression::parse("request.method=GET");
        
        let request_data = json!({
            "message_type": "request",
            "method": "GET",
            "path": "/api/test",
            "headers": {}
        });
        
        assert!(filter.evaluate(&request_data));
        
        let post_data = json!({
            "message_type": "request",
            "method": "POST",
            "path": "/api/test",
            "headers": {}
        });
        
        assert!(!filter.evaluate(&post_data));
    }

    #[test]
    fn test_response_filtering() {
        let filter = FilterExpression::parse("response.status_code=404");
        
        let response_data = json!({
            "message_type": "response",
            "status_code": 404,
            "status_text": "Not Found",
            "headers": {}
        });
        
        assert!(filter.evaluate(&response_data));
        
        let ok_data = json!({
            "message_type": "response",
            "status_code": 200,
            "status_text": "OK",
            "headers": {}
        });
        
        assert!(!filter.evaluate(&ok_data));
    }

    #[test]
    fn test_complex_expressions() {
        let filter = FilterExpression::parse("request.method=GET | response.status_code=404");
        
        let get_request = json!({
            "message_type": "request",
            "method": "GET",
            "path": "/api/test"
        });
        
        let not_found_response = json!({
            "message_type": "response",
            "status_code": 404
        });
        
        let post_request = json!({
            "message_type": "request", 
            "method": "POST",
            "path": "/api/test"
        });
        
        assert!(filter.evaluate(&get_request));
        assert!(filter.evaluate(&not_found_response));
        assert!(!filter.evaluate(&post_request));
    }

    #[test]
    fn test_http_filter_metrics() {
        let filter = HTTPFilter::with_patterns(vec!["request.method=GET".to_string()]);
        
        // Check initial metrics
        let initial_metrics = filter.get_metrics();
        assert_eq!(initial_metrics.total_events_processed, 0);
        assert_eq!(initial_metrics.filtered_events_count, 0);
        assert_eq!(initial_metrics.passed_events_count, 0);
        assert_eq!(initial_metrics.filter_rate(), 0.0);
        assert_eq!(initial_metrics.pass_rate(), 0.0);
        
        // Test metrics calculation
        let metrics = FilterMetrics {
            total_events_processed: 100,
            filtered_events_count: 25,
            passed_events_count: 75,
        };
        
        assert_eq!(metrics.filter_rate(), 25.0);
        assert_eq!(metrics.pass_rate(), 75.0);
        
        // Test edge case - no events processed
        let empty_metrics = FilterMetrics {
            total_events_processed: 0,
            filtered_events_count: 0,
            passed_events_count: 0,
        };
        
        assert_eq!(empty_metrics.filter_rate(), 0.0);
        assert_eq!(empty_metrics.pass_rate(), 0.0);
    }
}