# Agent Tracer

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/yunwei37/agent-tracer)

> **Zero-instrumentation observability framework for AI agent using eBPF-based system-level analysis**

Agent Tracer is a comprehensive observability framework designed specifically for monitoring AI agent behavior through SSL/TLS traffic interception and process monitoring. Unlike traditional application-level instrumentation, Agent Tracer observes at the system boundary using eBPF technology, providing tamper-resistant insights into AI agent interactions with minimal performance overhead.

## 🚀 Key Features

- **🔍 System-Level Observability**: Monitor AI agents without code modifications using eBPF
- **🔐 SSL/TLS Traffic Interception**: Capture encrypted communications in real-time 
- **⚡ Process Lifecycle Tracking**: Monitor process creation, execution, and file operations
- **🔄 Streaming Architecture**: Real-time event processing with pluggable analyzers
- **🛡️ Tamper-Resistant**: Independent monitoring that can't be easily compromised by agents
- **🏗️ Framework Agnostic**: Works with any AI agent framework (LangChain, AutoGen, etc.)
- **📊 Rich Analytics**: HTTP parsing, correlation analysis, and semantic event processing

## 🎯 Problem Statement

AI agent systems present unique observability challenges:

| Traditional Software | AI Agent Systems |
|---------------------|------------------|
| Deterministic behavior | Non-deterministic, emergent behavior |
| Structured logs/metrics | Semantics hidden in TLS payloads |
| Predictable failure modes | Hallucinations, prompt injection, reasoning loops |
| Request-scoped state | Long-lived conversations and memories |

**Agent Tracer bridges this gap** by providing system-level observability that captures the true behavior of AI agents, including their interactions with external systems, without relying on potentially compromised application-level instrumentation.

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────┐
│                 AI Agent System                 │
│  ┌─────────────────┐  ┌─────────────────────┐   │
│  │   SSL Traffic   │  │    Process Events   │   │
│  │   Monitoring    │  │    Monitoring       │   │
│  └─────────────────┘  └─────────────────────┘   │
│           │                      │              │
│           ▼                      ▼              │
│  ┌─────────────────────────────────────────┐    │
│  │         eBPF Data Collection           │    │
│  └─────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────┐
│            Rust Analysis Framework              │
│  ┌─────────────┐  ┌──────────────┐  ┌────────┐  │
│  │   Runners   │  │  Analyzers   │  │ Output │  │
│  │ (Collectors)│  │ (Processors) │  │        │  │
│  └─────────────┘  └──────────────┘  └────────┘  │
└─────────────────────────────────────────────────┘
```

### Core Components

1. **eBPF Data Collection**
   - `sslsniff`: Intercepts SSL/TLS read/write operations 
   - `process`: Monitors process lifecycle and file operations
   - Kernel-level hooks with <3% performance overhead

2. **Rust Streaming Framework**
   - **Runners**: Execute eBPF programs and stream JSON events
   - **Analyzers**: Process and transform event streams  
   - **Event System**: Standardized event format with rich metadata

3. **Analysis Pipeline**
   ```
   eBPF → JSON Stream → Runner → Analyzer Chain → Output
   ```

## 🚀 Quick Start

### Prerequisites

- Linux kernel 4.1+ with eBPF support
- Root privileges for eBPF program loading
- Rust 1.88.0+ (for building collector)

### Installation

```bash
# Clone repository
git clone https://github.com/yunwei37/agent-tracer.git --recursive
cd agent-tracer

# Install dependencies (Ubuntu/Debian)
make install

# Build eBPF programs
make build

# Build Rust collector
cd collector && cargo build --release
```

### Usage Examples

#### Monitor SSL Traffic
```bash
# Monitor all SSL/TLS traffic
sudo ./src/sslsniff

# Monitor specific process
sudo ./src/sslsniff -p <PID>

# Use Rust collector with HTTP parsing
cd collector && cargo run ssl --sse-merge
```

#### Monitor Process Activity
```bash
# Monitor all processes
sudo ./src/process

# Monitor specific commands
sudo ./src/process -c "python,node"

# Use Rust collector 
cd collector && cargo run process
```

#### Combined Monitoring
```bash
# Monitor both SSL and processes concurrently
cd collector && cargo run agent --comm python --pid 1234
```

### Docker Quick Start

```bash
# Run pre-built container
sudo docker run --rm -it --privileged ghcr.io/yunwei37/agent-tracer:latest

# Build local container
docker build -t agent-tracer .
sudo docker run --rm -it --privileged agent-tracer
```

## 📊 Output Examples

### SSL Traffic Event
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": 1640995200000,
  "source": "ssl",
  "event_type": "ssl_data",
  "data": {
    "pid": 1234,
    "comm": "python",
    "direction": "write", 
    "data_len": 512,
    "payload": "GET /api/chat HTTP/1.1\r\nHost: api.openai.com...",
    "parsed_http": {
      "method": "POST",
      "path": "/v1/chat/completions",
      "headers": {"Authorization": "Bearer sk-..."}
    }
  }
}
```

### Process Event
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440001", 
  "timestamp": 1640995201000,
  "source": "process",
  "event_type": "process_exec",
  "data": {
    "pid": 5678,
    "ppid": 1234,
    "comm": "curl",
    "filename": "/usr/bin/curl",
    "args": ["curl", "-X", "POST", "https://api.github.com"]
  }
}
```

## 🔧 Configuration

### eBPF Program Options

**SSL Monitor (`sslsniff`)**:
```bash
sudo ./src/sslsniff [OPTIONS]
  -p, --pid <PID>     Monitor specific process ID
  -c, --comm <NAME>   Monitor processes by command name  
  --extra             Extended output format
```

**Process Monitor (`process`)**:
```bash
sudo ./src/process [OPTIONS]
  -m, --mode <MODE>   Filter mode: 0=all, 1=proc, 2=filter (default: 2)
  -c, --comm <NAMES>  Comma-separated process names to monitor
  -d, --duration <MS> Minimum process duration in milliseconds
```

### Collector Framework

The Rust collector provides a flexible streaming architecture:

```rust
// SSL monitoring with HTTP parsing
let ssl_runner = SslRunner::from_binary_extractor(ssl_path)
    .with_args(vec!["--port", "443"])
    .add_analyzer(Box::new(ChunkMerger::new()))
    .add_analyzer(Box::new(HttpAnalyzer::new()))
    .add_analyzer(Box::new(OutputAnalyzer::new()));

// Combined monitoring
let agent_runner = AgentRunner::new(ssl_path, process_path)
    .with_comm_filter("python")
    .add_analyzer(Box::new(CorrelationAnalyzer::new()));
```

## 🔍 Use Cases

### AI Agent Security
- **Prompt Injection Detection**: Monitor unexpected API calls or system commands
- **Data Exfiltration Prevention**: Track file operations and network communications
- **Compliance Auditing**: Maintain tamper-resistant logs of agent interactions

### Development & Debugging  
- **Agent Behavior Analysis**: Understand how agents interact with external systems
- **Performance Optimization**: Identify bottlenecks in agent workflows
- **Integration Testing**: Verify agent interactions with APIs and databases

### Research & Analysis
- **Agent Interaction Patterns**: Study how agents use tools and APIs
- **Cross-Agent Correlation**: Analyze interactions between multiple agents
- **Semantic Anomaly Detection**: Identify unusual patterns in agent behavior

## 🏢 Enterprise Features

- **Multi-Agent Orchestration**: Monitor complex agent workflows
- **Real-time Alerting**: Custom analyzers for specific security patterns  
- **Data Pipeline Integration**: JSON output compatible with ELK, Splunk, etc.
- **Kubernetes Support**: Deploy as DaemonSet for cluster-wide monitoring

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

```bash
# Clone with submodules
git clone --recursive https://github.com/yunwei37/agent-tracer.git

# Install development dependencies
make install

# Run tests
make test
cd collector && cargo test

# Build debug versions
make debug
```

### Architecture Documentation

- [CLAUDE.md](CLAUDE.md) - Project guidelines and architecture overview
- [collector/DESIGN.md](collector/DESIGN.md) - Detailed framework design
- [docs/why.md](docs/why.md) - Problem statement and motivation

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built on [libbpf](https://github.com/libbpf/libbpf) for eBPF program management
- Inspired by the need for better observability in AI agent systems
- Thanks to the eBPF community for tools and documentation

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/yunwei37/agent-tracer/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yunwei37/agent-tracer/discussions)
- **Documentation**: [Project Wiki](https://github.com/yunwei37/agent-tracer/wiki)

---

**🚨 Security Notice**: This tool is designed for defensive security and monitoring purposes. Use responsibly and in compliance with applicable laws and regulations.