# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Agent-tracer is a comprehensive observability framework for monitoring AI agent behavior through SSL/TLS traffic and process monitoring. The project combines eBPF-based data collection with a Rust-based streaming analysis framework.

## Project Structure

- **`src/`**: Core eBPF programs and C utilities
  - `process.bpf.c` & `process.c`: Process monitoring eBPF program
  - `sslsniff.bpf.c` & `sslsniff.c`: SSL/TLS traffic monitoring eBPF program
  - `Makefile`: Build configuration for eBPF programs
- **`collector/`**: Rust-based analysis framework
  - `src/framework/`: Core streaming analysis framework
  - `src/main.rs`: CLI entry point
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

# Run tests
cd src && make test

# Clean build artifacts
make clean
cd collector && cargo clean
```

### Development Commands

```bash
# Run individual eBPF programs
sudo src/process
sudo src/sslsniff

# Run collector with embedded binaries
cd collector && cargo run

# Run collector in development mode
cd collector && cargo run -- --help

# Build with AddressSanitizer for debugging
cd src && make debug
cd src && make sslsniff-debug
```

### Testing

```bash
# Run C unit tests
cd src && make test

# Run Rust tests
cd collector && cargo test

# Run integration tests with fake data
cd collector && cargo test -- --test-threads=1
```

## Architecture Overview

### Core Components

1. **eBPF Data Collection Layer**
   - `process.bpf.c`: Monitors system processes and executions
   - `sslsniff.bpf.c`: Captures SSL/TLS traffic data
   - Both programs output JSON events to stdout

2. **Rust Analysis Framework** (`collector/src/framework/`)
   - **Runners**: Execute eBPF binaries and stream events
   - **Analyzers**: Process and transform event streams
   - **Core Events**: Standardized event format with JSON payloads

3. **Streaming Pipeline Architecture**
   ```
   eBPF Binary → JSON Output → Runner → Analyzer Chain → Final Output
   ```

### Key Framework Components

- **`framework/core/events.rs`**: Core event system with standardized `Event` structure
- **`framework/runners/`**: Data collection implementations (SSL, Process, Fake)
- **`framework/analyzers/`**: Stream processing plugins (HTTP, FileLogger, Output)
- **`framework/binary_extractor.rs`**: Manages embedded eBPF binaries

### Event Flow

1. **Data Collection**: eBPF programs collect kernel events
2. **JSON Streaming**: Events converted to JSON and streamed via stdout
3. **Runner Processing**: Rust runners parse JSON and create event streams
4. **Analyzer Chain**: Multiple analyzers process events in sequence
5. **Output**: Final processed events sent to console, file, or storage

## Development Patterns

### Adding New eBPF Programs

1. Create `.bpf.c` file with eBPF kernel code
2. Create `.c` file with userspace loader
3. Add to `APPS` variable in `src/Makefile`
4. Include vmlinux.h for kernel structures
5. Use libbpf for userspace interaction

### Adding New Analyzers

1. Implement the `Analyzer` trait in `collector/src/framework/analyzers/`
2. Add async processing logic for event streams
3. Export in `analyzers/mod.rs`
4. Use in runner chains via `add_analyzer()`

### Configuration Management

- eBPF programs use command-line arguments for configuration
- Collector framework uses builder pattern for configuration
- Binary extraction handled automatically via `BinaryExtractor`

## Key Design Principles

1. **Streaming Architecture**: Real-time event processing with minimal memory usage
2. **Plugin System**: Extensible analyzer chains for flexible data processing
3. **Error Resilience**: Graceful handling of malformed data and analyzer failures
4. **Resource Management**: Automatic cleanup of temporary files and processes

## Testing Strategy

- **Unit Tests**: C tests for utility functions (`test_process_utils.c`)
- **Integration Tests**: Rust tests with `FakeRunner` for full pipeline testing
- **Manual Testing**: Direct execution of eBPF programs for validation

## Security Considerations

- eBPF programs require root privileges for kernel access
- SSL traffic captured includes potentially sensitive data
- Temporary binary extraction requires secure cleanup
- Process monitoring may expose system information

## Dependencies

- **C/eBPF**: libbpf, libelf, clang, llvm
- **Rust**: tokio, serde, clap, async-trait, chrono
- **System**: Linux kernel with eBPF support

## Common Issues

- **Permission Errors**: eBPF programs require sudo privileges
- **Kernel Compatibility**: eBPF programs may need kernel-specific vmlinux.h
- **Binary Extraction**: Temporary directory permissions may affect execution
- **UTF-8 Handling**: HTTP parser includes UTF-8 safety fixes for malformed data