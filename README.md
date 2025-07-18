# AgentSight: Zero-Instrumentation AI Agent Observability with eBPF

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/eunomia-bpf/agentsight)

AgentSight is a comprehensive observability framework designed specifically for monitoring AI agent behavior through SSL/TLS traffic interception and process monitoring. Unlike traditional application-level instrumentation, AgentSight observes at the system boundary using eBPF technology, providing tamper-resistant insights into AI agent interactions with minimal performance overhead.

**✨ Zero Instrumentation Required** - No code changes, no new dependencies, no SDKs. Works with any AI framework or application out of the box.

## Quick Start

```bash
wget https://github.com/eunomia-bpf/agentsight/releases/download/v0.0.48/agentsight && chmod +x agentsight
# Record agent behavior from claude
sudo ./agentsight record -c "claude"
```

Visit [http://127.0.0.1:8080](http://127.0.0.1:8080) to view the recorded data.

<div align="center">
  <img src="docs/demo-tree.png" alt="AgentSight Demo - Process Tree Visualization" width="800">
  <p><em>Real-time process tree visualization showing AI agent interactions and file operations</em></p>
</div>

<div align="center">
  <img src="docs/demo-timeline.png" alt="AgentSight Demo - Timeline Visualization" width="800">
  <p><em>Real-time timeline visualization showing AI agent interactions and system calls</em></p>
</div>


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

## Usage

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
make -C bpf debug
```

### Key Resources

- [CLAUDE.md](CLAUDE.md) - Project guidelines and architecture
- [collector/DESIGN.md](collector/DESIGN.md) - Framework design details
- [docs/why.md](docs/why.md) - Problem analysis and motivation

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

---

**💡 The Future of AI Observability**: As AI agents become more autonomous and capable of self-modification, traditional observability approaches become insufficient. AgentSight provides the independent, tamper-resistant monitoring foundation needed for safe AI deployment at scale.
