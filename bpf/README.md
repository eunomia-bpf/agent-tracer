# eBPF Monitoring Tools

This directory contains two powerful eBPF-based monitoring tools for system observability and security analysis.

## Tools Overview

### 1. Process Tracer (`process`)

An advanced eBPF-based process monitoring tool that traces process lifecycles and file open operations with intelligent event deduplication.

**Key Features:**
- Monitor process creation and termination
- Track file open operations with deduplication
- Configurable filtering modes for different monitoring levels
- 60-second sliding window aggregation for repetitive file opens
- JSON output format for integration with analysis frameworks
- Verbose debugging mode for troubleshooting

**Usage:**
```bash
sudo ./process [OPTIONS]
```

**Command Line Arguments:**

| Argument | Short | Description | Default |
|----------|-------|-------------|---------|
| `--help` | `-?` | Show help message and exit | - |
| `--usage` | - | Show brief usage message | - |
| `--version` | `-V` | Show version information | - |
| `--verbose` | `-v` | Enable verbose debug output to stderr | disabled |
| `--duration=MS` | `-d MS` | Minimum process duration (ms) to report | 0 |
| `--commands=LIST` | `-c LIST` | Comma-separated list of commands to trace | all |
| `--pid=PID` | `-p PID` | Trace only this specific PID | all |
| `--mode=MODE` | `-m MODE` | Filter mode (0=all, 1=proc, 2=filter) | 2 |
| `--all` | `-a` | Deprecated: use `-m 0` instead | - |

**Filter Modes:**
- `0 (all)`: Trace all processes and all file open operations
- `1 (proc)`: Trace all processes but only file opens for tracked PIDs  
- `2 (filter)`: Only trace processes matching filters and their file opens (default)

**Examples:**
```bash
# Trace everything with verbose output
sudo ./process -v -m 0

# Trace all processes, selective read/write
sudo ./process -m 1

# Trace only specific processes
sudo ./process -c "claude,python,node"

# Trace processes lasting more than 1 second
sudo ./process -c "ssh" -d 1000

# Trace only PID 1234 with verbose debugging
sudo ./process -v -p 1234

# Trace multiple commands with minimum duration
sudo ./process -c "curl,wget" -d 500
```

**File Open Deduplication:**
- First occurrence of file opens reported immediately (`count=1`)
- Subsequent identical file opens within 60-second window are aggregated
- Aggregated results reported when window expires (`count=N`)
- All pending aggregations flushed on process exit
- Reduces event volume by 80-95% for repetitive file opens

**Verbose Debug Output (`-v`):**
- Shows when events are deduplicated/aggregated
- Reports aggregation window expirations
- Displays process exit aggregation flushes
- Shows aggregation table statistics
- Helps troubleshoot deduplication behavior

### 2. SSL Traffic Monitor (`sslsniff`) 

An eBPF-based SSL/TLS traffic interceptor that captures encrypted communications for security analysis.

**Key Features:**
- Intercept SSL/TLS traffic in real-time
- Support for multiple SSL libraries (OpenSSL, GnuTLS, NSS)
- Process-specific filtering capabilities (PID, UID, command)
- Plaintext extraction from encrypted streams
- Handshake event monitoring
- JSON output format for analysis

**Usage:**
```bash
sudo ./sslsniff [OPTIONS]
```

**Command Line Arguments:**

| Argument | Short | Description | Default |
|----------|-------|-------------|---------|
| `--help` | `-?` | Show help message and exit | - |
| `--usage` | - | Show brief usage message | - |
| `--version` | `-V` | Show version information | - |
| `--verbose` | `-v` | Enable verbose debug output to stderr | disabled |
| `--pid=PID` | `-p PID` | Trace only this specific PID | all |
| `--uid=UID` | `-u UID` | Trace only this specific UID | all |
| `--comm=COMMAND` | `-c COMMAND` | Trace only commands matching string | all |
| `--no-openssl` | `-o` | Disable OpenSSL traffic capture | enabled |
| `--no-gnutls` | `-g` | Disable GnuTLS traffic capture | disabled |
| `--no-nss` | `-n` | Disable NSS traffic capture | disabled |
| `--handshake` | `-h` | Show SSL handshake events | disabled |

**SSL Library Support:**
- **OpenSSL**: Enabled by default (most common)
- **GnuTLS**: Disabled by default, enable with `--gnutls` or disable OpenSSL with `--no-openssl`
- **NSS**: Disabled by default, enable with `--nss`

**Examples:**
```bash
# Monitor all SSL traffic with verbose output
sudo ./sslsniff -v

# Monitor specific process by PID
sudo ./sslsniff -p 1234

# Monitor specific user's SSL traffic
sudo ./sslsniff -u 1000

# Monitor only curl SSL traffic
sudo ./sslsniff -c curl

# Monitor with handshake events
sudo ./sslsniff -h

# Monitor only GnuTLS traffic (disable OpenSSL)
sudo ./sslsniff --no-openssl -g

# Monitor specific process with handshakes and verbose output
sudo ./sslsniff -v -h -p 1234

# Monitor multiple criteria
sudo ./sslsniff -c "python" -u 1000 -h
```

**Output Format:**
- Each SSL event is output as a JSON object
- eBPF capture is limited to 32KB per event due to kernel constraints
- Events include timestamps, process info, and SSL data
- Handshake events show SSL negotiation details

**Filtering Options:**
- **PID filtering**: Only capture traffic from specific process
- **UID filtering**: Only capture traffic from specific user
- **Command filtering**: Only capture traffic from matching command names
- **Library filtering**: Choose which SSL libraries to monitor

## Building the Tools

### Prerequisites
```bash
# Install dependencies (Ubuntu/Debian)
make install

# Or manually:
sudo apt-get install -y libelf1 libelf-dev zlib1g-dev make clang llvm
```

### Build Commands
```bash
# Build both tools
make build

# Build individual tools
make process
make sslsniff

# Build with debugging symbols
make debug
make sslsniff-debug

# Run tests
make test

# Clean build artifacts
make clean
```

## Architecture

Both tools utilize the same architectural pattern:

1. **eBPF Kernel Programs** (`.bpf.c` files)
   - Kernel-space code that hooks into system events
   - Collects data with minimal performance overhead
   - Outputs structured event data

2. **Userspace Loaders** (`.c` files)
   - Load and manage eBPF programs
   - Process kernel events and format output
   - Handle command-line arguments and configuration

3. **Header Files** (`.h` files)
   - Shared data structures between kernel and userspace
   - Event definitions and configuration constants

## Output Format

Both tools output JSON-formatted events to stdout, with debug information sent to stderr when verbose mode is enabled.

### JSON Schema

All events follow a common base schema with event-specific fields:

**Base Event Fields:**
- `timestamp`: Unix timestamp in nanoseconds (uint64)
- `event`: Event type string (EXEC, EXIT, FILE_OPEN, BASH_READLINE, SSL_READ, SSL_WRITE, SSL_HANDSHAKE)
- `comm`: Process command name (string, max 16 chars)
- `pid`: Process ID (int32)

**Process Event Fields:**
- `ppid`: Parent process ID (int32)
- `filename`: Executable path (string, EXEC events only)
- `exit_code`: Process exit code (uint32, EXIT events only)
- `duration_ms`: Process lifetime in milliseconds (uint64, EXIT events only)

**File Open Event Fields:**
- `count`: Number of aggregated file opens (uint32)
- `filepath`: Full path to the file being opened (string, max 256 chars)
- `flags`: File open flags (int32)
- `window_expired`: Present when aggregation window expires (boolean, optional)
- `reason`: Why aggregation was flushed (string, optional: "process_exit")

**Bash Readline Event Fields:**
- `command`: Command line entered (string, max 256 chars)

**SSL Event Fields:**
- `uid`: User ID of the process (uint32)
- `data`: SSL traffic data (string, max 32KB)
- `data_len`: Length of data captured (uint32)
- `truncated`: Whether data was truncated due to size limits (boolean)

### Process Tracer JSON Events

**Process Events:**
```json
{
  "timestamp": 1234567890123456789,
  "event": "EXEC",
  "comm": "python3",
  "pid": 1234,
  "ppid": 1000,
  "filename": "/usr/bin/python3"
}

{
  "timestamp": 1234567890123456789,
  "event": "EXIT", 
  "comm": "python3",
  "pid": 1234,
  "ppid": 1000,
  "exit_code": 0,
  "duration_ms": 5000
}
```

**File Open Events:**
```json
{
  "timestamp": 1234567890123456789,
  "event": "FILE_OPEN",
  "comm": "python3",
  "pid": 1234,
  "count": 1,
  "filepath": "/etc/passwd",
  "flags": 0
}

{
  "timestamp": 1234567890123456789,
  "event": "FILE_OPEN",
  "comm": "python3",
  "pid": 1234,
  "count": 5,
  "filepath": "/etc/passwd",
  "flags": 0,
  "window_expired": true
}

{
  "timestamp": 1234567890123456789,
  "event": "FILE_OPEN",
  "comm": "python3",
  "pid": 1234,
  "count": 3,
  "filepath": "/tmp/tempfile.txt",
  "flags": 577,
  "reason": "process_exit"
}
```

**Bash Readline Events:**
```json
{
  "timestamp": 1234567890123456789,
  "event": "BASH_READLINE",
  "comm": "bash",
  "pid": 1234,
  "command": "ls -la"
}
```

### SSL Traffic Monitor JSON Events

**SSL Traffic Events:**
```json
{
  "timestamp": 1234567890123456789,
  "event": "SSL_READ",
  "comm": "curl",
  "pid": 1234,
  "uid": 1000,
  "data": "GET / HTTP/1.1\r\nHost: example.com\r\n",
  "data_len": 32,
  "truncated": false
}

{
  "timestamp": 1234567890123456789,
  "event": "SSL_WRITE", 
  "comm": "curl",
  "pid": 1234,
  "uid": 1000,
  "data": "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n",
  "data_len": 45,
  "truncated": false
}
```

**SSL Handshake Events:**
```json
{
  "timestamp": 1234567890123456789,
  "event": "SSL_HANDSHAKE",
  "comm": "curl",
  "pid": 1234,
  "uid": 1000,
  "data": null,
  "truncated": false
}
```

### Common Usage Patterns

**Real-time Monitoring:**
```bash
# Monitor and format JSON output
sudo ./process -v -c "python" | jq '.'

# Filter specific event types
sudo ./process -c "curl" | jq 'select(.event == "FILE_OPEN")'

# Monitor SSL traffic with pretty printing
sudo ./sslsniff -p 1234 | jq '.'
```

**Log Collection:**
```bash
# Save to file with timestamps
sudo ./process -m 1 > process_events.jsonl 2> process_debug.log

# Pipe to syslog
sudo ./sslsniff -c "nginx" | logger -t sslsniff

# Send to log aggregation system
sudo ./process -c "app" | ./log_shipper --index=security
```

**Integration Examples:**
```bash
# Combine with analysis tools
sudo ./process -p 1234 | python3 analyze_events.py

# Real-time alerting
sudo ./sslsniff -u 1000 | grep -i "password" | alert_system.sh

# Statistical analysis
sudo ./process -c "python" | jq -r '.event' | sort | uniq -c
```

The JSON output is designed for:
- Log aggregation systems (ELK, Splunk, etc.)
- Real-time analysis pipelines
- Integration with the Rust collector framework
- Security information and event management (SIEM) systems
- Custom analysis and monitoring scripts

## Security Considerations

⚠️ **Important Security Notes:**
- Both tools require root privileges for eBPF program loading
- SSL traffic capture includes potentially sensitive data
- Process monitoring may expose system information
- Intended for defensive security and monitoring purposes only

## Integration

These tools are designed to work with the `collector` framework:
- Built binaries are embedded into the Rust collector at compile time
- Collector provides streaming analysis and event processing
- Output can be processed by multiple analyzer plugins

## Troubleshooting

**Permission Issues:**
```bash
# Ensure proper permissions
sudo ./process
sudo ./sslsniff
```

**Kernel Compatibility:**
- Requires Linux kernel with eBPF support (4.1+)
- CO-RE (Compile Once, Run Everywhere) support recommended
- Check kernel config: `CONFIG_BPF=y`, `CONFIG_BPF_SYSCALL=y`

**Debug Mode:**
```bash
# Build with AddressSanitizer for debugging
make debug
sudo ./process-debug
```

## Related Documentation

- See `/collector/README.md` for Rust framework integration
- See `/CLAUDE.md` for development guidelines
- See main project README for overall architecture