# Log Rotation & Limits Design for FileLogger

## 1. Configuration Structure

```rust
#[derive(Debug, Clone)]
pub struct LogRotationConfig {
    /// Maximum size of a single log file in bytes
    max_file_size: u64,
    
    /// Maximum number of rotated log files to keep (excluding current)
    max_files: usize,
    
    /// Check file size every N events (performance optimization)
    size_check_interval: u64,
    
    /// Whether to compress rotated files (optional feature)
    compress_rotated: bool,
}

impl Default for LogRotationConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10_000_000, // 10MB
            max_files: 5,
            size_check_interval: 100,
            compress_rotated: false,
        }
    }
}
```

## 2. Enhanced FileLogger Structure

```rust
pub struct FileLogger {
    name: String,
    file_path: String,
    file_handle: Arc<Mutex<std::fs::File>>,
    
    // New fields for rotation
    rotation_config: Option<LogRotationConfig>,
    event_count: Arc<Mutex<u64>>,  // For size check interval
    last_rotation_check: Arc<Mutex<std::time::Instant>>,
}
```

## 3. API Design

### Constructor Methods
```rust
impl FileLogger {
    /// Create FileLogger without rotation (existing behavior)
    pub fn new<P: AsRef<Path>>(file_path: P) -> Result<Self, std::io::Error>
    
    /// Create FileLogger with rotation configuration
    pub fn with_rotation<P: AsRef<Path>>(
        file_path: P, 
        config: LogRotationConfig
    ) -> Result<Self, std::io::Error>
    
    /// Convenience method for simple size-based rotation
    pub fn with_max_size<P: AsRef<Path>>(
        file_path: P, 
        max_size_mb: u64
    ) -> Result<Self, std::io::Error> {
        let config = LogRotationConfig {
            max_file_size: max_size_mb * 1_000_000,
            ..Default::default()
        };
        Self::with_rotation(file_path, config)
    }
}
```

## 4. Rotation Logic Flow

### Size Check Strategy
```rust
fn should_rotate(&self) -> Result<bool, std::io::Error> {
    // Only check if rotation is enabled
    let config = match &self.rotation_config {
        Some(config) => config,
        None => return Ok(false),
    };
    
    // Check interval optimization
    let mut event_count = self.event_count.lock().unwrap();
    *event_count += 1;
    
    if *event_count % config.size_check_interval != 0 {
        return Ok(false);
    }
    
    // Check actual file size
    let metadata = std::fs::metadata(&self.file_path)?;
    Ok(metadata.len() > config.max_file_size)
}
```

### Rotation Process
```rust
fn rotate_logs(&self) -> Result<(), std::io::Error> {
    let config = self.rotation_config.as_ref().unwrap();
    
    // 1. Close current file handle
    // 2. Rename files in reverse order (app.log.2 -> app.log.3)
    // 3. Rename current file (app.log -> app.log.1)
    // 4. Create new current file
    // 5. Update file handle
    // 6. Cleanup old files beyond max_files limit
}
```

## 5. File Naming Convention

```
app.log           # Current active log file
app.log.1         # Most recent rotated file
app.log.2         # Second most recent
app.log.3         # Third most recent
...
app.log.N         # Oldest kept file (N = max_files)
```

## 6. Error Handling Strategy

### Graceful Degradation
- If rotation fails, continue logging to current file
- Log rotation errors to stderr, not to the log file itself
- Never lose incoming log events due to rotation issues

### Error Types
```rust
#[derive(Debug)]
enum RotationError {
    FileSizeCheck(std::io::Error),
    FileRename(std::io::Error),
    FileCreate(std::io::Error),
    Cleanup(std::io::Error),
}
```

## 7. Thread Safety Considerations

### Atomic Rotation
- All rotation operations happen within the existing `Arc<Mutex<File>>` lock
- No concurrent writes during rotation
- Rotation state is consistent across threads

### Lock Ordering
```rust
// Always acquire locks in this order to prevent deadlocks:
// 1. file_handle
// 2. event_count  
// 3. last_rotation_check
```

## 8. Performance Optimizations

### Reduced Size Checking
- Only check file size every `size_check_interval` events
- Cache file metadata when possible
- Use async I/O for non-blocking operations

### Buffering Strategy
```rust
// Optional: Buffer writes to reduce I/O overhead
struct BufferedFileLogger {
    buffer: Vec<String>,
    buffer_size: usize,
    last_flush: std::time::Instant,
    flush_interval: std::time::Duration,
}
```

## 9. Configuration Examples

### Basic Size-Based Rotation
```rust
let logger = FileLogger::with_max_size("app.log", 10)?; // 10MB max
```

### Advanced Configuration
```rust
let config = LogRotationConfig {
    max_file_size: 50_000_000,  // 50MB
    max_files: 10,              // Keep 10 old files
    size_check_interval: 500,   // Check every 500 events
    compress_rotated: true,     // Compress old files
};
let logger = FileLogger::with_rotation("app.log", config)?;
```

## 10. Testing Strategy

### Unit Tests
```rust
#[tokio::test]
async fn test_rotation_on_size_limit()
#[tokio::test] 
async fn test_max_files_cleanup()
#[tokio::test]
async fn test_rotation_failure_graceful_degradation()
#[tokio::test]
async fn test_concurrent_rotation()
```

### Integration Tests
- Test with high-volume event streams
- Verify file integrity after rotation
- Test disk space exhaustion scenarios

## 11. Backward Compatibility

### Existing API Unchanged
- `FileLogger::new()` continues to work without rotation
- `FileLogger::new_with_options()` remains deprecated but functional
- All existing tests pass without modification

### Migration Path
```rust
// Old code continues to work
let logger = FileLogger::new("app.log")?;

// New code can opt into rotation
let logger = FileLogger::with_max_size("app.log", 10)?;
```

## 12. Future Enhancements

### Time-Based Rotation
```rust
pub enum RotationTrigger {
    Size(u64),
    Time(std::time::Duration),
    Both { size: u64, time: std::time::Duration },
}
```

### Compression Support
```rust
// Optional dependency on flate2 or similar
fn compress_rotated_file(path: &Path) -> Result<(), std::io::Error>
```

### Remote Log Shipping
```rust
// Optional: Ship rotated logs to remote storage
trait LogShipper {
    fn ship_log(&self, file_path: &Path) -> Result<(), Box<dyn std::error::Error>>;
}
```

## Implementation Summary

This design maintains simplicity while adding powerful log management capabilities. The implementation would be approximately 150-200 lines of additional code with comprehensive error handling and testing.

### Key Benefits:
- **Backward Compatible**: Existing code continues to work unchanged
- **Performance Optimized**: Minimal overhead with configurable size check intervals
- **Thread Safe**: Uses existing mutex-based approach
- **Graceful Degradation**: Never loses log events due to rotation failures
- **Flexible Configuration**: Simple size-based rotation with advanced options available

### Implementation Priority:
1. Add `LogRotationConfig` struct and new constructors
2. Implement size checking logic with interval optimization
3. Add rotation mechanism with proper file handling
4. Implement cleanup of old log files
5. Add comprehensive test coverage
6. Update documentation and examples