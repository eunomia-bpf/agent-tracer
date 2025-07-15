use super::{Analyzer, AnalyzerError};
use crate::framework::runners::EventStream;
use crate::framework::core::Event;
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde_json::Value;

/// SSL Filter Analyzer that filters SSL events based on configurable expressions
/// Filters SSL events based on data content, function, latency, etc.
pub struct SSLFilter {
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

/// A single filter expression that can evaluate SSL events
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
    And(Box<FilterNode>, Box<FilterNode>),
    /// Logical OR operation
    Or(Box<FilterNode>, Box<FilterNode>),
    /// Single condition check
    Condition {
        field: String,        // Field name (e.g., "data", "function", "latency_ms")
        operator: String,     // Operator (exact, prefix, suffix, contains, gt, lt, gte, lte)
        value: String,        // Expected value
    },
    /// Empty filter (matches nothing)
    Empty,
}

impl SSLFilter {
    /// Create a new SSL filter with no patterns (passes everything through)
    pub fn new() -> Self {
        SSLFilter {
            name: "SSLFilter".to_string(),
            exclude_patterns: Vec::new(),
            filters: Vec::new(),
            debug: false,
            total_events_processed: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            filtered_events_count: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            passed_events_count: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Create a new SSL filter with exclude patterns
    pub fn with_patterns(patterns: Vec<String>) -> Self {
        let mut filter = SSLFilter::new();
        filter.exclude_patterns = patterns.clone();
        filter.filters = patterns.into_iter()
            .map(|p| FilterExpression::parse(&p))
            .collect();
        filter
    }

    /// Check if an SSL event should be filtered out
    fn should_filter_event(&self, event: &Event) -> bool {
        if self.filters.is_empty() {
            return false;
        }

        // Only filter ssl events
        if event.source != "ssl" {
            return false;
        }

        let data = &event.data;

        // Evaluate each filter expression
        for filter in &self.filters {
            if filter.evaluate(data) {
                if self.debug {
                    eprintln!("[SSLFilter DEBUG] Event filtered by: {}", filter.expression);
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
        eprintln!("[SSLFilter Metrics] Total: {}, Filtered: {}, Passed: {}", 
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

/// Metrics for SSL filtering
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
        let parsed = Self::parse_expression(expression);
        FilterExpression {
            expression: expression.to_string(),
            parsed,
        }
    }

    /// Parse an expression string into a FilterNode tree
    fn parse_expression(expr: &str) -> FilterNode {
        let expr = expr.trim();
        
        if expr.is_empty() {
            return FilterNode::Empty;
        }

        // Handle OR operations (lowest precedence)
        if let Some(or_pos) = Self::find_operator(expr, '|') {
            let left = Self::parse_expression(&expr[..or_pos]);
            let right = Self::parse_expression(&expr[or_pos + 1..]);
            return FilterNode::Or(Box::new(left), Box::new(right));
        }

        // Handle AND operations (higher precedence)
        if let Some(and_pos) = Self::find_operator(expr, '&') {
            let left = Self::parse_expression(&expr[..and_pos]);
            let right = Self::parse_expression(&expr[and_pos + 1..]);
            return FilterNode::And(Box::new(left), Box::new(right));
        }

        // Parse single condition
        Self::parse_condition(expr)
    }

    /// Find the position of an operator at the top level (not inside parentheses)
    fn find_operator(expr: &str, op: char) -> Option<usize> {
        let mut paren_depth = 0;
        let chars: Vec<char> = expr.chars().collect();
        
        for (i, &c) in chars.iter().enumerate() {
            match c {
                '(' => paren_depth += 1,
                ')' => paren_depth -= 1,
                _ if c == op && paren_depth == 0 => return Some(i),
                _ => {}
            }
        }
        None
    }

    /// Parse a single condition like "data=0\r\n\r\n" or "function=READ/RECV"
    fn parse_condition(expr: &str) -> FilterNode {
        let expr = expr.trim();
        
        // Handle parentheses
        if expr.starts_with('(') && expr.ends_with(')') {
            return Self::parse_expression(&expr[1..expr.len()-1]);
        }

        // Find the operator
        let operators = [">=", "<=", "!=", "=", ">", "<", "~"];
        
        for &op in &operators {
            if let Some(pos) = expr.find(op) {
                let field = expr[..pos].trim().to_string();
                let value = expr[pos + op.len()..].trim().to_string();
                
                let operator = match op {
                    "=" => "exact",
                    "!=" => "not_equal",
                    ">" => "gt",
                    "<" => "lt", 
                    ">=" => "gte",
                    "<=" => "lte",
                    "~" => "contains",
                    _ => "exact",
                }.to_string();

                return FilterNode::Condition { field, operator, value };
            }
        }

        FilterNode::Empty
    }

    /// Evaluate this filter expression against SSL event data
    pub fn evaluate(&self, data: &Value) -> bool {
        self.evaluate_node(&self.parsed, data)
    }

    /// Evaluate a filter node against SSL event data
    fn evaluate_node(&self, node: &FilterNode, data: &Value) -> bool {
        match node {
            FilterNode::And(left, right) => {
                self.evaluate_node(left, data) && self.evaluate_node(right, data)
            }
            FilterNode::Or(left, right) => {
                self.evaluate_node(left, data) || self.evaluate_node(right, data)
            }
            FilterNode::Condition { field, operator, value } => {
                self.evaluate_condition(field, operator, value, data)
            }
            FilterNode::Empty => false,
        }
    }

    /// Evaluate a single condition against SSL event data
    fn evaluate_condition(&self, field: &str, operator: &str, expected: &str, data: &Value) -> bool {
        // Get the field value from SSL event data
        let field_value = match field {
            "data" => data.get("data").and_then(|v| v.as_str()),
            "function" => data.get("function").and_then(|v| v.as_str()),
            "comm" => data.get("comm").and_then(|v| v.as_str()),
            "is_handshake" => return data.get("is_handshake").and_then(|v| v.as_bool()).unwrap_or(false) == (expected == "true"),
            "truncated" => return data.get("truncated").and_then(|v| v.as_bool()).unwrap_or(false) == (expected == "true"),
            "len" | "pid" | "tid" | "uid" => {
                if let Some(num_val) = data.get(field).and_then(|v| v.as_u64()) {
                    return self.compare_numbers(num_val, operator, expected);
                }
                return false;
            }
            "latency_ms" => {
                if let Some(float_val) = data.get("latency_ms").and_then(|v| v.as_f64()) {
                    return self.compare_floats(float_val, operator, expected);
                }
                return false;
            }
            "timestamp_ns" => {
                if let Some(timestamp) = data.get("timestamp_ns").and_then(|v| v.as_u64()) {
                    return self.compare_numbers(timestamp, operator, expected);
                }
                return false;
            }
            _ => None,
        };

        if let Some(value) = field_value {
            self.compare_strings(value, operator, expected)
        } else {
            false
        }
    }

    /// Compare string values based on operator
    fn compare_strings(&self, actual: &str, operator: &str, expected: &str) -> bool {
        match operator {
            "exact" => actual == expected,
            "not_equal" => actual != expected,
            "contains" => actual.contains(expected),
            "prefix" => actual.starts_with(expected),
            "suffix" => actual.ends_with(expected),
            _ => false,
        }
    }

    /// Compare numeric values based on operator
    fn compare_numbers(&self, actual: u64, operator: &str, expected: &str) -> bool {
        if let Ok(expected_num) = expected.parse::<u64>() {
            match operator {
                "exact" => actual == expected_num,
                "not_equal" => actual != expected_num,
                "gt" => actual > expected_num,
                "lt" => actual < expected_num,
                "gte" => actual >= expected_num,
                "lte" => actual <= expected_num,
                _ => false,
            }
        } else {
            false
        }
    }

    /// Compare float values based on operator
    fn compare_floats(&self, actual: f64, operator: &str, expected: &str) -> bool {
        if let Ok(expected_num) = expected.parse::<f64>() {
            match operator {
                "exact" => (actual - expected_num).abs() < f64::EPSILON,
                "not_equal" => (actual - expected_num).abs() >= f64::EPSILON,
                "gt" => actual > expected_num,
                "lt" => actual < expected_num,
                "gte" => actual >= expected_num,
                "lte" => actual <= expected_num,
                _ => false,
            }
        } else {
            false
        }
    }
}

#[async_trait]
impl Analyzer for SSLFilter {
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
                
                // Check if this is an SSL event and should be filtered
                let should_filter = if filters.is_empty() {
                    false
                } else if event.source != "ssl" {
                    false
                } else {
                    // Evaluate each filter expression
                    let mut filtered = false;
                    for filter in &filters {
                        if filter.evaluate(&event.data) {
                            if debug {
                                eprintln!("[SSLFilter DEBUG] Event filtered by: {}", filter.expression);
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
    fn test_ssl_filter_expression_parsing() {
        let expr = FilterExpression::parse("function=READ/RECV");
        match expr.parsed {
            FilterNode::Condition { field, operator, value } => {
                assert_eq!(field, "function");
                assert_eq!(operator, "exact");
                assert_eq!(value, "READ/RECV");
            }
            _ => panic!("Expected single condition"),
        }
    }

    #[test]
    fn test_ssl_data_filtering() {
        // Use 'contains' operator for pattern matching
        let filter = FilterExpression::parse("data~chunked");
        
        let matching_event = json!({
            "data": "chunked data here",
            "function": "READ/RECV",
            "len": 5
        });
        
        let non_matching_event = json!({
            "data": "plain text response",
            "function": "READ/RECV", 
            "len": 15
        });
        
        assert!(filter.evaluate(&matching_event));
        assert!(!filter.evaluate(&non_matching_event));
    }

    #[test]
    fn test_ssl_function_filtering() {
        let filter = FilterExpression::parse("function=READ/RECV");
        
        let read_event = json!({
            "data": "some data",
            "function": "READ/RECV",
            "len": 10
        });
        
        let write_event = json!({
            "data": "some data",
            "function": "WRITE/SEND",
            "len": 10
        });
        
        assert!(filter.evaluate(&read_event));
        assert!(!filter.evaluate(&write_event));
    }

    #[test]
    fn test_ssl_numeric_filtering() {
        let filter = FilterExpression::parse("len<10");
        
        let small_event = json!({
            "data": "small",
            "function": "READ/RECV",
            "len": 5
        });
        
        let large_event = json!({
            "data": "much larger data",
            "function": "READ/RECV",
            "len": 15
        });
        
        assert!(filter.evaluate(&small_event));
        assert!(!filter.evaluate(&large_event));
    }

    #[test]
    fn test_ssl_complex_expressions() {
        let filter = FilterExpression::parse("data~chunked&function=READ/RECV");
        
        let matching_event = json!({
            "data": "chunked data here",
            "function": "READ/RECV",
            "len": 5
        });
        
        let partial_match = json!({
            "data": "chunked data here", 
            "function": "WRITE/SEND",
            "len": 5
        });
        
        let no_match = json!({
            "data": "plain text response",
            "function": "WRITE/SEND",
            "len": 15
        });
        
        assert!(filter.evaluate(&matching_event));
        assert!(!filter.evaluate(&partial_match));
        assert!(!filter.evaluate(&no_match));
    }

    #[test]
    fn test_ssl_filter_metrics() {
        let filter = SSLFilter::with_patterns(vec!["len<10".to_string()]);
        
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
            filtered_events_count: 30,
            passed_events_count: 70,
        };
        
        assert_eq!(metrics.filter_rate(), 30.0);
        assert_eq!(metrics.pass_rate(), 70.0);
    }
}