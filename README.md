# AgentSight: Zero‑instrumentation AI observability, powered by eBPF

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/yunwei37/agent-tracer)

`AgentSight` is a observability framework designed specifically for monitoring LLM applications and AI agents behavior through SSL/TLS traffic interception and system level behavior tracing. Unlike traditional application-level instrumentation tools, AgentSight observes **black box AI applications** at the system boundary using eBPF technology, providing tamper-resistant insights into AI agent interactions with minimal performance overhead. `*No code changes required, zero new dependencies, no new SDKs; Works for most frameworks and applications out of box.*`

## 🚀 Key Advantages Over Existing Solutions

### **vs. LangSmith/Helicone/Langfuse (Application-Level Tools)**

| **Challenge**                               | **Their approach**                               | **AgentSight's solution**                                           |
| ------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------------------- |
| **Getting started on a new stack, adopting a new framework**          | ❌ Add a new SDK / proxy for *each* framework, New plug‑in every time APIs change      | ✅ Drop‑in daemon and tooling; no code or env‑var changes                        |
| **Using commercial close source tools (claude‑code, …)**          | ❌ Hard to analysis, limited visibility into it's operations      | ✅ Have visibility into it's prompts, plan, behaviors, and more                        |
| **Agents that write code to create and run tools**         | ❌ Only trace the execution of the agent tools      | ✅ Tracks every process behaviors at minimal performance overhead, like shell cmd, file‑I/O, network call, etc.       |
| **Self‑modifying / prompt‑injected agents** | ❌ Logs can be silenced or faked in‑process       | ✅ Kernel‑level hooks record raw TLS & syscalls—tamper‑resistant     |
| **Encrypted LLM traffic**                   | ❌ Only what the wrapper emits; ciphertext unseen | ✅ Uprobes capture the *real* unencrypted request / response |
| **Cross‑agent coordination**                | ❌ Each process and framework traced in isolation               | ✅ Global analysis, and more            |

### **The System Boundary Advantage**

**AgentSight captures what others miss: interactions with the environment**

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
│                     ↕ (Can be silenced)         │
├─────────────────────────────────────────────────┤ ← System Boundary
│  🟢 AgentSight eBPF Monitoring (Tamper-proof)   │
│  ┌─────────────────┐  ┌─────────────────────┐   │
│  │   SSL Traffic   │  │    Process Events   │   │
│  │   Monitoring    │  │    Monitoring       │   │
│  └─────────────────┘  └─────────────────────┘   │
└─────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────┐
│            Rust Analysis Framework              │
│  ┌─────────────┐  ┌──────────────┐  ┌────────┐  │
│  │   Runners   │  │  Analyzers   │  │ Output │  │
│  │ (Collectors)│  │ (Processors) │  │        │  │
│  └─────────────┘  └──────────────┘  └────────┘  │
└─────────────────────────────────────────────────┘
```

### Core Components

1. **eBPF Data Collection** (Kernel Space)
   - `sslsniff`: Intercepts SSL/TLS read/write operations using uprobe hooks
   - `process`: Monitors process lifecycle and file operations via tracepoints
   - <3% performance overhead, operates below application layer

2. **Rust Streaming Framework** (User Space)
   - **Runners**: Execute eBPF programs and stream JSON events
   - **Analyzers**: Process and transform event streams with pluggable architecture
   - **Event System**: Standardized event format with rich metadata

3. **Analysis Pipeline**

   ```
   eBPF Hooks → Raw Data → JSON Stream → Runner → Analyzer Chain → Output
   ```

## 🚀 Quick Start

### Prerequisites

- **Linux kernel**: 4.1+ with eBPF support (5.0+ recommended)
- **Root privileges**: Required for eBPF program loading
- **Rust toolchain**: 1.88.0+ (for building collector)
- **Build tools**: clang, llvm, libelf-dev

### Installation

```bash
# Clone repository with submodules
git clone https://github.com/yunwei37/agent-tracer.git --recursive
cd agent-tracer

# Install system dependencies (Ubuntu/Debian)
make install

# Build eBPF programs
make build

# Build Rust collector
cd collector && cargo build --release
```

## ❓ Frequently Asked Questions

### General

**Q: What makes Agent Tracer different from traditional APM tools?**  
A: Agent Tracer operates at the kernel level using eBPF, providing tamper-resistant monitoring that agents cannot easily bypass or manipulate. Traditional APM requires instrumentation that can be compromised.

**Q: Does Agent Tracer impact application performance?**  
A: Minimal impact (<1% CPU overhead). eBPF runs in kernel space with optimized data collection, avoiding the overhead of userspace monitoring.

**Q: Can agents detect they're being monitored?**  
A: Detection is extremely difficult since monitoring occurs at the kernel level without modifying application code or injecting libraries.

### Technical

**Q: Which Linux distributions are supported?**  
A: Any distribution with kernel 4.1+ and eBPF support. Tested on Ubuntu 20.04+, CentOS 8+, RHEL 8+, and Amazon Linux 2.

**Q: Can I monitor multiple agents simultaneously?**  
A: Yes, use the `agent`  modes to monitor multiple processes concurrently with automatic event correlation.

### Troubleshooting

**Q: "Permission denied" when running eBPF programs**  
A: Ensure you're running with `sudo` or have `CAP_BPF` and `CAP_SYS_ADMIN` capabilities.

**Q: "Failed to load eBPF program" errors**  
A: Check kernel version (`uname -r`) and eBPF support (`zgrep BPF /proc/config.gz`). Update vmlinux.h if needed.

## Use cases

### Deployment Models

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md).

### Development Setup

```bash
# Clone with submodules
git clone --recursive https://github.com/yunwei37/agent-tracer.git

# Install development dependencies  
make install

# Run tests
make test
cd collector && cargo test

# Build debug versions with AddressSanitizer
make debug
```

### Architecture Documentation

- [CLAUDE.md](CLAUDE.md) - Project guidelines and architecture overview
- [collector/DESIGN.md](collector/DESIGN.md) - Detailed framework design
- [docs/why.md](docs/why.md) - Comprehensive problem analysis and motivation

## 📄 License

**💡 Why Agent Tracer?** In an era where AI agents can modify their own behavior, traditional observability falls short. Agent Tracer provides the independent, tamper-resistant monitoring that organizations need to safely deploy AI agents at scale.
