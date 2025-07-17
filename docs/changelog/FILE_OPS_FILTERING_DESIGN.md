# File Operations Filtering Design

## Problem Statement

File operations (open, openat, close) generate excessive events, causing:

- High CPU/memory usage
- Storage bloat
- Difficult analysis due to noise
- Performance degradation

## Proposed Solutions

### 1. Kernel-Level (eBPF) Filtering

#### 1.1 Path-Based Filtering

- **Exclude common system paths** that generate noise:
  - `/proc/*` - Process information files
  - `/sys/*` - System files
  - `/dev/*` - Device files (except important ones like /dev/null)
  - `/usr/lib/*` - Library files
  - `/usr/share/*` - Shared data files
  - `*.so*` - Shared libraries
  
- **Include only specific paths** (allowlist mode):
  - User home directories
  - Application-specific directories
  - Configuration directories (/etc)
  - Log directories (/var/log)

#### 1.2 File Type Filtering

- Filter by file extensions or patterns
- Skip temporary files (`*.tmp`, `*.swp`, `*.lock`)
- Skip cache files
- Focus on source code, configs, data files

#### 1.3 Operation-Based Filtering

- Option to track only writes (more security-relevant)
- Option to skip read-only operations
- Track only file creation/deletion (not every open/close)

#### 1.4 Rate Limiting

- Per-process rate limiting (max N events per second)
- Global rate limiting
- Burst allowance with token bucket algorithm

#### 1.5 Event Deduplication and Time-Window Aggregation

**Design**: Aggregate repeated open/close of same file within sliding time window

**Key Features**:

- Report first occurrence immediately (no delay)
- For subsequent identical operations within 1-minute window:
  - Increment counter but don't report
  - Report aggregated count after window expires
- Report different file operations immediately
- Report all pending operations immediately on process exit

**Implementation**:
- Track file operations per process with timestamps
- Use sliding window approach (not fixed intervals)
- Maintain counters for each unique (pid, filename, operation) tuple
- Flush pending reports on process termination

**Benefits**:
- Reduces event volume by 80-95% for repetitive operations
- Preserves first occurrence for immediate visibility
- Maintains accurate counts for analysis
- No loss of security-relevant information

**Example Flow**:
1. Process opens `/var/log/app.log` at T=0 → Report immediately
2. Same process opens same file at T=10s → Increment counter, no report
3. Same process opens same file at T=30s → Increment counter, no report
4. At T=60s → Report aggregated count (3 operations in window)
5. Process opens `/etc/config.ini` at T=35s → Report immediately (different file)
6. Process exits at T=45s → Report any pending aggregations

### 2. Smart Sampling

#### 2.1 Probabilistic Sampling

- Sample X% of file operations
- Adaptive sampling based on load
- Higher sampling for security-relevant operations

#### 2.2 Time-Window Aggregation

- Batch events within time windows (e.g., 100ms)
- Report summary: "Process X accessed files A,B,C N times"
- Maintain counts instead of individual events

### 3. Command-Line Configuration

Add new options to process binary:

```
--file-ops-mode <mode>    # none, all, write-only, filtered
--file-paths <paths>      # Comma-separated paths to include
--exclude-paths <paths>   # Comma-separated paths to exclude
--file-rate-limit <n>     # Max file events per second per process
--file-sample-rate <n>    # Percentage of events to capture (0-100)
--file-window <n>         # Time window for aggregation in seconds (default: 60)
--file-aggregate          # Enable time-window aggregation
```

### 4. Collector-Level Enhancements

#### 4.1 File Operation Analyzer

- Aggregate file operations by process
- Detect patterns (sequential reads, file scanning)
- Generate summary reports

#### 4.2 Smart Filtering

- ML-based anomaly detection
- Baseline normal file access patterns
- Alert on unusual file operations

## Implementation Priority

1. **Phase 1 - Basic Filtering** (High Priority)
   - Path-based exclusion of system directories
   - Operation type filtering (read/write)
   - Basic rate limiting

2. **Phase 2 - Advanced Filtering** (Medium Priority)
   - Path allowlisting
   - File type filtering
   - Event deduplication

3. **Phase 3 - Intelligent Features** (Low Priority)
   - Sampling strategies
   - Collector-level analysis
   - Pattern detection

## Expected Impact

- **70-90% reduction** in event volume
- Minimal performance overhead
- More meaningful security insights
- Easier log analysis

## Technical Implementation Details

### eBPF Map Structures

```c
// Key structure for tracking unique file operations
struct file_op_key {
    u32 pid;
    u32 tgid;
    char filename[256];
    u8 op_type; // 0=open, 1=close, 2=read, 3=write
} __attribute__((packed));

// State tracking for each operation
struct file_op_state {
    u64 first_seen_ns;      // First occurrence timestamp
    u64 last_seen_ns;       // Last occurrence timestamp  
    u32 count;              // Number of occurrences
    u8 first_reported;      // Whether first was reported
} __attribute__((packed));

// BPF hash map for tracking
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __type(key, struct file_op_key);
    __type(value, struct file_op_state);
    __uint(max_entries, 10240);
} file_op_tracker SEC(".maps");
```

### Kernel-Side Logic (eBPF)

```c
// In tracepoint handler
struct file_op_state *state = bpf_map_lookup_elem(&file_op_tracker, &key);
u64 now = bpf_ktime_get_ns();

if (!state) {
    // First occurrence - report immediately
    struct file_op_state new_state = {
        .first_seen_ns = now,
        .last_seen_ns = now,
        .count = 1,
        .first_reported = 0
    };
    bpf_map_update_elem(&file_op_tracker, &key, &new_state, BPF_ANY);
    // Send event to ringbuffer
    send_immediate_event(&key, 1);
} else {
    // Update existing entry
    state->count++;
    state->last_seen_ns = now;
    bpf_map_update_elem(&file_op_tracker, &key, state, BPF_EXIST);
}
```

### User-Space Processing

```c
// Timer-based window checking (runs every second)
void check_aggregation_windows() {
    struct file_op_key key = {0}, next_key;
    struct file_op_state state;
    u64 now = get_time_ns();
    u64 window_ns = config.window_seconds * 1000000000ULL;
    
    while (bpf_map_get_next_key(map_fd, &key, &next_key) == 0) {
        if (bpf_map_lookup_elem(map_fd, &next_key, &state) == 0) {
            if (now - state.first_seen_ns > window_ns) {
                // Window expired - report aggregation
                if (state.count > 1) {
                    report_aggregated(&next_key, &state);
                }
                // Remove or reset entry
                bpf_map_delete_elem(map_fd, &next_key);
            }
        }
        key = next_key;
    }
}

// Process exit handler
void handle_process_exit(u32 pid) {
    // Iterate and report all pending operations for this PID
    // Similar to above but filtered by pid
}
```

### Output Format

```json
// First occurrence (immediate)
{
  "type": "file_op",
  "subtype": "immediate",
  "timestamp": 1234567890,
  "pid": 1234,
  "comm": "python",
  "operation": "open",
  "filename": "/var/log/app.log"
}

// Aggregated report
{
  "type": "file_op", 
  "subtype": "aggregated",
  "timestamp": 1234567950,
  "pid": 1234,
  "comm": "python",
  "operation": "open",
  "filename": "/var/log/app.log",
  "count": 150,
  "window_start": 1234567890,
  "window_duration_sec": 60
}
```

## Configuration Examples

```bash
# Track only writes to user directories
./process --file-ops-mode write-only --file-paths "/home,/tmp,/var" 

# Exclude system directories, sample 10% of remaining
./process --exclude-paths "/proc,/sys,/dev" --file-sample-rate 10

# Rate limit to 100 events/sec per process
./process --file-rate-limit 100

# Enable time-window aggregation with 30-second window
./process --file-aggregate --file-window 30

# Combine aggregation with path filtering
./process --file-aggregate --exclude-paths "/proc,/sys" --file-ops-mode all
```
