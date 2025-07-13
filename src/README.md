# eBPF Monitoring Tools

This directory contains two powerful eBPF-based monitoring tools for system observability and security analysis.

## Tools Overview

### 1. Process Tracer (`process`)

An advanced eBPF-based process monitoring tool that traces process lifecycles and file operations.

**Key Features:**
- Monitor process creation and termination
- Track file read/write operations 
- Configurable filtering modes for different monitoring levels
- JSON output format for integration with analysis frameworks

**Usage:**
```bash
sudo ./process [OPTIONS]

# Examples:
sudo ./process -m 0                   # Trace everything
sudo ./process -m 1                   # Trace all processes, selective read/write
sudo ./process -c "claude,python"     # Trace only specific processes
sudo ./process -c "ssh" -d 1000       # Trace processes lasting > 1 second
```

**Filter Modes:**
- `0 (all)`: Trace all processes and all read/write operations
- `1 (proc)`: Trace all processes but only read/write for tracked PIDs
- `2 (filter)`: Only trace processes matching filters and their read/write (default)

### 2. SSL Traffic Monitor (`sslsniff`) 

An eBPF-based SSL/TLS traffic interceptor that captures encrypted communications for security analysis.

**Key Features:**
- Intercept SSL/TLS traffic in real-time
- Support for multiple SSL libraries (OpenSSL, GnuTLS, etc.)
- Process-specific filtering capabilities
- Plaintext extraction from encrypted streams

**Usage:**
```bash
sudo ./sslsniff [OPTIONS]

# Examples:
sudo ./sslsniff                       # Monitor all SSL traffic
sudo ./sslsniff -p <PID>             # Monitor specific process
sudo ./sslsniff --extra              # Extended output format
```

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

Both tools output JSON-formatted events to stdout, making them suitable for:
- Log aggregation systems
- Real-time analysis pipelines  
- Integration with the Rust collector framework
- Security information and event management (SIEM) systems

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