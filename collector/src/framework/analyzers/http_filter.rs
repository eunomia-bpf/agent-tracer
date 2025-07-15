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
        
        let filtered_stream = stream.filter_map(move |event| {
            let filters = filters.clone();
            
            async move {
                // Create a temporary HTTPFilter to use the filtering logic
                let temp_filter = HTTPFilter {
                    name: "temp".to_string(),
                    exclude_patterns: Vec::new(),
                    filters,
                    debug,
                };

                // If event should be filtered, return None (filter out)
                if temp_filter.should_filter_event(&event) {
                    None
                } else {
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
}