# AgentSight Collector

A high-performance Rust streaming framework for real-time AI agent observability through eBPF-based system monitoring. The collector provides a pluggable architecture for processing SSL/TLS traffic and process lifecycle events with minimal overhead.

## Overview

The AgentSight Collector is the core data processing engine that:

- Executes embedded eBPF programs for system-level monitoring
- Processes event streams through configurable analyzer chains
- Provides multiple runners for different monitoring scenarios
- Offers real-time streaming with async/await architecture
- Supports flexible output formats and destinations

## Architecture

```text
eBPF Programs → JSON Events → Runners → Analyzer Chains → Output
```

### Core Components

- **Runners**: Execute eBPF binaries and create event streams
- **Analyzers**: Process and transform events in configurable chains
- **Events**: Standardized event format with rich metadata
- **Binary Extractor**: Manages embedded eBPF programs with automatic cleanup

## Quick Start

### Installation

```bash
# Install dependencies
sudo apt-get update
sudo apt-get install -y clang llvm libelf-dev

# Clone and build
git clone <repository>
cd agent-tracer/collector
cargo build --release
```

### Basic Usage

```bash
# SSL traffic monitoring with HTTP parsing
sudo ./target/release/collector ssl --http-parser

# Process lifecycle monitoring
sudo ./target/release/collector process

# Combined agent monitoring
sudo ./target/release/collector agent --comm python --pid 1234
```

## Commands

### SSL Monitoring

Monitor SSL/TLS traffic with advanced processing capabilities:

```bash
# Basic SSL monitoring
sudo collector ssl

# Enable Server-Sent Events processing
sudo collector ssl --sse-merge

# Enable HTTP parsing with raw data
sudo collector ssl --http-parser --http-raw-data

# Apply filters to reduce noise
sudo collector ssl --http-parser --http-filter "GET /health" --ssl-filter "handshake"

# Pass arguments to underlying eBPF program
sudo collector ssl -- --port 443 --interface eth0
```

### Process Monitoring

Track process lifecycle events:

```bash
# Basic process monitoring
sudo collector process

# Filter by process name
sudo collector process -- --comm python

# Filter by PID
sudo collector process -- --pid 1234

# Quiet mode (no console output)
sudo collector process --quiet
```

### Agent Monitoring (Combined)

Comprehensive monitoring with both SSL and process events:

```bash
# Full agent monitoring
sudo collector agent

# Filter by process command
sudo collector agent --comm python

# SSL-only monitoring
sudo collector agent --process false

# Process-only monitoring
sudo collector agent --ssl false

# Advanced filtering
sudo collector agent --pid 1234 --ssl-uid 1000 --http-filter "POST /api"

# Custom output file
sudo collector agent --output /var/log/agent.log --quiet
```

## Configuration Options

### SSL Options

- `--sse-merge`: Enable Server-Sent Events processing
- `--http-parser`: Parse HTTP requests/responses from SSL traffic
- `--http-raw-data`: Include raw SSL data in HTTP events
- `--http-filter`: Filter HTTP events by pattern
- `--ssl-filter`: Filter SSL events by pattern

### Process Options

- `--comm`: Filter by process command name
- `--pid`: Filter by process ID
- `--duration`: Minimum process duration in ms
- `--mode`: Process filtering mode (0=all, 1=proc, 2=filter)

### Agent Options

- `--ssl`: Enable/disable SSL monitoring
- `--process`: Enable/disable process monitoring
- `--ssl-uid`: Filter SSL events by user ID
- `--ssl-handshake`: Show SSL handshake events
- `--output`: Output file path
- `--quiet`: Suppress console output

## Framework Architecture

### Runners

Runners execute eBPF programs and create event streams:

```rust
// SSL Runner
let ssl_runner = SslRunner::from_binary_extractor(binary_path)
    .with_args(&["--port", "443"])
    .add_analyzer(Box::new(HTTPParser::new()))
    .add_analyzer(Box::new(OutputAnalyzer::new()));

// Process Runner
let process_runner = ProcessRunner::from_binary_extractor(binary_path)
    .with_args(&["--comm", "python"])
    .add_analyzer(Box::new(OutputAnalyzer::new()));

// Agent Runner (combines SSL + Process)
let agent_runner = AgentRunner::new("agent")
    .add_runner(Box::new(ssl_runner))
    .add_runner(Box::new(process_runner))
    .add_global_analyzer(Box::new(FileLogger::new("agent.log")));
```

### Analyzers

Analyzers process event streams in configurable chains:

- **SSEProcessor**: Merges HTTP chunks and processes Server-Sent Events
- **HTTPParser**: Parses HTTP requests/responses from SSL traffic
- **HTTPFilter**: Filters HTTP events by patterns
- **SSLFilter**: Filters SSL events by patterns
- **FileLogger**: Logs events to files
- **OutputAnalyzer**: Outputs events to console

### Event Format

Events use a standardized format:

```rust
pub struct Event {
    pub timestamp: u64,
    pub source: String,
    pub pid: u32,
    pub comm: String,
    pub data: serde_json::Value,
}
```

## Performance

- **Low Overhead**: eBPF monitoring with <3% performance impact
- **Async Processing**: Tokio-based async/await architecture
- **Streaming**: Real-time event processing with minimal memory usage
- **Configurable**: Modular analyzer chains for optimal performance

## Examples

### SSL Traffic Analysis

```bash
# Monitor HTTPS traffic with HTTP parsing
sudo collector ssl --http-parser --http-filter "POST /api" -- --port 443

# Monitor multiple ports
sudo collector ssl --sse-merge -- --port 80 --port 443
```

### Process Lifecycle Tracking

```bash
# Monitor Python processes
sudo collector process -- --comm python --duration 1000

# Monitor specific PID
sudo collector process -- --pid 1234
```

### Combined Monitoring

```bash
# Monitor web application
sudo collector agent --comm nginx --ssl-uid 33 --http-filter "GET /metrics"

# Full system monitoring
sudo collector agent --output /var/log/system.log --quiet
```

## Development

### Building

```bash
# Development build
cargo build

# Release build with optimizations
cargo build --release

# Run tests
cargo test
```

### Adding Analyzers

```rust
use async_trait::async_trait;
use futures::stream::Stream;

#[async_trait]
impl Analyzer for MyAnalyzer {
    async fn process(&mut self, mut stream: EventStream) -> Result<EventStream, AnalyzerError> {
        // Process events and return transformed stream
    }
}
```

### Binary Embedding

The collector automatically embeds eBPF binaries at compile time:

```rust
let binary_extractor = BinaryExtractor::new().await?;
let ssl_path = binary_extractor.get_sslsniff_path();
let process_path = binary_extractor.get_process_path();
```

## Security

- **Root Privileges**: eBPF programs require root access for kernel monitoring
- **Tamper Resistant**: System-level monitoring prevents agent manipulation
- **Data Sensitivity**: SSL traffic may contain sensitive information
- **Secure Cleanup**: Automatic cleanup of temporary files and processes

## Troubleshooting

### Common Issues

1. **Permission Denied**: Ensure running with `sudo` or appropriate capabilities
2. **eBPF Not Supported**: Requires Linux kernel 4.1+ with eBPF enabled
3. **Binary Extraction Failed**: Check `/tmp` permissions and disk space
4. **High CPU Usage**: Reduce event volume with filters

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug cargo run ssl --http-parser

# Verbose eBPF program output
sudo collector ssl -- --verbose
```

## Requirements

- **Rust**: 1.88.0 or later (edition 2024)
- **Linux**: Kernel 4.1+ with eBPF support
- **Libraries**: clang, llvm, libelf-dev
- **Privileges**: Root access for eBPF operations

## Dependencies

- **tokio**: Async runtime and stream processing
- **serde**: JSON serialization and deserialization
- **clap**: Command-line argument parsing
- **chrono**: Timestamp handling
- **futures**: Stream utilities and async processing

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass: `cargo test`
5. Submit a pull request

## License

[License information would go here]

## Related Projects

- **AgentSight**: Complete observability framework
- **eBPF Programs**: Low-level monitoring components (`../src/`)
- **Frontend**: Web interface for visualization (`../frontend/`)
- **Analysis Tools**: Python utilities for data processing (`../script/`)
 