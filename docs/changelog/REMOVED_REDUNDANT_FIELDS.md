# Removed Redundant Fields from Analysis Events

## Summary
Removed duplicate `comm`, `pid`, and `timestamp_ns` fields from analysis event structs since these values can be obtained from the original event passed to `to_event()`.

## Changes Made

### SSEProcessorEvent
**Removed fields:**
- `pub comm: String`
- `pub pid: u64`

**Updated constructor signature:**
```rust
// Before
pub fn new(
    connection_id: String,
    message_id: Option<String>,
    start_time: u64,
    end_time: u64,
    original_source: String,
    function: String,
    comm: String,        // ❌ REMOVED
    pid: u64,            // ❌ REMOVED  
    tid: u64,
    json_content: String,
    text_content: String,
    total_size: usize,
    event_count: usize,
    has_message_start: bool,
    sse_events: Vec<Value>,
) -> Self

// After
pub fn new(
    connection_id: String,
    message_id: Option<String>,
    start_time: u64,
    end_time: u64,
    original_source: String,
    function: String,
    tid: u64,
    json_content: String,
    text_content: String,
    total_size: usize,
    event_count: usize,
    has_message_start: bool,
    sse_events: Vec<Value>,
) -> Self
```

**Updated to_event JSON data:**
```rust
// Removed from JSON output:
// "comm": self.comm,
// "pid": self.pid,
```

### HTTPEvent
**Removed fields:**
- `pub comm: String`
- `pub pid: u64` 
- `pub timestamp_ns: u64`

**Updated constructor signature:**
```rust
// Before
pub fn new(
    tid: u64,
    message_type: String,
    first_line: String,
    method: Option<String>,
    path: Option<String>,
    protocol: Option<String>,
    status_code: Option<u16>,
    status_text: Option<String>,
    headers: HashMap<String, String>,
    body: Option<String>,
    total_size: usize,
    has_body: bool,
    is_chunked: bool,
    content_length: Option<usize>,
    original_source: String,
    comm: String,         // ❌ REMOVED
    pid: u64,             // ❌ REMOVED
    timestamp_ns: u64,    // ❌ REMOVED
) -> Self

// After
pub fn new(
    tid: u64,
    message_type: String,
    first_line: String,
    method: Option<String>,
    path: Option<String>,
    protocol: Option<String>,
    status_code: Option<u16>,
    status_text: Option<String>,
    headers: HashMap<String, String>,
    body: Option<String>,
    total_size: usize,
    has_body: bool,
    is_chunked: bool,
    content_length: Option<usize>,
    original_source: String,
) -> Self
```

**Updated to_event JSON data:**
```rust
// Removed from JSON output:
// "comm": self.comm,
// "pid": self.pid,
// "timestamp_ns": self.timestamp_ns,
```

**Updated timestamp logic:**
```rust
// Before
Event::new_with_timestamp(
    self.timestamp_ns / 1_000_000,  // Convert ns to ms
    "http_parser".to_string(), 
    original_event.pid, 
    original_event.comm.clone(), 
    data
)

// After
Event::new_with_timestamp(
    original_event.timestamp,  // Use original event timestamp directly
    "http_parser".to_string(), 
    original_event.pid, 
    original_event.comm.clone(), 
    data
)
```

### Updated Constructor Calls

**sse_processor.rs:**
```rust
// Removed these parameters from SSEProcessorEvent::new():
// original_event.data.get("comm").unwrap_or(&json!("unknown")).as_str().unwrap_or("unknown").to_string(),
// original_event.data.get("pid").unwrap_or(&json!(0)).as_u64().unwrap_or(0),
```

**http_parser.rs:**
```rust
// Removed these parameters from HTTPEvent::new():
// original_event.data.get("comm").unwrap_or(&json!("unknown")).as_str().unwrap_or("unknown").to_string(),
// original_event.data.get("pid").unwrap_or(&json!(0)).as_u64().unwrap_or(0),
// original_event.data.get("timestamp_ns").unwrap_or(&json!(0)).as_u64().unwrap_or(0),
```

## Benefits

1. **Reduced Duplication**: No more redundant fields that duplicate original event data
2. **Cleaner Architecture**: Analysis events focus on their specific analyzed data only
3. **Single Source of Truth**: pid/comm/timestamp come from original event, preventing inconsistencies
4. **Simplified Constructors**: Fewer parameters to manage and maintain
5. **Better Timestamp Handling**: Direct use of original timestamps eliminates conversion complexities

## Testing
- ✅ All 48 tests pass
- ✅ Code compiles without errors  
- ✅ Removed unused imports
- ✅ Backward compatibility maintained through `to_event(original_event)` signature

## Impact
These changes make the analysis events cleaner and ensure that pid, comm, and timestamp information always comes from the authoritative source (the original event), eliminating any possibility of data drift or inconsistency.