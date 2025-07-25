# AgentSight: System-Level Observability for AI Agents Using eBPF

## Abstract

Modern AI agents present unique observability challenges due to their non-deterministic behavior, dynamic code generation capabilities, and complex interactions with external systems. Traditional application-level monitoring approaches fail to capture the full spectrum of agent behaviors, particularly when agents spawn subprocesses or modify their own execution paths. We present AgentSight, an eBPF-based observability framework that captures AI agent behavior at system boundaries—specifically at the network (TLS) and kernel (syscall) interfaces. Our approach enables framework-agnostic monitoring without requiring instrumentation of rapidly evolving agent codebases. AgentSight captures both high-level semantic information (LLM prompts and responses) and low-level system interactions (process spawning, file operations) with measured overhead below 3%. We discuss the technical architecture, implementation challenges, and propose this as a foundation for future research in AI agent observability. The system is available as open source to facilitate community collaboration and experimentation.

**Repository**: [https://github.com/eunomia-bpf/agentsight](https://github.com/eunomia-bpf/agentsight)

---

## 1. Introduction

The emergence of AI agents—autonomous systems that combine large language models (LLMs) with tool use and environmental interactions—introduces fundamental challenges for system observability. Unlike traditional software that follows predetermined execution paths, AI agents generate dynamic workflows, spawn arbitrary subprocesses, and modify their behavior based on learned patterns. This shift from static to adaptive software systems necessitates rethinking our observability approaches.

Current observability solutions for AI agents rely primarily on application-level instrumentation within agent frameworks (LangChain, AutoGen, Claude Code). However, this approach faces significant limitations. Agent frameworks evolve rapidly with frequent breaking changes to APIs and internal structures. Agents can execute arbitrary code through tool use, potentially bypassing or disabling their own monitoring. Cross-process interactions through subprocess spawning or network calls often escape framework-level observation.

We propose a different approach: observing AI agents at the system level using eBPF (extended Berkeley Packet Filter) technology. By intercepting interactions at kernel syscalls and TLS encryption boundaries, we can capture comprehensive agent behavior without modifying agent code. This system-level perspective provides stable observation points that remain consistent regardless of framework changes.

This paper presents AgentSight, an open-source implementation that demonstrates the feasibility of system-level AI agent observability. AgentSight captures both semantic information (LLM prompts and responses via TLS interception) and system behavior (process creation, file operations via syscall monitoring). We show how this dual perspective enables understanding agent behavior across abstraction levels—from high-level reasoning to low-level system interactions.

Our work aims to establish a foundation for the research community to explore AI agent observability challenges. We provide the implementation as open source to enable experimentation with different monitoring strategies, semantic analysis techniques, and integration approaches. The goal is not to present a complete solution but to demonstrate a promising direction and invite collaboration on this emerging problem space.

---

## 2. Background and Problem Statement

### 2.1 AI Agent Architecture

AI agents represent a new class of software systems that combine language models with environmental interactions. These systems typically consist of three core components: (1) an LLM backend that provides reasoning capabilities, (2) a tool execution framework that enables system interactions, and (3) a control loop that orchestrates prompts, tool calls, and state management. Popular frameworks such as LangChain, AutoGen, and Claude Code implement variations of this architecture.

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

**Semantic Telemetry**: Current observability tools lack primitives for capturing AI-specific behaviors. We need new telemetry formats that can represent prompt chains (`prompt.parent_id`, `prompt.temperature`), reasoning patterns (`reasoning.depth`, `reasoning.loop_count`), and semantic anomalies (`hallucination.score`, `persona.drift`). These metrics must bridge the gap between system-level observations and high-level agent behaviors.

**Causal Correlation**: Understanding agent behavior requires correlating events across multiple abstraction layers. A single agent action might involve an LLM API call, multiple file operations, subprocess spawning, and network requests. Current tools struggle to maintain causality relationships across these boundaries, especially when agents spawn independent processes.

**Cross-Process Visibility**: Agents routinely escape their parent process boundaries through subprocess execution. A Python agent might write a bash script, execute it, which then launches additional programs. Traditional process-scoped monitoring loses visibility at each boundary crossing. System-level observation becomes essential for maintaining comprehensive visibility.

In summary, AI agent observability demands treating agents as autonomous, potentially unreliable entities rather than deterministic software components. This perspective shift drives our exploration of system-level monitoring approaches that observe agent behavior at stable system boundaries rather than within rapidly evolving application code.

---

## 3. Related Work and Current Approaches

### 3.1 Application-Level Instrumentation in Agent Frameworks

Current approaches to AI agent observability predominantly rely on application-level instrumentation integrated within agent frameworks. These solutions typically implement one of three patterns: (1) callback-based hooks that intercept framework method calls, (2) middleware layers that wrap LLM API interactions, or (3) explicit logging statements embedded within agent logic.

While these approaches provide immediate visibility into agent operations, they face fundamental limitations when applied to autonomous AI systems. Agent frameworks undergo rapid iteration cycles—LangChain, for instance, has averaged multiple breaking changes per month throughout 2024. This instability forces continuous updates to instrumentation code. More critically, agents can dynamically modify their execution environment, loading new tools, rewriting prompts, or even generating code that bypasses instrumented pathways.

The most concerning limitation emerges from the trust model mismatch. Traditional instrumentation assumes the monitored application cooperates with observation efforts. However, AI agents can be influenced through prompt injection or emergent behaviors to disable logging, falsify telemetry, or execute operations through uninstrumented channels. Consider an agent that writes malicious commands to a shell script, then executes it through standard tool APIs—the file creation appears benign, while the subsequent execution escapes monitoring entirely.

### 3.2 Limitations of Current Approaches

Our analysis identifies three fundamental limitations in existing agent observability solutions:

**Instrumentation Fragility**: The rapid evolution of agent frameworks creates a moving target for instrumentation. Framework APIs change frequently, internal structures are refactored, and new capabilities are added continuously. More challenging still, agents themselves can modify their runtime environment—loading new libraries, generating helper functions, or creating novel tool implementations. This dynamic nature means instrumentation code requires constant maintenance to remain functional.

**Limited Scope of Visibility**: Application-level instrumentation captures only events within the instrumented process. When agents spawn subprocesses, make system calls, or interact with external services, these activities often escape observation. A Python-based agent executing shell commands through `subprocess.run()` leaves no trace in Python-level monitoring. Similarly, network requests made by child processes remain invisible to the parent's instrumentation.

**Semantic Gap**: Even when instrumentation successfully captures low-level operations, interpreting their meaning requires understanding the agent's high-level intent. Current tools struggle to correlate system activities (file writes, network requests) with agent reasoning (prompts, model responses). This semantic gap makes it difficult to distinguish between legitimate agent operations and potentially harmful behaviors.

### 3.3 Existing System-Level Monitoring Approaches

Several research efforts have explored system-level monitoring for security and performance analysis. Tools like Falco and Tracee use eBPF for runtime security monitoring, detecting anomalous system behaviors. However, these solutions focus on predefined security policies rather than understanding AI agent semantics.

The key insight from examining these approaches is that while system-level monitoring provides comprehensive visibility, existing tools lack the semantic understanding necessary for AI agent observability. They can detect that a process spawned a shell, but cannot correlate this with an agent's reasoning chain or determine whether the action aligns with the agent's stated goals.

---

## 4. Landscape of AI Agent Observability Solutions

### 4.1 Survey Methodology

To understand the current state of AI agent observability, we surveyed existing commercial and open-source solutions. Our analysis focused on tools that: (1) provide production-ready monitoring capabilities for LLM-based systems, (2) offer integration paths for popular agent frameworks, and (3) ship with trace collection and analysis features. We evaluated 12 representative solutions across multiple dimensions including integration approach, visibility scope, and architectural design.

### 4.2 Existing Solutions

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

### 4.3 Analysis of Current Approaches

Our survey reveals three dominant architectural patterns in existing solutions:

**SDK Instrumentation** (LangSmith, Langfuse, Traceloop): These tools require modifying agent code to add instrumentation hooks. While providing detailed visibility into framework operations, they suffer from tight coupling to rapidly evolving APIs. Version incompatibilities and breaking changes require constant maintenance.

**Proxy Interception** (Helicone, PromptLayer): Proxy-based solutions intercept HTTP traffic between agents and LLM providers. This approach avoids code modification but only captures LLM interactions, missing local tool usage, file operations, and subprocess activities.

**Standardization Efforts** (OpenTelemetry GenAI, OpenInference): Recent standardization initiatives define common schemas for AI observability data. While improving interoperability, these standards still rely on voluntary instrumentation and trust the agent to report accurately.

### 4.4 Critical Gaps

Our analysis identifies several critical gaps in current solutions:

**Lack of System-Level Visibility**: All surveyed tools operate within application boundaries. None capture system calls, subprocess creation, or network activities occurring outside the instrumented process. This limitation becomes critical when agents execute external commands or spawn helper processes.

**Assumption of Cooperative Behavior**: Existing tools assume agents will faithfully report their activities through instrumentation APIs. This assumption fails when agents are compromised, experience bugs, or intentionally bypass monitoring.

**Semantic Understanding**: While tools capture operational metrics (latency, token usage), they struggle to understand the semantic meaning of agent actions. Correlating low-level operations with high-level agent intentions remains an unsolved challenge.

**Cross-Process Correlation**: When agents spawn multiple processes or interact across system boundaries, maintaining causal relationships between events becomes difficult. Current tools lack mechanisms to track activity flows across process boundaries.

These gaps motivate our exploration of system-level monitoring approaches that observe agent behavior at kernel and network boundaries, providing comprehensive visibility regardless of agent cooperation or framework changes.

---

## 5. System-Level Observability Through Boundary Tracing

### 5.1 Core Concept

We propose *boundary tracing* as a novel approach to AI agent observability. The key insight is that all meaningful agent interactions must traverse well-defined system boundaries: the kernel interface for system operations and the network interface for external communications. By observing at these boundaries rather than within agent code, we achieve stable, comprehensive monitoring independent of agent implementation details.

Boundary tracing leverages the principle that while agent internals may change rapidly and unpredictably, the interfaces through which agents interact with their environment remain stable. System calls, network protocols, and file system operations provide consistent observation points that persist across framework versions and agent modifications.

### 5.2 System Architecture and Observation Points

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

### 5.3 Advantages of Boundary Tracing

Boundary tracing offers several key advantages over traditional instrumentation approaches:

**Framework Independence**: By observing at system interfaces rather than within application code, boundary tracing works identically across all agent frameworks. Whether an agent uses LangChain, AutoGen, or custom implementations, the system calls and network communications remain consistent.

**Semantic Completeness**: Network boundary observation captures full LLM interactions including prompts, model responses, and reasoning chains. Kernel boundary observation captures all system effects including file operations, process spawning, and network activities. Together, they provide complete visibility into both agent reasoning and actions.

**Stability Under Change**: System interfaces (POSIX syscalls, TLS protocols) evolve slowly compared to agent frameworks. A monitoring solution built on these interfaces remains functional despite rapid changes in agent implementations.

**Correlation Capability**: Events captured at both boundaries share common identifiers (process IDs, timestamps) enabling correlation between high-level reasoning (captured at network boundary) and low-level actions (captured at kernel boundary). This correlation reveals the causal chain from agent intent to system effect.

### 5.4 Technical Challenges

Implementing boundary tracing presents several technical challenges:

**TLS Decryption**: Capturing LLM communications requires intercepting TLS-encrypted traffic. We address this through eBPF uprobes on SSL library functions, capturing data after decryption within the application's address space.

**Event Correlation**: Associating network communications with subsequent system calls requires maintaining state across observation points. Process IDs, thread IDs, and temporal proximity provide correlation signals.

**Performance Overhead**: System-level monitoring must minimize impact on agent performance. eBPF's in-kernel execution and efficient data structures help achieve low overhead.

**Semantic Reconstruction**: Raw system events must be interpreted to understand agent behavior. This requires reconstructing higher-level operations from sequences of low-level events.

---

## 6. Technical Foundation: eBPF

### 6.1 What is eBPF?

eBPF (extended Berkeley Packet Filter) is a kernel technology that allows safe, efficient programs to run in kernel space¹⁰. Originally designed for packet filtering, it now powers advanced observability tools.

### 6.2 Key Capabilities for AI Observability

- **uprobes**: Hook userspace functions (e.g., SSL_write) to capture decrypted data¹¹
- **tracepoints**: Monitor system events like process creation¹²
- **Safety**: Programs are verified before execution, preventing kernel crashes¹³

### 6.3 Performance Characteristics

Recent benchmarks show¹⁴:
- CPU overhead: 1-3% for typical workloads
- Latency impact: <50μs per event
- Memory usage: Bounded by design

---

## 8. System Build

1. A zero-instrumentation observability tool for AI agent systems built entirely on **system-level tracing (eBPF)** to achieve unified semantic and operational visibility independent of the rapidly-evolving agent runtimes and frameworks.
2. A llm "sidecar" approach to detect subtle semantic anomalies (e.g., reasoning loops, contradictions, persona shifts) together with the system logs.

---

## 9. Challenges

However, implementing a system-level observability tool is not stra

The AI Agent is fundamental different from traditional software, it's more like a "user in the system" that can do anything. It can spawn subprocesses, use external tools, and even modify its own code. It can also be compromised by malicious prompts or self-modifying code.

One core challenge lies in the **semantic gap** between kernel-level signals and AI agent behaviors. While eBPF can capture comprehensive system-level data with minimal overhead (typically 2-3% CPU usage), translating this into meaningful insights about agent performance requires sophisticated correlation techniques.

Another challenge is capture all prompts and interactions witrh backend server is from encrypted TLS traffic. most llm serving are using TLS to communicate with backend server, and using SSE to stream the response. Using traditional network packet capture tools like tcpdump or wireshark is not enough, because the traffic is encrypted. Proxy the traffic can be a alternative solution, but proxy solutions require explicit configuration changes to route agent traffic through the proxy, which may not work with third party applications or frameworks and can introduce additional latency and complexity. Even if existing eBPF tools can capture the traffic, it lacks support for SSE stream API support.

By using eBPF uprobe to hook the TLS read and write in userspace, we can capture the traffic and decrypt it.

---

## 10. Open Challenges and Future Directions

### 10.1 Technical Challenges

1. **Semantic Understanding**: Bridging kernel events to high-level agent intent
2. **Privacy**: Balancing comprehensive monitoring with data protection
3. **Standardization**: Need for common semantic conventions

### 10.2 Research Opportunities

- Multi-agent correlation across distributed systems
- Real-time anomaly detection using behavioral patterns
- Privacy-preserving analysis techniques

### 10.3 The Path Forward

The AI agent ecosystem needs:
1. Industry standards for agent observability
2. Integration with existing security frameworks
3. Regulatory guidance on agent monitoring requirements

---

## 11. Conclusion

AI agents represent a fundamental shift in software - from predictable tools to autonomous entities. This shift demands equally fundamental changes in observability. Application-level instrumentation, while valuable, cannot provide the comprehensive monitoring these systems require.

Boundary tracing offers a solution: observe agents where they interact with the system, at boundaries they cannot forge. By leveraging eBPF technology, we can achieve framework-agnostic, zero-instrumentation monitoring with minimal overhead.

The stakes are high. As agents become more capable and widespread, the risks of unobserved misbehavior grow exponentially. We need observability infrastructure that treats agents as the semi-trusted, potentially compromised entities they can become.

AgentSight demonstrates this approach is not just theoretical but practical and performant. The question isn't whether we need better agent observability - it's how quickly we can deploy it.

**Get involved**: 
- Explore AgentSight: [github.com/eunomia-bpf/agentsight]
- Join the discussion: [Workshop/Conference details]
- Contribute: Standards development, tool integration, research

---

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
