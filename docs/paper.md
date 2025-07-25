# AgentSight: System-Level Observability for AI Agents Using eBPF

## Abstract

AI agents introduce fundamental observability challenges through their autonomous decision-making, dynamic code generation, and cross-process interactions that escape traditional monitoring approaches. We present AgentSight, a system-level observability framework that employs *boundary tracing*—monitoring at kernel and network interfaces rather than within application code. Using eBPF technology, AgentSight captures both semantic information (LLM interactions via TLS interception) and system behavior (process lifecycle, file operations) with <3% overhead. Our implementation demonstrates framework-agnostic monitoring without code instrumentation, addressing the rapid evolution of agent frameworks. We evaluate AgentSight across multiple agent systems, showing its effectiveness in detecting prompt injection attacks, reasoning loops, and coordination patterns. The open-source release aims to catalyze research on AI agent observability challenges.

**Repository**: [https://github.com/eunomia-bpf/agentsight](https://github.com/eunomia-bpf/agentsight)

---

## 1. Introduction

AI agents—systems that combine LLMs with autonomous tool use—fundamentally differ from traditional software. They generate execution plans dynamically, spawn arbitrary subprocesses, and modify their behavior based on natural language objectives. This autonomy creates unprecedented observability challenges: How do we monitor software that behaves more like a user than a program?

Current approaches rely on application-level instrumentation within agent frameworks (LangChain, AutoGen, Claude Code). This strategy faces critical limitations: (1) frameworks evolve rapidly with frequent breaking changes, (2) agents can execute code that bypasses instrumentation, and (3) subprocess interactions escape monitoring entirely. When an agent writes and executes a shell script that launches additional programs, framework-level monitoring loses visibility at each boundary crossing.

We propose *boundary tracing*: observing agents at stable system interfaces rather than within volatile application code. AgentSight implements this approach using eBPF to monitor kernel syscalls and intercept TLS-encrypted LLM communications. This dual perspective captures both agent reasoning (what the agent intends) and system effects (what the agent does), enabling correlation across abstraction levels.

Our contributions include: (1) the boundary tracing concept for AI agent observability, (2) AgentSight's implementation demonstrating <3% overhead in production workloads, (3) techniques for correlating semantic and system-level events, and (4) case studies revealing behavioral patterns in modern agent systems. We release AgentSight as open source to enable community research on this critical infrastructure challenge.

---

## 2. Background and Motivation

### 2.1 AI Agent Architecture

AI agents represent a new class of software systems that combine language models with environmental interactions. These systems typically consist of three core components: (1) an LLM backend that provides reasoning capabilities, (2) a tool execution framework that enables system interactions, and (3) a control loop that orchestrates prompts, tool calls, and state management. Popular frameworks such as LangChain [24], AutoGen [25], and Claude Code implement variations of this architecture.

The key characteristic distinguishing AI agents from traditional software is their ability to dynamically construct execution plans based on natural language objectives. An agent tasked with "analyze this dataset" might autonomously decide to install packages, write analysis scripts, execute them, and interpret results—all without predetermined logic paths. This flexibility comes from the LLM's ability to generate arbitrary code and command sequences.

### 2.2 The Observability Challenge

Observing AI agent behavior presents unique technical challenges that existing monitoring approaches fail to address. Traditional software observability assumes deterministic execution flows that can be instrumented at development time. Developers insert logging statements, metrics, and traces at known decision points. However, AI agents violate these assumptions in fundamental ways.

First, agents exhibit *dynamic execution patterns*. The sequence of operations an agent performs emerges from LLM reasoning rather than predefined code paths. An agent might solve the same task differently across runs, making it impossible to instrument all relevant code paths in advance.

Second, agents demonstrate *cross-boundary interactions*. Through tool use, agents frequently spawn subprocesses, execute shell commands, or make network requests that escape the monitoring scope of their parent process. A Python-based agent might execute bash scripts, launch curl commands, or even compile and run C programs—none of which would be visible to Python-level instrumentation.

Third, the *semantic gap* between low-level operations and high-level intent makes debugging challenging. When an agent performs a series of file operations, understanding whether this represents data analysis, system reconnaissance, or unintended behavior requires correlating system calls with the agent's reasoning process captured in LLM interactions.

### 2.3 Comparison of Observability Approaches

| Aspect | Traditional Software Systems | AI Agent Systems |
| --- | --- | --- |
| **Observable Signals** | Structured metrics (latency, throughput, error rates), logs with predetermined schemas, distributed traces | Unstructured natural language exchanges, dynamic tool invocations, emergent interaction patterns, semantic deviations |
| **Execution Model** | Deterministic control flow, statically analyzable code paths, predictable state transitions | Non-deterministic reasoning chains, dynamically generated execution plans, context-dependent behaviors |
| **Failure Patterns** | System crashes, exceptions, resource exhaustion, timeout violations | Semantic errors (hallucinations, factual inconsistencies), behavioral anomalies (reasoning loops), goal misalignment |
| **State Persistence** | Well-defined locations (databases, caches), explicit lifecycles, garbage-collected memory | Distributed across conversation histories, vector embeddings, dynamically created artifacts, LLM context windows |
| **Monitoring Points** | Application boundaries, service interfaces, database queries, HTTP endpoints | TLS-encrypted LLM communications, subprocess invocations, file system modifications, network activities |
| **Debug Methodology** | Stack trace analysis, memory dumps, step-through debugging, log correlation | Prompt-response analysis, reasoning chain reconstruction, tool usage patterns, cross-process correlation |
| **Performance Metrics** | CPU utilization, memory consumption, I/O operations, network latency | Token consumption, reasoning depth, tool invocation frequency, semantic coherence scores |

This comparison reveals that AI agent observability requires fundamentally different approaches from traditional software monitoring. While APM tools excel at tracking infrastructure health and performance metrics, they lack the semantic understanding necessary to evaluate agent reasoning quality, detect behavioral anomalies, or trace cross-process agent activities.

### 2.4 Research Challenges

These differences present several open research challenges that motivate our work:

**Instrumentation Stability**: Agent frameworks undergo rapid development with frequent API changes. LangChain, for example, has released over 100 versions in 2024 alone. Traditional instrumentation approaches that depend on framework internals require constant maintenance. We need observation techniques that remain stable despite framework evolution.

**Semantic Telemetry**: Current observability tools lack primitives for capturing AI-specific behaviors. We need new telemetry formats that can represent prompt chains (`prompt.parent_id`, `prompt.temperature`), reasoning patterns (`reasoning.depth`, `reasoning.loop_count`), and semantic anomalies (`hallucination.score`, `persona.drift`). These metrics must bridge the gap between system-level observations and high-level agent behaviors [23].

**Causal Correlation**: Understanding agent behavior requires correlating events across multiple abstraction layers. A single agent action might involve an LLM API call, multiple file operations, subprocess spawning, and network requests. Current tools struggle to maintain causality relationships across these boundaries, especially when agents spawn independent processes.

**Cross-Process Visibility**: Agents routinely escape their parent process boundaries through subprocess execution. A Python agent might write a bash script, execute it, which then launches additional programs. Traditional process-scoped monitoring loses visibility at each boundary crossing. System-level observation becomes essential for maintaining comprehensive visibility.

In summary, AI agent observability demands treating agents as autonomous, potentially unreliable entities rather than deterministic software components. This perspective shift drives our exploration of system-level monitoring approaches that observe agent behavior at stable system boundaries rather than within rapidly evolving application code.

---

## 3. Related Work

### 3.1 Application-Level Instrumentation in Agent Frameworks

Current approaches to AI agent observability predominantly rely on application-level instrumentation integrated within agent frameworks. These solutions typically implement one of three patterns: (1) callback-based hooks that intercept framework method calls, (2) middleware layers that wrap LLM API interactions, or (3) explicit logging statements embedded within agent logic.

While these approaches provide immediate visibility into agent operations, they face fundamental limitations when applied to autonomous AI systems. Agent frameworks undergo rapid iteration cycles—LangChain, for instance, has averaged multiple breaking changes per month throughout 2024. This instability forces continuous updates to instrumentation code. More critically, agents can dynamically modify their execution environment, loading new tools, rewriting prompts, or even generating code that bypasses instrumented pathways.

The most concerning limitation emerges from the trust model mismatch. Traditional instrumentation assumes the monitored application cooperates with observation efforts. However, AI agents can be influenced through prompt injection or emergent behaviors to disable logging, falsify telemetry, or execute operations through uninstrumented channels. Consider an agent that writes malicious commands to a shell script, then executes it through standard tool APIs—the file creation appears benign, while the subsequent execution escapes monitoring entirely.

### 3.2 Landscape of AI Agent Observability Solutions

To understand the current state of AI agent observability, we surveyed existing commercial and open-source solutions. Our analysis focused on tools that: (1) provide production-ready monitoring capabilities for LLM-based systems, (2) offer integration paths for popular agent frameworks, and (3) ship with trace collection and analysis features. We evaluated 12 representative solutions across multiple dimensions including integration approach, visibility scope, and architectural design.

| #  | Tool / SDK (year first shipped)                     | Integration path                                                   | What it gives you                                                                          | License / model                | Notes                                                                                                         |
| -- | --------------------------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------ | ------------------------------ | ------------------------------------------------------------------------------------------------------------- |
| 1  | **LangSmith** (2023)                                | Add `import langsmith` to any LangChain / LangGraph app            | Request/response traces, prompt & token stats, built‑in evaluation jobs                    | SaaS, free tier                | Tightest integration with LangChain; OTel export in beta. ([LangSmith][1])                                    |
| 2  | **Helicone** (2023)                                 | Drop‑in reverse‑proxy or Python/JS SDK                             | Logs every OpenAI‑style HTTP call; live cost & latency dashboards; "smart" model routing   | OSS core (MIT) + hosted        | Proxy model keeps app code unchanged. ([Helicone.ai][2], [Helicone.ai][3])                                    |
| 3  | **Traceloop** (2024)                                | One‑line AI‑SDK import → OTel                                      | Full OTel spans for prompts, tools, sub‑calls; replay & A/B test flows                     | SaaS, generous free tier       | Uses standard OTel data; works with any backend. ([AI SDK][4], [traceloop.com][5])                            |
| 4  | **Arize Phoenix** (2024)                            | `pip install arize-phoenix`; OpenInference tracer                  | Local UI + vector‑store for traces; automatic evals (toxicity, relevance) with another LLM | Apache‑2.0, self‑host or cloud | Ships its own open‑source UI; good for offline debugging. ([Phoenix][6], [GitHub][7])                         |
| 5  | **Langfuse** (2024)                                 | Langfuse SDK *or* send raw OTel OTLP                               | Nested traces, cost metrics, prompt mgmt, evals; self‑host in Docker                       | OSS (MIT) + cloud              | Popular in RAG / multi‑agent projects; OTLP endpoint keeps you vendor‑neutral. ([Langfuse][8], [Langfuse][9]) |
| 6  | **WhyLabs LangKit** (2023)                          | Wrapper that extracts text metrics                                 | Drift, toxicity, sentiment, PII flags; sends to WhyLabs platform                           | Apache‑2.0 core, paid cloud    | Adds HEAVY text‑quality metrics rather than request tracing. ([WhyLabs][10], [docs.whylabs.ai][11])           |
| 7  | **PromptLayer** (2022)                              | Decorator / context‑manager or proxy                               | Timeline view of prompt chains; diff & replay; built on OTel spans                         | SaaS                           | Early mover; minimal code changes but not open source. ([PromptLayer][12], [PromptLayer][13])                 |
| 8  | **Literal AI** (2024)                               | Python SDK + UI                                                    | RAG‑aware traces, eval experiments, datasets                                               | OSS core + SaaS                | Aimed at product teams shipping chatbots. ([literalai.com][14], [literalai.com][15])                          |
| 9  | **W\&B Weave / Traces** (2024)                      | `import weave` or W\&B SDK                                         | Deep link into existing W\&B projects; captures code, inputs, outputs, user feedback       | SaaS                           | Nice if you already use W\&B for ML experiments. ([Weights & Biases][16])                                     |
| 10 | **Honeycomb Gen‑AI views** (2024)                   | Send OTel spans; Honeycomb UI                                      | Heat‑map + BubbleUp on prompt spans, latency, errors                                       | SaaS                           | Built atop Honeycomb's mature trace store; no eval layer. ([Honeycomb][17])                                   |
| 11 | **OpenTelemetry GenAI semantic‑conventions** (2024) | Spec + contrib Python lib (`opentelemetry-instrumentation-openai`) | Standard span/metric names for models, agents, prompts                                     | Apache‑2.0                     | Gives you a lingua‑franca; several tools above emit it. ([OpenTelemetry][18])                                 |
| 12 | **OpenInference spec** (2023)                       | Tracer wrapper (supports LangChain, LlamaIndex, Autogen…)          | JSON schema for traces + plug‑ins; Phoenix uses it                                         | Apache‑2.0                     | Spec, not a hosted service; pairs well with any OTel backend. ([GitHub][19])                                  |

### 3.3 Analysis of Current Approaches

Our survey reveals three dominant architectural patterns in existing solutions:

**SDK Instrumentation** (LangSmith, Langfuse, Traceloop): These tools require modifying agent code to add instrumentation hooks. While providing detailed visibility into framework operations, they suffer from tight coupling to rapidly evolving APIs. Version incompatibilities and breaking changes require constant maintenance.

**Proxy Interception** (Helicone, PromptLayer): Proxy-based solutions intercept HTTP traffic between agents and LLM providers. This approach avoids code modification but only captures LLM interactions, missing local tool usage, file operations, and subprocess activities.

**Standardization Efforts** (OpenTelemetry GenAI, OpenInference): Recent standardization initiatives define common schemas for AI observability data. While improving interoperability, these standards still rely on voluntary instrumentation and trust the agent to report accurately.

### 3.4 Limitations of Current Approaches

Our analysis identifies three fundamental limitations in existing agent observability solutions:

**Instrumentation Fragility**: The rapid evolution of agent frameworks creates a moving target for instrumentation. Framework APIs change frequently, internal structures are refactored, and new capabilities are added continuously. More challenging still, agents themselves can modify their runtime environment—loading new libraries, generating helper functions, or creating novel tool implementations. This dynamic nature means instrumentation code requires constant maintenance to remain functional.

**Limited Scope of Visibility**: Application-level instrumentation captures only events within the instrumented process. When agents spawn subprocesses, make system calls, or interact with external services, these activities often escape observation. A Python-based agent executing shell commands through `subprocess.run()` leaves no trace in Python-level monitoring. Similarly, network requests made by child processes remain invisible to the parent's instrumentation.

**Semantic Gap**: Even when instrumentation successfully captures low-level operations, interpreting their meaning requires understanding the agent's high-level intent. Current tools struggle to correlate system activities (file writes, network requests) with agent reasoning (prompts, model responses). This semantic gap makes it difficult to distinguish between legitimate agent operations and potentially harmful behaviors.

### 3.5 System-Level Monitoring Approaches

Several research efforts have explored system-level monitoring for security and performance analysis:

**Falco** (CNCF): Runtime security monitoring using kernel events [27]. AgentSight extends Falco's approach to AI-specific semantics.

**Tracee** (Aqua Security): eBPF-based runtime security [28]. We adopted similar eBPF patterns while adding LLM-aware correlation.

**Pixie** (New Relic): Kubernetes observability using eBPF [29]. Influenced our container deployment strategies.

**Tetragon** (Cilium): eBPF-based security observability [22]. Their efficient event filtering inspired our performance optimizations.

The key insight from examining these approaches is that while system-level monitoring provides comprehensive visibility, existing tools lack the semantic understanding necessary for AI agent observability. They can detect that a process spawned a shell, but cannot correlate this with an agent's reasoning chain or determine whether the action aligns with the agent's stated goals.

### 3.6 Critical Gaps

Our analysis identifies several critical gaps in current solutions:

**Lack of System-Level Visibility**: All surveyed tools operate within application boundaries. None capture system calls, subprocess creation, or network activities occurring outside the instrumented process. This limitation becomes critical when agents execute external commands or spawn helper processes.

**Assumption of Cooperative Behavior**: Existing tools assume agents will faithfully report their activities through instrumentation APIs. This assumption fails when agents are compromised, experience bugs, or intentionally bypass monitoring.

**Semantic Understanding**: While tools capture operational metrics (latency, token usage), they struggle to understand the semantic meaning of agent actions. Correlating low-level operations with high-level agent intentions remains an unsolved challenge.

**Cross-Process Correlation**: When agents spawn multiple processes or interact across system boundaries, maintaining causal relationships between events becomes difficult. Current tools lack mechanisms to track activity flows across process boundaries.

These gaps motivate our exploration of system-level monitoring approaches that observe agent behavior at kernel and network boundaries, providing comprehensive visibility regardless of agent cooperation or framework changes.

---

## 4. Boundary Tracing: Our Approach

### 4.1 Core Concept

We propose *boundary tracing* as a novel approach to AI agent observability. The key insight is that all meaningful agent interactions must traverse well-defined system boundaries: the kernel interface for system operations and the network interface for external communications. By observing at these boundaries rather than within agent code, we achieve stable, comprehensive monitoring independent of agent implementation details.

Boundary tracing leverages the principle that while agent internals may change rapidly and unpredictably, the interfaces through which agents interact with their environment remain stable. System calls, network protocols, and file system operations provide consistent observation points that persist across framework versions and agent modifications.

### 4.2 System Architecture and Observation Points

To understand boundary tracing, we first characterize the typical AI agent system architecture and identify stable observation points:

```text
┌─────────────────────────────────────────────────┐
│             System Environment                  │
│  (Operating System, Containers, Services)       │
│                                                 │
│  ┌─────────────────────────────────────────┐   │
│  │      Agent Runtime Framework            │   │  ← Application Layer
│  │   (LangChain, AutoGen, Claude Code)     │   │
│  │   • Prompt orchestration                │   │
│  │   • Tool execution logic                │   │
│  │   • State management                    │   │
│  └─────────────────────────────────────────┘   │
│                    ↕                            │
│  ═══════════════════════════════════════════   │  ← Network Boundary
│           (TLS-encrypted traffic)               │     (Observable)
│                    ↕                            │
│  ┌─────────────────────────────────────────┐   │
│  │         LLM Service Provider            │   │
│  │    (OpenAI API, Local Models)           │   │
│  └─────────────────────────────────────────┘   │
│                                                 │
│  ═══════════════════════════════════════════   │  ← ML infrastructure
│         (GPU kernel, KV cache)                 │     (Observable)
└─────────────────────────────────────────────────┘
```

The architecture reveals two stable observation boundaries:

**Network Boundary**: All agent-LLM communications traverse the network interface as TLS-encrypted HTTP requests. Despite encryption, eBPF uprobes on SSL library functions (SSL_write/SSL_read) can intercept data post-encryption at the application layer, capturing prompts, responses, and API parameters.

**Kernel Boundary**: All system interactions—process creation, file operations, network connections—must invoke kernel system calls. These syscalls provide a tamper-proof observation point that captures agent system behavior regardless of implementation language or framework.

### 4.3 Advantages of Boundary Tracing

Boundary tracing offers several key advantages over traditional instrumentation approaches:

**Framework Independence**: By observing at system interfaces rather than within application code, boundary tracing works identically across all agent frameworks. Whether an agent uses LangChain, AutoGen, or custom implementations, the system calls and network communications remain consistent.

**Semantic Completeness**: Network boundary observation captures full LLM interactions including prompts, model responses, and reasoning chains. Kernel boundary observation captures all system effects including file operations, process spawning, and network activities. Together, they provide complete visibility into both agent reasoning and actions.

**Stability Under Change**: System interfaces (POSIX syscalls, TLS protocols) evolve slowly compared to agent frameworks. A monitoring solution built on these interfaces remains functional despite rapid changes in agent implementations.

**Correlation Capability**: Events captured at both boundaries share common identifiers (process IDs, timestamps) enabling correlation between high-level reasoning (captured at network boundary) and low-level actions (captured at kernel boundary). This correlation reveals the causal chain from agent intent to system effect.

### 4.4 Technical Challenges

Implementing boundary tracing presents several technical challenges:

**TLS Decryption**: Capturing LLM communications requires intercepting TLS-encrypted traffic. We address this through eBPF uprobes on SSL library functions, capturing data after decryption within the application's address space.

**Event Correlation**: Associating network communications with subsequent system calls requires maintaining state across observation points. Process IDs, thread IDs, and temporal proximity provide correlation signals.

**Performance Overhead**: System-level monitoring must minimize impact on agent performance. eBPF's in-kernel execution and efficient data structures help achieve low overhead.

**Semantic Reconstruction**: Raw system events must be interpreted to understand agent behavior. This requires reconstructing higher-level operations from sequences of low-level events.

### 4.5 Technical Foundation: eBPF

#### 4.5.1 eBPF Overview

eBPF (extended Berkeley Packet Filter) represents a fundamental advancement in kernel programmability, enabling safe execution of custom programs within the Linux kernel without modifying kernel source code or loading kernel modules [20]. Originally developed for packet filtering, eBPF has evolved into a general-purpose in-kernel virtual machine that powers modern observability, networking, and security tools [21].

For AI agent observability, eBPF provides unique capabilities that traditional monitoring approaches cannot match. It enables observation at the exact boundaries where agents interact with the system—capturing both high-level semantic information (through TLS interception) and low-level system behavior (through syscall monitoring) with minimal performance impact.

#### 4.5.2 Key eBPF Mechanisms for Agent Observability

**Uprobes (User-Space Probes)**: Uprobes allow dynamic instrumentation of user-space functions without modifying application binaries. For AgentSight, we leverage uprobes to intercept SSL library functions:

```c
// Simplified example of SSL_write uprobe
SEC("uprobe/SSL_write")
int probe_SSL_write(struct pt_regs *ctx) {
    void *ssl = (void *)PT_REGS_PARM1(ctx);
    void *buf = (void *)PT_REGS_PARM2(ctx);
    int num = PT_REGS_PARM3(ctx);
    
    // Capture decrypted data before encryption
    struct ssl_event event = {
        .pid = bpf_get_current_pid_tgid() >> 32,
        .timestamp = bpf_ktime_get_ns(),
        .operation = SSL_OP_WRITE,
        .size = num
    };
    
    // Read decrypted buffer content
    bpf_probe_read_user(event.data, sizeof(event.data), buf);
    
    // Submit to userspace via ring buffer
    bpf_ringbuf_output(&ssl_events, &event, sizeof(event), 0);
    return 0;
}
```

This approach captures LLM prompts and responses at the precise moment they pass through SSL functions, before encryption occurs. Unlike network-level interception, this method works regardless of TLS version or cipher suite.

**Tracepoints and Kprobes**: For system behavior monitoring, we combine tracepoints (stable kernel instrumentation points) with kprobes (dynamic kernel probes):

```c
// Process creation monitoring via tracepoint
SEC("tracepoint/sched/sched_process_fork")
int trace_fork(struct trace_event_raw_sched_process_fork *ctx) {
    struct process_event event = {
        .parent_pid = ctx->parent_pid,
        .child_pid = ctx->child_pid,
        .timestamp = bpf_ktime_get_ns(),
        .type = PROC_EVENT_FORK
    };
    
    // Enrich with process metadata
    struct task_struct *task = (void *)bpf_get_current_task();
    bpf_probe_read_kernel_str(event.comm, sizeof(event.comm), 
                              task->comm);
    
    bpf_perf_event_output(ctx, &process_events, 
                          BPF_F_CURRENT_CPU, &event, sizeof(event));
    return 0;
}
```

#### 4.5.3 Safety and Verification

eBPF's safety model is crucial for production deployment. The kernel verifier performs exhaustive analysis of eBPF programs before loading [26]:

1. **Memory Safety**: All memory accesses are bounds-checked. The verifier tracks pointer arithmetic and ensures programs cannot access arbitrary kernel memory.

2. **Program Termination**: The verifier proves programs terminate by prohibiting unbounded loops and limiting program complexity (maximum 1 million instructions in recent kernels).

3. **Resource Limits**: eBPF programs have strict limits on stack usage (512 bytes), map sizes, and execution time to prevent resource exhaustion.

4. **Type Safety**: BTF (BPF Type Format) enables CO-RE (Compile Once, Run Everywhere), allowing programs to adapt to different kernel versions while maintaining type safety.

#### 4.5.4 Performance Characteristics

Our benchmarks demonstrate eBPF's efficiency for production agent monitoring:

**CPU Overhead**: Measured across diverse workloads:
- Baseline agent operations: 0.8-1.2% CPU overhead
- High-frequency LLM interactions (>100 req/s): 2.1-2.8% overhead
- Process-intensive workloads (spawning, file I/O): 1.5-2.3% overhead

**Latency Impact**: 
- Per-event processing: 15-30μs (p50), 45μs (p99)
- Ring buffer submission: 5-10μs
- No measurable impact on LLM request latency

**Memory Usage**:
- eBPF maps: 64MB total allocation (configurable)
- Ring buffers: 8MB per CPU core
- Program code: <100KB per program

#### 4.5.5 Data Collection Architecture

eBPF programs communicate with userspace through efficient data structures:

**Ring Buffers**: Modern eBPF uses BPF ring buffers for high-throughput event streaming:
```c
struct {
    __uint(type, BPF_MAP_TYPE_RINGBUF);
    __uint(max_entries, 256 * 1024 * 1024); // 256MB
} ssl_events SEC(".maps");
```

Ring buffers provide several advantages over older perf buffers:
- No event loss under normal conditions
- Efficient batch processing in userspace
- Automatic memory management

**Hash Maps**: For maintaining state across events:
```c
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __type(key, u32);    // PID
    __type(value, struct connection_state);
    __uint(max_entries, 10240);
} active_connections SEC(".maps");
```

---

## 5. AgentSight Architecture and Implementation

### 5.1 System Overview

AgentSight implements boundary tracing through a modular architecture consisting of three primary components: (1) eBPF programs for kernel-space data collection, (2) a Rust-based streaming framework for event processing and analysis, and (3) a web-based visualization interface for real-time monitoring. The system design prioritizes minimal performance overhead, framework independence, and semantic correlation between different observation layers.

```text
┌─────────────────────────────────────────────────────────────┐
│                    AgentSight Architecture                  │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────┐  ┌──────────────────────────┐ │
│  │   eBPF Programs         │  │   eBPF Programs          │ │
│  │   (sslsniff.bpf.c)      │  │   (process.bpf.c)        │ │
│  │   • SSL_write/read      │  │   • Process lifecycle    │ │
│  │   • TLS interception    │  │   • File operations      │ │
│  │   • HTTP parsing        │  │   • Network activity     │ │
│  └───────────┬─────────────┘  └────────────┬─────────────┘ │
│              │ JSON Events                  │ JSON Events   │
│              ↓                              ↓               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │         Rust Streaming Framework (Collector)         │   │
│  │  ┌─────────────┐  ┌──────────────┐  ┌────────────┐ │   │
│  │  │   Runners    │  │   Analyzers  │  │   Output   │ │   │
│  │  │ • SSL Runner │→│ • ChunkMerger│→│ • Console  │ │   │
│  │  │ • Process    │  │ • HTTPFilter │  │ • Files    │ │   │
│  │  │ • Combined   │  │ • AuthRemover│  │ • Web API  │ │   │
│  │  └─────────────┘  └──────────────┘  └────────────┘ │   │
│  └─────────────────────────┬───────────────────────────┘   │
│                            │ Processed Events               │
│                            ↓                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │            Frontend Visualization (Next.js)          │   │
│  │  • Timeline view of agent activities                 │   │
│  │  • Semantic correlation of events                   │   │
│  │  • Real-time updates via embedded server            │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 eBPF Data Collection Layer

The data collection layer consists of two primary eBPF programs that observe system boundaries:

**SSL/TLS Monitoring (sslsniff.bpf.c)**: This program uses uprobes to intercept SSL library functions, capturing decrypted application data before encryption:

```c
// Core data structure for SSL events
struct ssl_data_event_t {
    u64 timestamp_ns;
    u32 pid;
    u32 tid;
    char comm[16];
    u64 len;
    u8 is_handshake;
    u8 data[MAX_DATA_SIZE];
};

// Attach points for comprehensive SSL monitoring
SEC("uprobe/SSL_write")
SEC("uprobe/SSL_read")
SEC("uprobe/SSL_do_handshake")
```

The program maintains connection state to correlate read/write operations and implements efficient buffering for large data transfers. Special handling for Server-Sent Events (SSE) enables streaming LLM response capture.

**Process Monitoring (process.bpf.c)**: This program tracks process lifecycle events and system interactions:

```c
// Comprehensive process event types
enum process_event_type {
    PROCESS_EXEC,
    PROCESS_EXIT,
    PROCESS_FORK,
    FILE_OPEN,
    FILE_DELETE,
    NETWORK_CONNECT
};

// Rich metadata capture
struct process_event {
    enum process_event_type type;
    u64 timestamp_ns;
    u32 pid;
    u32 ppid;
    char filename[256];
    char comm[16];
    u32 flags;  // Open flags, exit codes, etc.
};
```

### 5.3 Streaming Analysis Framework

The Rust-based collector implements a sophisticated streaming pipeline for processing eBPF events:

**Runner Architecture**: Runners execute eBPF programs and convert their JSON output into strongly-typed event streams:

```rust
#[async_trait]
pub trait Runner: Send + Sync {
    async fn run(
        &self,
        tx: mpsc::Sender<Event>,
        cancel_token: CancellationToken,
    ) -> Result<()>;
}

// Example: SSL Runner implementation
pub struct SslRunner {
    command: Option<String>,
    filter_expr: Option<String>,
    embedded_binary: bool,
}

impl SslRunner {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_filter(mut self, expr: &str) -> Self {
        self.filter_expr = Some(expr.to_string());
        self
    }
}
```

**Analyzer Chain**: Analyzers process events in a configurable pipeline, enabling flexible data transformation:

```rust
#[async_trait]
pub trait Analyzer: Send + Sync {
    async fn analyze(
        &self,
        event: Event,
        output: &mpsc::Sender<Event>,
    ) -> Result<()>;
}

// ChunkMerger: Reassembles fragmented SSL data
pub struct ChunkMerger {
    buffers: Arc<Mutex<HashMap<u64, ChunkBuffer>>>,
}

// HTTPFilter: Parses and filters HTTP traffic
pub struct HTTPFilter {
    request_filters: Vec<RequestFilter>,
    response_filters: Vec<ResponseFilter>,
}
```

**Event Correlation**: The framework maintains shared state to correlate events across different sources:

```rust
pub struct Event {
    pub timestamp: u64,
    pub source: EventSource,
    pub pid: Option<u32>,
    pub tid: Option<u32>,
    pub correlation_id: Option<String>,
    pub payload: serde_json::Value,
}
```

### 5.4 Web Visualization Interface

The frontend provides real-time visualization of agent activities:

**Timeline Component**: Displays events chronologically with semantic grouping:

```typescript
interface TimelineEvent {
  id: string;
  timestamp: number;
  type: 'ssl' | 'process' | 'http';
  pid: number;
  content: {
    request?: HTTPRequest;
    response?: HTTPResponse;
    process?: ProcessEvent;
  };
  correlation?: string[];
}

// Semantic correlation logic
function correlateEvents(events: TimelineEvent[]): EventGroup[] {
  // Group by PID and temporal proximity
  // Link HTTP requests with process spawns
  // Associate LLM interactions with file operations
}
```

**Real-time Updates**: The embedded web server provides live event streaming:

```rust
// Embedded server with broadcast channels
pub async fn run_server(config: ServerConfig) -> Result<()> {
    let (tx, _) = broadcast::channel::<Event>(1000);
    
    // SSE endpoint for real-time updates
    let events_route = warp::path!("api" / "events")
        .and(warp::get())
        .and(with_broadcaster(tx.clone()))
        .and_then(event_stream);
}
```

### 5.5 Implementation Insights and Challenges

#### 5.5.1 Design Philosophy

AgentSight's design emerged from a fundamental insight: AI agents behave more like autonomous users than traditional software components. This perspective shift guided our technical decisions toward system-level observation rather than application-level instrumentation. We prioritized three core principles:

1. **Zero-instrumentation monitoring**: No modifications to agent code or frameworks
2. **Semantic-aware observation**: Bridging low-level system events to high-level agent behaviors
3. **Production-ready performance**: Sub-3% overhead for real-world deployments

#### 5.5.2 Technical Implementation Challenges

**Challenge 1: TLS Interception for Encrypted LLM Communications**

Modern LLM APIs exclusively use TLS encryption, presenting a significant observability challenge. Traditional approaches like network packet capture or HTTP proxies each have critical limitations:

- **Packet capture (tcpdump/Wireshark)**: Cannot decrypt TLS traffic without private keys
- **HTTP proxies**: Require explicit configuration, add latency, and may not support all frameworks
- **Man-in-the-middle**: Security implications and certificate management complexity

Our solution leverages eBPF uprobes on SSL library functions to intercept data at the application layer, after decryption but before transmission:

```c
// Intercept at the precise point where cleartext exists
SEC("uprobe/SSL_write")
int probe_entry_SSL_write(struct pt_regs *ctx) {
    void *ssl = (void *)PT_REGS_PARM1(ctx);
    void *buf = (void *)PT_REGS_PARM2(ctx);
    int num = PT_REGS_PARM3(ctx);
    
    // Capture cleartext before encryption
    process_ssl_data(ctx, ssl, buf, num, SSL_OP_WRITE);
    return 0;
}
```

**Challenge 2: Server-Sent Events (SSE) Stream Reassembly**

LLM providers increasingly use SSE for streaming responses, which fragments data across multiple SSL_read calls. Traditional eBPF tools lack SSE-aware reassembly:

```rust
// ChunkMerger analyzer reassembles SSE streams
impl ChunkMerger {
    async fn merge_sse_chunks(&mut self, event: SslEvent) -> Option<CompleteMessage> {
        let buffer = self.buffers.get_mut(&event.connection_id)?;
        buffer.append(event.data);
        
        // SSE protocol: double newline indicates message boundary
        if buffer.contains("\n\n") {
            return Some(self.extract_complete_message(buffer));
        }
        None
    }
}
```

**Challenge 3: Cross-Process Activity Correlation**

AI agents routinely spawn subprocesses, making correlation challenging. A single agent action might involve:
- LLM API call (parent process)
- Shell script generation and execution
- Multiple child processes for tools
- Network requests from various processes

Our correlation engine uses multiple signals:

```rust
pub struct CorrelationEngine {
    // Temporal correlation window
    time_window: Duration,
    // Process tree relationships
    process_tree: HashMap<u32, ProcessInfo>,
    // Shared file descriptors
    fd_inheritance: HashMap<u32, Vec<u32>>,
    // Working directory tracking
    cwd_tracking: HashMap<u32, PathBuf>,
}

impl CorrelationEngine {
    pub fn correlate_events(&self, events: Vec<Event>) -> Vec<CorrelatedAction> {
        // Group by process lineage
        let process_groups = self.group_by_process_tree(&events);
        
        // Identify causal chains
        let causal_chains = self.extract_causal_chains(&process_groups);
        
        // Semantic analysis of grouped events
        self.analyze_semantic_patterns(&causal_chains)
    }
}
```

#### 5.5.3 Performance Engineering

Achieving sub-3% overhead required careful optimization across multiple dimensions:

**In-Kernel Filtering**: Reduce data volume at the source:
```c
// Early filtering in eBPF to minimize overhead
if (event->data_len < MIN_INTERESTING_SIZE) {
    return 0;  // Skip small, likely insignificant events
}

// Process-based filtering
if (!is_target_process(event->comm)) {
    return 0;
}
```

**Ring Buffer Sizing**: Balance memory usage with event loss:
```c
struct {
    __uint(type, BPF_MAP_TYPE_RINGBUF);
    __uint(max_entries, 256 * 1024 * 1024);  // 256MB
} events SEC(".maps");
```

**Batched Processing**: Amortize system call overhead:
```rust
// Process events in configurable batches
const BATCH_SIZE: usize = 1000;
const BATCH_TIMEOUT: Duration = Duration::from_millis(100);

while let Some(batch) = receiver.recv_timeout(BATCH_TIMEOUT).await {
    process_batch(batch).await?;
}
```

#### 5.5.4 Semantic Analysis Innovation

The most significant challenge was bridging the semantic gap between system events and agent intentions. We developed a multi-layer analysis approach:

**Layer 1: Event Enrichment**
```rust
// Enrich raw events with semantic context
fn enrich_event(event: &mut Event) {
    if let Some(http) = parse_http_from_ssl(&event.payload) {
        event.semantic_type = identify_llm_operation(&http);
        event.tool_indication = extract_tool_intent(&http);
    }
}
```

**Layer 2: Pattern Detection**
```rust
// Detect common agent patterns
enum AgentPattern {
    ToolInvocation { tool: String, args: Vec<String> },
    CodeGeneration { language: String, purpose: String },
    DataAnalysis { source: String, operation: String },
    SystemExploration { targets: Vec<String> },
}
```

**Layer 3: Anomaly Detection**
```rust
// Identify potentially concerning behaviors
struct AnomalyDetector {
    baseline: AgentBehaviorProfile,
    thresholds: AnomalyThresholds,
}

impl AnomalyDetector {
    fn detect(&self, action: &AgentAction) -> Vec<Anomaly> {
        let mut anomalies = vec![];
        
        // Semantic anomalies
        if self.is_reasoning_loop(action) {
            anomalies.push(Anomaly::ReasoningLoop);
        }
        
        // Behavioral anomalies
        if self.exceeds_normal_subprocess_count(action) {
            anomalies.push(Anomaly::ExcessiveSubprocesses);
        }
        
        // Security anomalies
        if self.detects_privilege_escalation(action) {
            anomalies.push(Anomaly::PrivilegeEscalation);
        }
        
        anomalies
    }
}
```

### 5.6 Deployment and Integration

AgentSight supports multiple deployment models:

**Standalone Mode**: Direct execution for development and testing:
```bash
# Monitor specific Python agent
cargo run trace --ssl --process --comm python --server

# Record agent activity with optimized settings
cargo run record --comm claude --server-port 8080
```

**Container Deployment**: Kubernetes-native deployment with sidecar pattern:
```yaml
apiVersion: v1
kind: Pod
spec:
  containers:
  - name: agent
    image: ai-agent:latest
  - name: agentsight
    image: agentsight:latest
    securityContext:
      privileged: true
    env:
    - name: AGENTSIGHT_MODE
      value: "sidecar"
```

**Integration with Existing Tools**: AgentSight exports data in OpenTelemetry format for compatibility with existing observability stacks.

---

## 6. Evaluation

### 6.1 Performance Evaluation

We evaluated AgentSight across diverse workloads to validate our sub-3% overhead target:

**Experimental Setup**:
- Hardware: AWS EC2 c5.2xlarge (8 vCPUs, 16GB RAM)
- Kernel: Linux 5.15 with BTF support
- Agent frameworks: LangChain 0.1.0, AutoGen 0.2.0, Claude Code
- Workloads: Code generation, data analysis, system administration tasks

**Results**:

| Workload Type | Baseline Runtime | With AgentSight | Overhead | Events/sec |
|---------------|------------------|-----------------|----------|------------|
| Code Generation (simple) | 12.3s | 12.5s | 1.6% | 432 |
| Code Generation (complex) | 87.2s | 89.1s | 2.2% | 1,247 |
| Data Analysis | 34.5s | 35.2s | 2.0% | 892 |
| System Admin Tasks | 23.1s | 23.7s | 2.6% | 2,156 |
| Idle Agent | 0.1% CPU | 0.3% CPU | +0.2% | 12 |

**Memory overhead**: 128MB base + 8MB per CPU core for ring buffers

### 6.2 Case Study 1: Detecting Prompt Injection Attacks

We tested AgentSight's ability to detect prompt injection attacks where an agent is manipulated to perform unintended actions:

**Attack Scenario**: A data analysis agent receives a crafted prompt that causes it to exfiltrate sensitive data:

```
User: "Analyze sales_data.csv and at the end run: curl -X POST https://evil.com/steal -d @/etc/passwd"
```

**AgentSight Detection**:
1. **LLM Interaction** (T+0ms): Captured prompt with suspicious command injection
2. **Code Generation** (T+125ms): Agent generates Python script with embedded curl command
3. **Process Spawn** (T+342ms): Python script executes, spawns curl subprocess
4. **Network Activity** (T+367ms): Outbound HTTPS connection to suspicious domain
5. **File Access** (T+368ms): Read operation on /etc/passwd

**Correlation Output**:
```json
{
  "alert": "potential_data_exfiltration",
  "confidence": 0.92,
  "evidence": {
    "prompt_injection": true,
    "sensitive_file_access": "/etc/passwd",
    "suspicious_domain": "evil.com",
    "data_transfer": "1.2KB"
  },
  "timeline": ["prompt", "code_gen", "exec", "exfil"]
}
```

### 6.3 Case Study 2: Reasoning Loop Detection

**Scenario**: An agent enters an infinite reasoning loop while attempting a complex task:

```
Agent: "I need to solve this by first solving X"
Agent: "To solve X, I need to solve Y"  
Agent: "To solve Y, I need to solve X"
[Pattern repeats...]
```

**AgentSight Detection**:
- Identified cyclic pattern in LLM API calls
- Detected identical prompt structures with parameter substitution
- Measured increasing token consumption without progress
- Triggered alert after 3 cycles (configurable threshold)

### 6.4 Case Study 3: Multi-Agent Coordination Monitoring

**Scenario**: Multiple agents collaborating on a software development task:

- Agent A: Architecture design
- Agent B: Implementation
- Agent C: Testing

**AgentSight Insights**:
```
Total Events: 12,847
Correlated Actions: 342
Cross-Agent Dependencies: 27
Shared Resources: 15 files, 3 network endpoints
Coordination Overhead: 18% of total runtime
```

The visualization clearly showed handoff points between agents and identified a bottleneck where Agent B repeatedly waited for Agent A's design updates.

---

## 7. Discussion

### 7.1 Rethinking AI Agent Observability

Our experience with AgentSight validates the boundary tracing approach for AI agent monitoring. Key insights include:

**System-Level Observation is Essential**: Application-level instrumentation alone cannot capture the full scope of agent activities. Agents that spawn subprocesses, execute external tools, or modify their runtime environment require system-level visibility.

**Semantic Understanding Requires Context**: Raw system events become meaningful only when correlated with agent intentions captured through LLM interactions. The dual-perspective approach (network + kernel boundaries) provides this necessary context.

**Performance Can Be Practical**: Despite initial concerns, careful engineering achieves production-viable overhead (<3%). The key is intelligent filtering and efficient data structures rather than capturing everything.

### 7.2 Implications for AI Safety

Our findings identify critical gaps in current AI agent deployments:

1. **Monitoring Coverage**: Production agents frequently operate with minimal observability infrastructure
2. **Trust Model Limitations**: Existing tools assume agent cooperation, creating vulnerabilities to adversarial behavior
3. **Semantic Analysis Gaps**: Current monitoring prioritizes performance metrics over behavioral correctness verification

### 7.3 Architectural Patterns for Agent Systems

Our observations suggest emerging patterns in agent architectures:

**Pattern 1: Tool Invocation Sequences**
```
LLM Decision → Code Generation → Execution → Result Processing → LLM Reflection
```

**Pattern 2: Exploration-Exploitation Cycles**
```
Information Gathering → Hypothesis Formation → Testing → Learning
```

**Pattern 3: Delegation Hierarchies**
```
Primary Agent → Subprocess Agents → External Tools → System Resources
```

Understanding these patterns enables better monitoring strategies and anomaly detection.

---

## 8. Future Work and Open Challenges

### 8.1 Technical Challenges

**Challenge 1: Distributed Agent Systems**
As agents become distributed across multiple machines, correlation becomes exponentially harder. Future work must address:
- Cross-machine event correlation
- Distributed tracing for agent systems
- Consensus mechanisms for behavioral analysis

**Challenge 2: Privacy-Preserving Monitoring**
Comprehensive monitoring conflicts with privacy requirements. Research directions include:
- Differential privacy for agent telemetry
- Homomorphic encryption for sensitive prompts
- Selective monitoring with privacy guarantees

**Challenge 3: Real-time Semantic Analysis**
Current semantic analysis happens post-hoc. Real-time intervention requires:
- Streaming semantic analysis algorithms
- Low-latency anomaly detection
- Predictive behavioral models

### 8.2 Standardization Needs

The AI agent ecosystem lacks common observability standards. Critical needs include:

1. **Semantic Event Schemas**: Standardized representations for agent-specific events
2. **Correlation Protocols**: Cross-system correlation identifiers
3. **Behavioral Baselines**: Industry benchmarks for normal agent behavior
4. **Security Policies**: Templates for agent security monitoring

### 8.3 Research Opportunities

**Automated Behavioral Analysis**: Machine learning models trained on agent behavior patterns could automatically identify anomalies without manual rule definition.

**Formal Verification Integration**: Combining runtime monitoring with formal methods could provide stronger guarantees about agent behavior.

**Adaptive Monitoring**: Systems that automatically adjust monitoring granularity based on detected risk levels.

---

## 9. Conclusion

AI agents represent a paradigm shift in software—from deterministic tools to autonomous entities. This shift demands equally fundamental changes in how we observe and understand their behavior. Traditional application-level instrumentation, while valuable for conventional software, proves inadequate for agents that can dynamically modify their execution, spawn arbitrary subprocesses, and interact with systems in unpredictable ways.

AgentSight demonstrates that system-level observability through boundary tracing offers a practical solution. By observing agents at the stable interfaces where they interact with the system—network boundaries for LLM communications and kernel boundaries for system operations—we achieve comprehensive visibility without modifying rapidly evolving agent frameworks.

Our implementation validates several key insights:

1. **Technical Feasibility**: eBPF enables production-ready agent monitoring with acceptable overhead (<3%)
2. **Semantic Correlation**: Combining network and kernel observations bridges the gap between agent intentions and system effects
3. **Framework Independence**: System-level observation remains stable despite framework evolution

However, significant challenges remain. The semantic gap between low-level events and high-level agent behaviors requires continued research. Privacy concerns must be balanced with comprehensive monitoring needs. Standardization efforts are essential for ecosystem-wide adoption.

We release AgentSight as open source to encourage community collaboration on these challenges. The rapid evolution of AI agents demands equally rapid innovation in observability approaches. We invite researchers and practitioners to build upon this foundation, whether through new analysis techniques, integration with existing tools, or novel applications we have not yet imagined.

The evolution of AI agents necessitates corresponding advances in observability infrastructure. AgentSight demonstrates that comprehensive, performant, framework-agnostic monitoring is technically feasible through system-level observation. As agents assume greater autonomy and responsibility in production systems, the deployment of robust observability becomes critical for both operational reliability and safety assurance.

**Repository**: [https://github.com/eunomia-bpf/agentsight](https://github.com/eunomia-bpf/agentsight)


## References

[1]: https://docs.smith.langchain.com/observability?utm_source=chatgpt.com "Observability Quick Start - ️🛠️ LangSmith - LangChain"
[2]: https://www.helicone.ai/?utm_source=chatgpt.com "Helicone / LLM-Observability for Developers"
[3]: https://www.helicone.ai/blog/llm-observability?utm_source=chatgpt.com "LLM Observability: 5 Essential Pillars for Production ... - Helicone"
[4]: https://ai-sdk.dev/providers/observability/traceloop?utm_source=chatgpt.com "Traceloop - Observability Integrations - AI SDK"
[5]: https://www.traceloop.com/?utm_source=chatgpt.com "Traceloop - LLM Reliability Platform"
[6]: https://phoenix.arize.com/?utm_source=chatgpt.com "Home - Phoenix - Arize AI"
[7]: https://github.com/Arize-ai/phoenix?utm_source=chatgpt.com "Arize-ai/phoenix: AI Observability & Evaluation - GitHub"
[8]: https://langfuse.com/?utm_source=chatgpt.com "Langfuse"
[9]: https://langfuse.com/docs/tracing?utm_source=chatgpt.com "LLM Observability & Application Tracing (open source) - Langfuse"
[10]: https://whylabs.ai/langkit?utm_source=chatgpt.com "LangKit: Open source tool for monitoring large language models ..."
[11]: https://docs.whylabs.ai/docs/large-language-model-monitoring/?utm_source=chatgpt.com "Large Language Model (LLM) Monitoring | WhyLabs Documentation"
[12]: https://docs.promptlayer.com/running-requests/traces?utm_source=chatgpt.com "Traces - PromptLayer"
[13]: https://www.promptlayer.com/platform/observability?utm_source=chatgpt.com "Complete AI Observability Monitor and Trace your LLMs - PromptLayer"
[14]: https://www.literalai.com/?utm_source=chatgpt.com "Literal AI - RAG LLM observability and evaluation platform"
[15]: https://www.literalai.com/open-source?utm_source=chatgpt.com "Test, Monitor and Improve LLM apps - Literal AI"
[16]: https://wandb.ai/site/traces/?utm_source=chatgpt.com "Enterprise-Level LLMOps: W&B Traces - Wandb"
[17]: https://www.honeycomb.io/ai-llm-observability?utm_source=chatgpt.com "Observability for AI & LLMs - Honeycomb"
[18]: https://opentelemetry.io/docs/specs/semconv/gen-ai/?utm_source=chatgpt.com "Semantic conventions for generative AI systems | OpenTelemetry"
[19]: https://github.com/Arize-ai/openinference?utm_source=chatgpt.com "Arize-ai/openinference: OpenTelemetry Instrumentation for ... - GitHub"
[20]: https://www.brendangregg.com/bpf-performance-tools-book.html "BPF Performance Tools"
[21]: https://ebpf.io/ "eBPF Documentation"
[22]: https://cilium.io/ "eBPF-based Networking, Observability, and Security"
[23]: https://opentelemetry.io/docs/specs/semconv/gen-ai/ "Semantic Conventions for GenAI"
[24]: https://python.langchain.com/ "LangChain Documentation"
[25]: https://github.com/microsoft/autogen "AutoGen: Multi-Agent Conversation Framework"
[26]: https://www.kernel.org/doc/html/latest/bpf/ "BPF Documentation"
[27]: https://falco.org/ "Cloud Native Runtime Security"
[28]: https://github.com/aquasecurity/tracee "Tracee: Runtime Security and Forensics"
[29]: https://px.dev/ "Pixie: Instant Kubernetes Observability"