# Timestamp Preservation Fix

## Problem
SSL operations and Claude API calls appeared at different times in logs despite happening simultaneously due to timestamp modification in analyzers.

## Root Cause
Two analyzers were incorrectly creating new timestamps instead of preserving original event timing:

1. **SSE Processor** (`event.rs:89`): Used `Event::new()` which creates current timestamp
2. **HTTP Parser** (`event.rs:199`): Used `Event::new()` which creates current timestamp

## Solution
Refactored `to_event` methods to:

1. **Accept original event as parameter**: `to_event(&self, original_event: &Event)`
2. **Use original pid/comm**: No need to maintain duplicate fields when original event has them
3. **Preserve timestamps intelligently**:
   - **SSE Processor**: Use `end_time` if events were merged (event_count > 1), otherwise use original timestamp
   - **HTTP Parser**: Always use original timestamp from SSL event

## Changes Made

### `event.rs` - SSEProcessorEvent::to_event()
```rust
// Before
pub fn to_event(&self) -> Event {
    Event::new("sse_processor".to_string(), self.pid as u32, self.comm.clone(), data)
}

// After  
pub fn to_event(&self, original_event: &Event) -> Event {
    let timestamp = if self.event_count > 1 {
        self.end_time  // Use merged end_time for multi-event merging
    } else {
        original_event.timestamp  // Preserve original for single events
    };
    
    Event::new_with_timestamp(
        timestamp,
        "sse_processor".to_string(), 
        original_event.pid,        // Use original pid
        original_event.comm.clone(), // Use original comm
        data
    )
}
```

### `event.rs` - HTTPEvent::to_event()
```rust
// Before
pub fn to_event(&self) -> Event {
    Event::new("http_parser".to_string(), self.pid as u32, self.comm.clone(), data)
}

// After
pub fn to_event(&self, original_event: &Event) -> Event {
    Event::new_with_timestamp(
        self.timestamp_ns / 1_000_000,  // Convert ns to ms, preserve original time
        "http_parser".to_string(), 
        original_event.pid,        // Use original pid
        original_event.comm.clone(), // Use original comm  
        data
    )
}
```

### Updated method calls
- `http_parser.rs:216`: `http_event.to_event(original_event)`
- `sse_processor.rs:424`: `sse_processor_event.to_event(original_event)`

## Benefits

1. **Accurate Timeline**: SSL ops and API calls now show correct chronological order
2. **Timestamp Integrity**: Original eBPF timestamps preserved throughout pipeline
3. **Reduced Duplication**: Analysis events use original pid/comm instead of duplicating
4. **Smart Merging**: SSE processor correctly uses last timestamp when merging multiple events

## Testing
- All 48 tests pass
- Cargo check passes without errors
- Backward compatibility maintained

## Verification
Use the new `verify_timestamp_fix.py` script to validate timestamp consistency:
```bash
python3 script/verify_timestamp_fix.py /path/to/ssl.log
```