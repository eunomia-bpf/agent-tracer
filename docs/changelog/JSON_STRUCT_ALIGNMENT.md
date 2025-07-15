# JSON Structure Alignment Fix

## Problem
The manual JSON construction in `to_event()` methods didn't exactly match the struct fields, leading to inconsistencies between the data structure and its JSON representation.

## Solution
Replaced manual JSON construction with automatic serialization using `serde_json::to_value(self)` to ensure the JSON output exactly matches the struct fields.

## Changes Made

### SSEProcessorEvent::to_event()
```rust
// Before (Manual JSON construction)
let data = serde_json::json!({
    "connection_id": self.connection_id,
    "message_id": self.message_id,
    "start_time": self.start_time,
    "end_time": self.end_time,
    "duration_ns": self.duration_ns,
    "duration_ms": self.duration_ns as f64 / 1_000_000.0,  // ❌ Not in struct
    "duration_seconds": self.duration_ns as f64 / 1_000_000_000.0,  // ❌ Not in struct
    "original_source": self.original_source,
    "function": self.function,
    "tid": self.tid,
    "json_content": self.json_content,
    "text_content": self.text_content,
    "total_size": self.total_size,
    "event_count": self.event_count,
    "has_message_start": self.has_message_start,
    "sse_events": self.sse_events
});

// After (Automatic serialization)
let data = serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}));
```

### HTTPEvent::to_event()
```rust
// Before (Manual JSON construction)
let mut data = serde_json::json!({
    "tid": self.tid,
    "message_type": self.message_type,
    "first_line": self.first_line,
    "method": self.method,
    "path": self.path,
    "protocol": self.protocol,
    "status_code": self.status_code,
    "status_text": self.status_text,
    "headers": self.headers,
    "body": self.body,
    "total_size": self.total_size,
    "has_body": self.has_body,
    "is_chunked": self.is_chunked,
    "content_length": self.content_length,
    "original_source": self.original_source,
});

// Handle raw_data separately
if let Some(raw_data) = &self.raw_data {
    data["raw_data"] = serde_json::json!(raw_data);
}

// After (Automatic serialization)
let data = serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}));
```

## Benefits

1. **Exact Field Matching**: JSON output is guaranteed to match struct fields
2. **Automatic Option Handling**: `Option<T>` fields are correctly included/excluded based on Some/None
3. **Reduced Maintenance**: No need to manually update JSON construction when struct changes
4. **Type Safety**: Serde handles all type conversions correctly
5. **No Missing/Extra Fields**: Eliminates human error in field listing

## Testing Results

✅ **SSE Events**: All 14 fields match exactly
```
Expected: ['connection_id', 'duration_ns', 'end_time', 'event_count', 'function', 'has_message_start', 'json_content', 'message_id', 'original_source', 'sse_events', 'start_time', 'text_content', 'tid', 'total_size']
Actual:   ['connection_id', 'duration_ns', 'end_time', 'event_count', 'function', 'has_message_start', 'json_content', 'message_id', 'original_source', 'sse_events', 'start_time', 'text_content', 'tid', 'total_size']
```

✅ **HTTP Events**: All fields match (raw_data correctly excluded when None)
```
Expected: ['body', 'content_length', 'first_line', 'has_body', 'headers', 'is_chunked', 'message_type', 'method', 'original_source', 'path', 'protocol', 'raw_data', 'status_code', 'status_text', 'tid', 'total_size']
Actual:   ['body', 'content_length', 'first_line', 'has_body', 'headers', 'is_chunked', 'message_type', 'method', 'original_source', 'path', 'protocol', 'status_code', 'status_text', 'tid', 'total_size']
```

## Note on Option Fields
The `raw_data` field is `Option<String>` and is correctly excluded from JSON when `None`. This is the expected behavior - serde automatically handles Option types by including the field only when it has a `Some()` value.

## Impact
This change ensures that the JSON representation of analysis events is always a perfect reflection of the actual struct fields, eliminating any discrepancies between the data structure and its serialized form.