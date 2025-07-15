# SSL Filter Implementation

## Overview
Added a new SSL filter analyzer that filters SSL events based on configurable expressions, similar to the HTTP filter but designed specifically for raw SSL traffic data.

## Features

### 1. SSL Event Filtering
Filters SSL events based on various fields:
- `data`: SSL payload content (using exact match, contains, etc.)
- `function`: SSL operation type (READ/RECV, WRITE/SEND, etc.)
- `comm`: Process command name
- `is_handshake`: Boolean handshake flag
- `truncated`: Boolean truncation flag
- `len`, `pid`, `tid`, `uid`: Numeric fields with comparison operators
- `latency_ms`: Float latency with comparison operators
- `timestamp_ns`: Timestamp with comparison operators

### 2. Expression Syntax
```
# Field matching
data=0\r\n\r\n                 # Exact match
data~chunked                   # Contains
function=READ/RECV             # Exact function match
len<10                         # Numeric comparison
latency_ms>1.5                 # Float comparison

# Logical operators
data~chunked&function=READ/RECV    # AND operation
len<10|len>1000                    # OR operation

# Comparison operators
=   # Exact match
!=  # Not equal
>   # Greater than
<   # Less than  
>=  # Greater than or equal
<=  # Less than or equal
~   # Contains
```

### 3. CLI Integration
Added to the SSL command in main.rs:
```bash
# Filter SSL events with small payloads
cargo run ssl --ssl-filter "len<10"

# Filter specific SSL operations  
cargo run ssl --ssl-filter "function=READ/RECV"

# Complex filtering
cargo run ssl --ssl-filter "data~0&function=READ/RECV" --ssl-filter "len>1000"
```

### 4. Metrics Tracking
- `total_events_processed`: Total SSL events processed
- `filtered_events_count`: Events filtered out
- `passed_events_count`: Events that passed through
- Thread-safe atomic counters
- Real-time filter rate calculation

## Usage Examples

### Filter Small SSL Payloads
```bash
cargo run ssl --ssl-filter "len<10"
```
Filters out SSL events with data length less than 10 bytes (e.g., connection teardown messages).

### Filter SSL Handshake Events
```bash
cargo run ssl --ssl-filter "is_handshake=true"
```

### Filter by Process
```bash
cargo run ssl --ssl-filter "comm=curl"
```

### Complex Filtering
```bash
cargo run ssl --ssl-filter "data~0&function=READ/RECV" --http-parser
```
Filters SSL events containing "0" in data during READ operations, then processes remaining events through HTTP parser.

### Multiple Filters
```bash
cargo run ssl --ssl-filter "len<5" --ssl-filter "latency_ms>10.0"
```

## Architecture

### Filter Placement
SSL filter is placed **immediately after the SSL runner** and **before other analyzers**:
```
SSL Runner → SSL Filter → SSE Processor → HTTP Parser → HTTP Filter → Output
```

This ensures:
1. Raw SSL events are filtered first
2. Downstream analyzers only process relevant events
3. Performance optimization by early filtering
4. Accurate metrics for SSL-level filtering

### Data Structure
```rust
pub struct SSLFilter {
    name: String,
    exclude_patterns: Vec<String>,
    filters: Vec<FilterExpression>,
    debug: bool,
    // Thread-safe metrics
    total_events_processed: Arc<AtomicU64>,
    filtered_events_count: Arc<AtomicU64>,
    passed_events_count: Arc<AtomicU64>,
}
```

### Expression Parsing
Supports hierarchical expression parsing:
- OR operations (lowest precedence)
- AND operations (higher precedence)  
- Parentheses for grouping
- Multiple comparison operators
- Type-aware comparisons (string, numeric, boolean, float)

## Implementation Details

### 1. Filter Expression Engine
```rust
pub enum FilterNode {
    And(Box<FilterNode>, Box<FilterNode>),
    Or(Box<FilterNode>, Box<FilterNode>),
    Condition { field: String, operator: String, value: String },
    Empty,
}
```

### 2. SSL Event Field Access
Directly accesses SSL event data structure:
```json
{
  "data": "0\r\n\r\n",
  "function": "READ/RECV", 
  "comm": "claude",
  "pid": 78948,
  "tid": 78948,
  "len": 5,
  "latency_ms": 0.037,
  "is_handshake": false,
  "truncated": false,
  "uid": 1000,
  "timestamp_ns": 14932791543817
}
```

### 3. Console Output Integration
Updates status messages to show SSL filtering:
```
Starting SSL event stream with SSL filtering, with SSE processing, HTTP parsing enabled (press Ctrl+C to stop):
Starting SSL event stream with SSL filtering and SSE processing enabled (press Ctrl+C to stop):
Starting SSL event stream with SSL filtering and raw JSON output (press Ctrl+C to stop):
```

## Performance Considerations

1. **Early Filtering**: SSL events filtered before expensive operations
2. **Atomic Counters**: Minimal overhead for metrics tracking
3. **Expression Caching**: Compiled expressions reused across events
4. **Stream Processing**: Non-blocking async filtering

## Testing

Comprehensive test suite covering:
- Expression parsing for different operators
- SSL field matching (function, data content, numeric fields)
- Complex logical expressions (AND/OR)
- Metrics tracking and calculation
- Edge cases and error handling

```rust
#[test]
fn test_ssl_complex_expressions() {
    let filter = FilterExpression::parse("data~chunked&function=READ/RECV");
    // ... test implementation
}
```

## Example SSL Log Entry
The filter works with SSL events like:
```json
{
  "comm": "claude",
  "data": {
    "comm": "claude", 
    "data": "0\r\n\r\n",
    "function": "READ/RECV",
    "is_handshake": false,
    "latency_ms": 0.037,
    "len": 5,
    "pid": 78948,
    "tid": 78948,
    "timestamp_ns": 14932791543817,
    "truncated": false,
    "uid": 1000
  },
  "pid": 78948,
  "source": "ssl", 
  "timestamp": 14932791543817
}
```

## Integration Benefits

1. **Noise Reduction**: Filter out connection teardown messages (`data=0\r\n\r\n`)
2. **Process Isolation**: Focus on specific applications (`comm=chrome`)
3. **Performance Filtering**: Remove small keepalive messages (`len<10`)
4. **Operation Filtering**: Separate reads from writes (`function=READ/RECV`)
5. **Latency Analysis**: Focus on slow operations (`latency_ms>5.0`)

This SSL filter provides powerful capabilities for reducing noise and focusing analysis on relevant SSL traffic patterns.