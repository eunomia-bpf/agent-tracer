# AgentSight: Zero-Instrumentation AI Agent Observability with eBPF

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/eunomia-bpf/agentsight)

AgentSight is a comprehensive observability framework designed specifically for monitoring AI agent behavior through SSL/TLS traffic interception and process monitoring. Unlike traditional application-level instrumentation, AgentSight observes at the system boundary using eBPF technology, providing tamper-resistant insights into AI agent interactions with minimal performance overhead.

**✨ Zero Instrumentation Required** - No code changes, no new dependencies, no SDKs. Works with any AI framework or application out of the box.

## 🚀 Why AgentSight?

### Traditional Observability vs. System-Level Monitoring

| **Challenge** | **Application-Level Tools** | **AgentSight Solution** |
|---------------|----------------------------|------------------------|
| **Framework Adoption** | ❌ New SDK/proxy for each framework | ✅ Drop-in daemon, no code changes |
| **Closed-Source Tools** | ❌ Limited visibility into operations | ✅ Complete visibility into prompts & behaviors |
| **Dynamic Agent Behavior** | ❌ Logs can be silenced or manipulated | ✅ Kernel-level hooks, tamper-resistant |
| **Encrypted Traffic** | ❌ Only sees wrapper outputs | ✅ Captures real unencrypted requests/responses |
| **System Interactions** | ❌ Misses subprocess executions | ✅ Tracks all process behaviors & file operations |
| **Multi-Agent Systems** | ❌ Isolated per-process tracing | ✅ Global correlation and analysis |

### System Boundary Advantage

AgentSight captures critical interactions that application-level tools miss:

- Subprocess executions that bypass instrumentation
- Raw encrypted payloads before agent processing
- File operations and system resource access  
- Cross-agent communications and coordination

## 🏗️ Architecture

```ascii
┌─────────────────────────────────────────────────┐
│              AI Agent Runtime                   │
│   ┌─────────────────────────────────────────┐   │
│   │    Application-Level Observability      │   │
│   │  (LangSmith, Helicone, Langfuse, etc.)  │   │
│   │         🔴 Tamper Vulnerable             │   │
│   └─────────────────────────────────────────┘   │
│                     ↕ (Can be bypassed)         │
├─────────────────────────────────────────────────┤ ← System Boundary
│  🟢 AgentSight eBPF Monitoring (Tamper-proof)   │
│  ┌─────────────────┐  ┌─────────────────────┐   │
│  │   SSL Traffic   │  │    Process Events   │   │
│  │   Monitoring    │  │    Monitoring       │   │
│  └─────────────────┘  └─────────────────────┘   │
└─────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────┐
│         Rust Streaming Analysis Framework       │
│  ┌─────────────┐  ┌──────────────┐  ┌────────┐  │
│  │   Runners   │  │  Analyzers   │  │ Output │  │
│  │ (Collectors)│  │ (Processors) │  │        │  │
│  └─────────────┘  └──────────────┘  └────────┘  │
└─────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────┐
│           Frontend Visualization                │
│     Timeline • Process Tree • Event Logs       │
└─────────────────────────────────────────────────┘
```

### Core Components

1. **eBPF Data Collection** (Kernel Space)
   - **SSL Monitor**: Intercepts SSL/TLS read/write operations via uprobe hooks
   - **Process Monitor**: Tracks process lifecycle and file operations via tracepoints
   - **<3% Performance Overhead**: Operates below application layer with minimal impact

2. **Rust Streaming Framework** (User Space)
   - **Runners**: Execute eBPF programs and stream JSON events (SSL, Process, Agent, Combined)
   - **Analyzers**: Pluggable processors for HTTP parsing, chunk merging, filtering, logging
   - **Event System**: Standardized event format with rich metadata and JSON payloads

3. **Frontend Visualization** (React/TypeScript)
   - **Timeline View**: Interactive event timeline with zoom and filtering
   - **Process Tree**: Hierarchical process visualization with lifecycle tracking
   - **Log View**: Raw event inspection with syntax highlighting
   - **Real-time Updates**: Live data streaming and analysis

### Data Flow Pipeline

```
eBPF Programs → JSON Events → Runners → Analyzer Chain → Frontend/Storage/Output
```

## 🚀 Quick Start

### Prerequisites

- **Linux kernel**: 4.1+ with eBPF support (5.0+ recommended)
- **Root privileges**: Required for eBPF program loading
- **Rust toolchain**: 1.88.0+ (for building collector)
- **Node.js**: 18+ (for frontend development)
- **Build tools**: clang, llvm, libelf-dev

### Installation

```bash
# Clone repository with submodules
git clone https://github.com/eunomia-bpf/agentsight.git --recursive
cd agentsight

# Install system dependencies (Ubuntu/Debian)
make install

# Build all components (frontend, eBPF, and Rust)
make build

# Or build individually:
# make build-frontend  # Build frontend assets
# make build-bpf       # Build eBPF programs  
# make build-rust      # Build Rust collector

```

### Basic Usage

#### Monitor SSL Traffic

```bash
# Using collector framework with filtering
cd collector && cargo run ssl --sse-merge -- -p 1234
```

#### Monitor Process Lifecycle

```bash
# Using collector framework
cd collector && cargo run process -- -c python
```

#### Combined Agent Monitoring

```bash
# Monitor specific agent by PID or command
cd collector && cargo run agent --comm python --pid 1234
```

#### Web Interface

```bash
# Using embedded web server
cd collector && cargo run server
# Open http://localhost:8080/timeline

# Using Next.js development server
cd frontend && npm run dev
# Open http://localhost:3000/timeline
```

## 🔧 Usage Examples

### Real-World Scenarios

#### Monitoring Claude Code AI Assistant

```bash
# Monitor Claude Code interactions
cd collector && cargo run agent -- --comm claude-code
```

#### Analyzing LangChain Applications

```bash
# Monitor Python-based LangChain agents
cd collector && cargo run agent -- --comm python
```

#### Debugging Multi-Agent Systems

```bash
# Monitor all agent processes with correlation
cd collector && cargo run ssl -- --sse-merge > agents.log
```

### Configuration Options

#### SSL Traffic Filtering

```bash
# Filter by port and merge SSE streams
cd collector && cargo run ssl -- --port 443 --sse-merge

# Monitor specific processes
cd collector && cargo run ssl -- --comm python
```

#### Process Monitoring

```bash
# Monitor file operations for specific commands
cd collector && cargo run process -- --comm python

# Filter by process name
cd collector && cargo run process -- --comm node
```

## 📊 Visualization Features

### Timeline View

- Interactive timeline with zoom and pan
- Event grouping by type and source
- Real-time filtering and search
- Minimap for navigation

### Process Tree View  

- Hierarchical process visualization
- Lifecycle tracking (fork, exec, exit)
- Resource usage monitoring
- Parent-child relationship mapping

### Log View

- Raw event inspection with JSON formatting
- Syntax highlighting and pretty printing
- Export capabilities (JSON, CSV)
- Error detection and validation

## 🔍 Advanced Features

### HTTP Traffic Analysis

- Automatic HTTP request/response parsing
- Header extraction and analysis
- Chunked transfer encoding support
- Authentication token filtering

### Event Correlation

- Cross-process event correlation
- Timeline synchronization
- Causal relationship detection
- Performance metrics calculation

### Security Features

- Tamper-resistant kernel-level monitoring
- Encrypted traffic decryption
- Privilege escalation detection
- Suspicious behavior alerting

## ❓ Frequently Asked Questions

### General

**Q: How does AgentSight differ from traditional APM tools?**  
A: AgentSight operates at the kernel level using eBPF, providing tamper-resistant monitoring that agents cannot bypass. Traditional APM requires instrumentation that can be compromised.

**Q: What's the performance impact?**  
A: Minimal impact (<3% CPU overhead). eBPF runs in kernel space with optimized data collection.

**Q: Can agents detect they're being monitored?**  
A: Detection is extremely difficult since monitoring occurs at the kernel level without code modification.

### Technical

**Q: Which Linux distributions are supported?**  
A: Any distribution with kernel 4.1+ and eBPF support. Tested on Ubuntu 20.04+, CentOS 8+, RHEL 8+.

**Q: Can I monitor multiple agents simultaneously?**  
A: Yes, use combined monitoring modes for concurrent multi-agent observation with correlation.

**Q: How do I filter sensitive data?**  
A: Built-in analyzers can remove authentication headers and filter specific content patterns.

### Troubleshooting

**Q: "Permission denied" errors**  
A: Ensure you're running with `sudo` or have `CAP_BPF` and `CAP_SYS_ADMIN` capabilities.

**Q: "Failed to load eBPF program" errors**  
A: Check kernel version and eBPF support. Update vmlinux.h for your architecture if needed.

**Q: Frontend not loading data**  
A: Verify the collector is running and check network connectivity to port 8080.

## 📁 Project Structure

```
agentsight/
├── bpf/                   # Core eBPF programs
│   ├── sslsniff.bpf.c     # SSL/TLS traffic monitoring
│   ├── process.bpf.c      # Process lifecycle tracking
│   └── *.c                # Userspace loaders
├── collector/             # Rust analysis framework (agentsight package)
│   ├── src/framework/     # Core streaming framework
│   ├── src/main.rs        # CLI entry point
│   └── DESIGN.md          # Architecture documentation
├── frontend/              # React/TypeScript visualization
│   ├── src/components/    # UI components
│   └── src/app/           # Next.js application
├── docs/                  # Documentation
├── script/                # Python analysis tools
└── vmlinux/               # Kernel headers
```

## 🤝 Contributing

We welcome contributions! See our development setup:

```bash
# Clone with submodules
git clone --recursive https://github.com/eunomia-bpf/agentsight.git

# Install development dependencies  
make install

# Run tests
make test

# Frontend development
cd frontend && npm run dev

# Build debug versions with AddressSanitizer
make debug
```

### Key Resources

- [CLAUDE.md](CLAUDE.md) - Project guidelines and architecture
- [collector/DESIGN.md](collector/DESIGN.md) - Framework design details
- [docs/why.md](docs/why.md) - Problem analysis and motivation

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

---

**💡 The Future of AI Observability**: As AI agents become more autonomous and capable of self-modification, traditional observability approaches become insufficient. AgentSight provides the independent, tamper-resistant monitoring foundation needed for safe AI deployment at scale.
