# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

AgentSight is a comprehensive observability framework designed specifically for monitoring AI agent behavior through SSL/TLS traffic interception and process monitoring. Unlike traditional application-level instrumentation, AgentSight observes at the system boundary using eBPF technology, providing tamper-resistant insights into AI agent interactions with minimal performance overhead.

## Project Structure

- **`bpf/`**: Core eBPF programs and C utilities
  - `process.bpf.c` & `process.c`: Process monitoring eBPF program with lifecycle tracking
  - `sslsniff.bpf.c` & `sslsniff.c`: SSL/TLS traffic monitoring eBPF program
  - `test_process_utils.c`: Unit tests for process utilities
  - `Makefile`: Advanced build configuration with AddressSanitizer support
- **`collector/`**: Rust-based streaming analysis framework
  - `src/framework/`: Core streaming analysis framework with pluggable analyzers
    - `analyzers/`: HTTP parsing, chunk merging, file logging, output handling
    - `runners/`: SSL, Process, and Fake data runners
    - `core/events.rs`: Standardized event system with JSON payloads
    - `binary_extractor.rs`: Embedded eBPF binary management
  - `src/main.rs`: CLI entry point with multiple operation modes
  - `DESIGN.md`: Detailed framework architecture documentation
- **`frontend/`**: Next.js web interface for visualization
  - React/TypeScript frontend with timeline visualization
  - Real-time log parsing and event display
- **`script/`**: Python analysis tools
  - SSL traffic analyzers and timeline generators
  - Data processing and visualization utilities
- **`docs/`**: Project documentation
  - Problem statement, architectural decisions, and usage guides
- **`vmlinux/`**: Kernel headers for different architectures (x86, arm64, riscv)
- **`libbpf/`**: libbpf library submodule
- **`bpftool/`**: bpftool utility submodule

## Common Development Commands

### Building the Project

```bash
# Install dependencies (Ubuntu/Debian)
make install

# Build eBPF programs
make build

# Build collector (requires Rust 1.88.0+)
cd collector && cargo build --release

# Build frontend
cd frontend && npm install && npm run build

# Run tests
cd src && make test

# Clean build artifacts
make clean
cd collector && cargo clean
```

### Development Commands

```bash
# Run individual eBPF programs
sudo bpf/process
sudo bpf/sslsniff

# Run collector with different modes
cd collector && cargo run ssl --sse-merge
cd collector && cargo run process
cd collector && cargo run trace --ssl --process --comm python --server
cd collector && cargo run record --comm claude --server-port 8080

# Run frontend development server
cd frontend && npm run dev

# Run embedded web server for frontend
cd collector && cargo run server --log-file trace.log

# Build with AddressSanitizer for debugging
cd bpf && make debug
cd bpf && make sslsniff-debug
```

### Testing

```bash
# Run C unit tests
cd bpf && make test

# Run Rust tests
cd collector && cargo test

# Run integration tests with fake data
cd collector && cargo test -- --test-threads=1

# Frontend linting and type checking
cd frontend && npm run lint
cd frontend && npm run build  # Also runs type checking
```

## Architecture Overview

### Core Components

1. **eBPF Data Collection Layer**
   - `process.bpf.c`: Monitors system processes, executions, and file operations
   - `sslsniff.bpf.c`: Captures SSL/TLS traffic data with <3% performance overhead
   - Both programs output structured JSON events to stdout

2. **Rust Streaming Framework** (`collector/src/framework/`)
   - **Runners**: Execute eBPF binaries and stream events (SSL, Process, Fake, Agent, Combined)
   - **Analyzers**: Process and transform event streams with pluggable architecture
   - **Core Events**: Standardized event format with rich metadata and JSON payloads
   - **Binary Extractor**: Manages embedded eBPF binaries with automatic cleanup

3. **Frontend Visualization** (`frontend/`)
   - Next.js/React application for real-time event visualization
   - Timeline view with log parsing and semantic event processing
   - TypeScript implementation with Tailwind CSS styling
   - Embedded web server integration via `/api/events` endpoint

4. **Embedded Web Server** (`collector/src/server/`)
   - Hyper-based HTTP server with embedded frontend assets
   - `/api/events` endpoint for log file serving
   - `/api/assets` endpoint for asset enumeration
   - Real-time event broadcasting with tokio broadcast channels
   - Static asset serving with proper MIME types and caching

5. **Analysis Tools** (`script/`)
   - Python utilities for SSL traffic analysis and timeline generation
   - Data processing pipelines for correlation analysis

### Streaming Pipeline Architecture

```
eBPF Binary → JSON Output → Runner → Analyzer Chain → Frontend/Storage/Output
```

### Key Framework Components

- **`framework/core/events.rs`**: Core event system with standardized `Event` structure
- **`framework/runners/`**: Data collection implementations with fluent builders
- **`framework/analyzers/`**: Stream processing plugins (ChunkMerger, FileLogger, Output, HTTPFilter, SSLFilter, AuthHeaderRemover)
- **`framework/binary_extractor.rs`**: Manages embedded eBPF binaries with security

### Event Flow

1. **Data Collection**: eBPF programs collect kernel events (SSL/TLS, process lifecycle)
2. **JSON Streaming**: Events converted to JSON with timestamps and metadata
3. **Runner Processing**: Rust runners parse JSON and create typed event streams
4. **Analyzer Chain**: Multiple analyzers process events in configurable sequences
5. **Output**: Processed events sent to console, files, frontend, or external systems

## Development Patterns

### Adding New eBPF Programs

1. Create `.bpf.c` file with eBPF kernel code using CO-RE (Compile Once - Run Everywhere)
2. Create `.c` file with userspace loader and JSON output formatting
3. Add to `APPS` variable in `bpf/Makefile`
4. Include appropriate vmlinux.h for target architecture
5. Use libbpf for userspace interaction and event handling
6. Add unit tests following `test_process_utils.c` pattern

### Adding New Analyzers

1. Implement the `Analyzer` trait in `collector/src/framework/analyzers/`
2. Add async processing logic for event streams using tokio
3. Export in `analyzers/mod.rs`
4. Use in runner chains via fluent builder pattern `add_analyzer()`
5. Follow existing patterns for error handling and stream processing

### Adding New Runners

1. Implement the `Runner` trait in `collector/src/framework/runners/`
2. Use fluent builder pattern for configuration
3. Support embedded binary extraction via `BinaryExtractor`
4. Add comprehensive error handling and logging
5. Export in `runners/mod.rs`

### Configuration Management

- eBPF programs use command-line arguments for runtime configuration
- Collector framework uses fluent builder pattern for type-safe configuration
- Binary extraction handled automatically via `BinaryExtractor` with temp file cleanup
- Frontend configuration through environment variables and build-time settings

### Advanced Filtering and Analysis

The framework includes sophisticated filtering capabilities for both SSL and HTTP traffic:

- **SSL Filter**: Expression-based filtering with field-specific patterns (data, function, latency, etc.)
- **HTTP Filter**: Request/response filtering by method, path, status code, headers
- **Authorization Header Removal**: Automatically removes sensitive headers from HTTP events
- **Filter Expressions**: Support for AND/OR logic with escape sequences
- **Global Metrics**: Atomic counters for filter performance tracking
- **Log Rotation**: Built-in log rotation with configurable size limits

## Key Design Principles

1. **Streaming Architecture**: Real-time event processing with minimal memory usage and async/await
2. **Plugin System**: Extensible analyzer chains for flexible data processing pipelines
3. **Error Resilience**: Graceful handling of malformed data, process failures, and analyzer errors
4. **Resource Management**: Automatic cleanup of temporary files, processes, and kernel resources
5. **Type Safety**: Rust type system ensures memory safety and prevents common vulnerabilities
6. **Zero-Instrumentation**: System-level monitoring without modifying target applications
7. **Integrated Web Interface**: Embedded web server for real-time visualization and log serving

## Development Workflow

### Typical Development Cycle

1. **Build eBPF programs**: `make build` (run from root directory)
2. **Build collector**: `cd collector && cargo build`
3. **Test changes**: `cd bpf && make test && cd ../collector && cargo test`
4. **Run specific components**: Use individual commands for testing (see Development Commands)
5. **Frontend integration**: Use embedded server or Next.js dev server for UI testing

### CLI Command Structure

The collector uses a subcommand-based CLI:
- `ssl`: Monitor SSL/TLS traffic with configurable filtering and analysis
- `process`: Monitor process lifecycle events and file operations  
- `trace`: Combined SSL and Process monitoring with configurable options (most flexible)
- `record`: Optimized agent activity recording with predefined filters for common use cases

Note: The standalone `server` command mentioned earlier has been integrated into each monitoring command via the `--server` flag.

All commands support integrated web server via `--server` flag and log file serving via `--log-file` parameter. The `trace` command provides the most comprehensive monitoring capabilities with granular control over both SSL and process monitoring.

## Testing Strategy

- **Unit Tests**: C tests for utility functions (`bpf/test_process_utils.c`)
- **Integration Tests**: Rust tests with `FakeRunner` for full pipeline testing
- **Manual Testing**: Direct execution of eBPF programs for validation
- **Frontend Testing**: React component and TypeScript type checking
- **Performance Testing**: eBPF overhead measurement and memory usage analysis

## Security Considerations

- eBPF programs require root privileges for kernel access (CAP_BPF, CAP_SYS_ADMIN)
- SSL traffic captured includes potentially sensitive data - handle responsibly
- Temporary binary extraction requires secure cleanup and proper permissions
- Process monitoring may expose system information - use appropriate filtering
- Frontend serves processed data - sanitize outputs and validate inputs
- Tamper-resistant monitoring design prevents agent manipulation

## Dependencies

### Core Dependencies
- **C/eBPF**: libbpf (v1.0+), libelf, clang (v10+), llvm
- **Rust**: tokio (async runtime), serde (JSON), clap (CLI), async-trait, chrono (requires Rust edition 2024)
- **Frontend**: Next.js 15.3+, React 18+, TypeScript 5+, Tailwind CSS
- **System**: Linux kernel 4.1+ with eBPF support

### Development Dependencies
- **Rust**: cargo edition 2024, env_logger, tempfile, uuid, hex, chunked_transfer
- **Frontend**: ESLint, PostCSS, Autoprefixer, TypeScript compiler
- **Python**: Analysis scripts for data processing (optional)
- **Web Server**: hyper, hyper-util, rust-embed for embedded frontend serving

## Common Issues and Solutions

- **Permission Errors**: eBPF programs require sudo privileges - use `sudo` or appropriate capabilities
- **Kernel Compatibility**: Use architecture-specific vmlinux.h from `vmlinux/` directory
- **Binary Extraction**: Ensure `/tmp` permissions allow execution, check `BinaryExtractor` cleanup
- **UTF-8 Handling**: HTTP parser includes safety fixes for malformed data
- **Frontend Build**: Ensure Node.js version compatibility and clean `node_modules` if needed
- **Cargo Edition**: Project uses Rust edition 2024 - ensure compatible toolchain
- **eBPF Program Loading**: If programs fail to load, check for missing BTF support or use fallback vmlinux.h
- **Port Conflicts**: Default web server runs on 8080, frontend dev server on 3000

## Usage Examples

### Basic SSL Traffic Monitoring
```bash
sudo ./bpf/sslsniff -p 1234
cd collector && cargo run ssl --sse-merge -- -p 1234
```

### Process Lifecycle Tracking
```bash
sudo ./bpf/process -c python
cd collector && cargo run process -- -c python
```

### Combined Monitoring with Trace Command
```bash
# Basic trace monitoring
cd collector && cargo run trace --ssl --process --comm python --server

# Advanced trace with filtering
cd collector && cargo run trace --ssl-filter "data.type=binary" --http-filter "request.method=POST" --server --log-file trace.log

# Optimized agent recording
cd collector && cargo run record --comm claude --server-port 8080
```

### Frontend Visualization
```bash
# Using Next.js development server
cd frontend && npm run dev
# Open http://localhost:3000/timeline

# Using embedded web server (integrated into monitoring commands)
cd collector && cargo run trace --server --log-file trace.log
# Open http://localhost:8080/timeline

# Server integrated with monitoring
cd collector && cargo run trace --server --log-file output.log
# Open http://localhost:8080/timeline
```