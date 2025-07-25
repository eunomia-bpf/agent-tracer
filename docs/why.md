# AgentSight: Why We Need eBPF for AI Agent Observability

## TL;DR

AI agents are fundamentally different from traditional software: they exhibit emergent behaviors, make autonomous decisions, and can modify their own execution patterns. This creates critical observability gaps that existing tools cannot address.

**AgentSight introduces boundary tracing**: a framework-agnostic approach that observes AI agents at the system boundary using eBPF technology. By capturing SSL/TLS traffic and process behaviors at the kernel level, we achieve framework independence that works across LangChain, AutoGen, Claude Code, and emerging frameworks. The kernel-level tracing provides tamper resistance that compromised agents cannot evade, while maintaining full semantic visibility including prompt/response capture with streaming Server-Sent Events (SSE). Most importantly, this approach maintains less than 3% CPU overhead through efficient eBPF programs.

## The Problem at a Glance

Consider the security implications: A recent prompt-injection vulnerability in Meta AI's assistant exposed user conversations to unauthorized parties in January 2025[^1]. Traditional application-level monitoring would miss such attacks because compromised agents can disable their own logging. 

The financial stakes are significant. Security breaches cost organizations an average of $4.88 million per incident according to IBM's 2024 report[^2]. Meanwhile, the pace of change in AI frameworks compounds the challenge—LangChain alone shipped over 100 releases in 2024[^3], each potentially breaking existing instrumentation. These AI agents can spawn processes, modify code, and interact with systems in unpredictable ways that traditional monitoring simply cannot capture.

For security, consider a scenario where an LLM agent first writes a bash file with malicious commands (seemingly safe since it's just writing, not executing), and then executes it through basic tool calls that are often allowed. This attack pattern demonstrates why we need system-wide observability and constraints that go beyond application-level monitoring.

## From Deterministic Code to Autonomous Agents

The rise of AI-powered agentic systems is transforming modern software infrastructure. Frameworks like AutoGen, LangChain, Claude Code, and gemini-cli orchestrate large language models (LLMs) to automate software engineering tasks, data analysis pipelines, and multi-agent decision-making. Unlike traditional software components that produce deterministic, easily observable behaviors, these AI-agent systems generate open-ended, non-deterministic outputs, often conditioned on hidden internal states and emergent interactions between multiple agents. Consequently, debugging and monitoring agentic software pose unprecedented observability challenges that classic application performance monitoring (APM) tools cannot address adequately.

This new paradigm requires a fundamental shift in our approach to observability. We are moving from monitoring predictable, stateless services to overseeing dynamic, stateful entities that can learn, adapt, and evolve. The very definition of a failure has changed, expanding from simple crashes and errors to subtle semantic deviations like factual inaccuracies, logical loops, or undesirable emergent behaviors.

### How AI-agent observability differs from classic software observability

| Dimension | Traditional app / micro-service | LLM or multi-agent system |
| --- | --- | --- |
| **What you try to "see"** | Latency, errors, CPU, GC, SQL counts, request paths | *Semantics* — prompt / tool trace, reasoning steps, toxicity, hallucination rate, persona drift, token / money you spend |
| **Ground truth** | Deterministic spec: given X you must produce Y or an exception | Open-ended output: many "acceptable" Y's; quality judged by similarity, helpfulness, or policy compliance |
| **Failure modes** | Crashes, 5xx, memory leaks, deadlocks | Wrong facts, infinite reasoning loops, forgotten instructions, emergent mis-coordination between agents |
| **Time scale** | Millisecond spans; state usually dies at request end | Dialogue history and scratch memories can live for hours or days; "state" hides in vector DB rows and system prompts |
| **Signal source** | Structured logs and metrics you emit on purpose | Often *inside plain-text TLS payloads*; and tools exec logs |
| **Fix workflow** | Reproduce, attach debugger, patch code | Re-prompt, fine-tune, change tool wiring, tweak guardrails—code may be fine but "thought process" is wrong |
| **Safety / audit** | Trace shows what code ran | Need evidence of *why* the model said something for compliance / incident reviews |

This table underscores a crucial point: the semantics of AI agent behavior are the new frontier. While traditional APM tools are excellent at tracking the health of infrastructure, they are blind to the quality and safety of the agent's reasoning and interactions. This is not just a technical gap; it's a business risk.

These differences present significant research and engineering challenges. The **instrumentation gap** is a primary concern; as agent logic and algorithms evolve daily, relying on in-code hooks creates constant maintenance churn. A more stable approach, like kernel-side or side-car tracing, is needed. Furthermore, we require a new form of **semantic telemetry**, with attributes that capture the nuances of agent behavior (`model.temp`, `reasoning.loop_id`) and detectors for anomalies like persona drift. A key research challenge lies in **causal fusion**: merging low-level system events with high-level semantic spans into a unified timeline. This would empower developers to answer complex questions about agent behavior. Finally, **tamper resistance** is paramount. If a prompt injection turns an agent malicious, it may silence its own logs. Out-of-process, kernel-level tracing provides an essential, independent audit channel that cannot be easily compromised.

In short, AI-agent observability inherits the **unreliable, emergent behaviour** of AI Agents. Treat the agent runtime as a semi-trusted black box and observe at the system boundary: that's where the stability and opportunities lie.

## Observability Gap in Today's Tooling

Current agent observability techniques rely predominantly on application-level instrumentation—callbacks, middleware hooks, or explicit logging—integrated within each agent framework. While this approach seems intuitive, it suffers from fundamental limitations that make it unsuitable for production AI systems.

The first challenge is maintenance overhead. Agent frameworks evolve at a breakneck pace, constantly changing their prompts, tools, workflows, and memory interfaces. These systems can even modify their own code to create new tools and behaviors dynamically. Any instrumentation embedded within agent codebases becomes a moving target, requiring constant updates just to maintain basic visibility.

Security vulnerabilities present an even more serious concern. Agent runtimes can be tampered with or compromised through prompt injection attacks, allowing malicious actors or buggy behaviors to evade logging entirely. When the very system you're monitoring can turn against its own observability layer, application-level instrumentation becomes fundamentally unreliable.

Perhaps most critically, application-level instrumentation suffers from cross-boundary blindness. It cannot reliably capture cross-agent semantics such as reasoning loops, semantic contradictions, or persona shifts. When agent interactions cross process boundaries—spawning external tools, executing subprocesses, or communicating through system calls—traditional instrumentation loses the thread entirely.

### Current Landscape

Below is a quick landscape scan of LLM / AI‑agent observability tooling as of July 2025. I focused on offerings that (a) expose an SDK, proxy, or spec you can wire into an agent stack today and (b) ship some way to trace / evaluate / monitor model calls in production.

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

### What We Still Can't See


Our analysis of the current landscape reveals several key trends. The vast majority of existing solutions hook into the SDK layer, requiring developers to wrap or proxy function calls. While this approach is suitable for proof-of-concepts, it becomes brittle when agents dynamically change their behavior. On the positive side, OpenTelemetry is emerging as the de-facto wire format, simplifying backend integration. However, semantic evaluation is still in its early stages, with most tools focusing on latency and cost rather than the quality of the agent's output. Most importantly, none of the surveyed tools perform kernel-level capture; they all trust the application layer to be a reliable source of information. This leaves a significant blind spot for prompt-injection or self-modifying agents, a gap that a zero-instrumentation eBPF tracer is perfectly positioned to fill.

In summary, Current agent observability techniques rely predominantly on application-level instrumentation—callbacks, middleware hooks, or explicit logging—integrated within each agent framework. While intuitive, this approach suffers three fundamental limitations. First, agent frameworks evolve rapidly, changing prompts, tools, workflow and memory interfaces frequently. They can even modify their self code to create new tools, change prompts and behaviors. Thus, instrumentation embedded within agent codebases incurs significant maintenance overhead. Second, agent runtimes can be tampered with or compromised (e.g., via prompt injection), allowing attackers or buggy behaviors to evade logging entirely. Third, application-level instrumentation cannot reliably capture cross-agent semantics, such as reasoning loops, semantic contradictions, persona shifts, or the behaviors when it's interacting with its environment, especially when interactions cross process or binary boundaries (e.g., external tools or subprocesses).

Consider a concrete attack scenario: an LLM agent first writes a bash file with malicious commands (seemingly safe since it's just writing, not executing), then executes it through basic tool calls that are often allowed. This pattern exploits the gap between what application-level monitoring sees and what actually happens at the system level, demonstrating why we need system-wide observability and constraints.

### How This Motivates the "Boundary Tracing" Idea

Because today's solutions live mostly inside the agent process, they inherit the same fragility as the agent code. This leads to several critical blind spots: breakage when the prompt graph is tweaked, evasion by malicious prompts, and blindness to cross-process side effects like writing and executing a shell script.

A system-level eBPF tracer that scoops TLS write buffers and syscalls sidesteps these issues, providing a more robust and tamper-proof view of the agent's behavior. For example, where an SDK-based tool would miss an agent spawning `curl` directly, a boundary tracer would see the `execve("curl", ...)` syscall and the subsequent network write. Similarly, if an agent mutates its own prompt string before logging, a boundary tracer would capture the raw ciphertext leaving the TLS socket.

In other words, existing tools solve the "what happened inside my code?" story; kernel-side tracing can answer "what actually hit the wire and the OS?"—a complementary, harder-to-tamper vantage point.

That gap is wide open for research and open‑source innovation.

## Boundary Tracing: Core Idea

All meaningful interactions of an AI agent system traverse two clear boundaries: the network and the kernel. This leads to our key insight:

> AI agent observability must be decoupled from agent internals. **Observing from the boundary provides a stable, trustworthy, and semantically rich interface.**

### AI Agent Architecture

An agent-centric stack as three nested circles:

```
┌───────────────────────────────────────────────┐
│          ☁  Rest of workspace / system        │
│  (APIs, DBs, message bus, OS, Kubernetes…)    │
│                                               │
│   ┌───────────────────────────────────────┐   │
│   │       Agent runtime / framework       │   │
│   │ (LangChain, claude-code, gemini-cli …)│   │
│   │  • orchestrates prompts & tool calls  │   │
│   │  • owns scratch memory / vector DB    │   │
│   └───────────────────────────────────────┘   │
│               ↑ outbound API calls            │
│───────────────────────────────────────────────│
│               ↓ inbound events                │
│   ┌───────────────────────────────────────┐   │
│   │          LLM serving provider         │   │
│   │    (OpenAI endpoint, local llama.cpp) │   │
│   └───────────────────────────────────────┘   │
└───────────────────────────────────────────────┘
```

* **LLM serving provider** – token generation, non-deterministic reasoning, chain-of-thought text that may or may not be surfaced. Most system work are around the LLM serving layer.
* **Agent runtime layer** – turns tasks into a sequence of LLM calls plus external tool invocations; stores transient "memories".
* **Outside world** – OS, containers, other services.

For **observability purposes** the clean interface is usually the *network boundary* (TLS write of a JSON inference request) and the system boundary (syscall / subprocess when the agent hits commands `curl`, `grep`). Anything below those lines (GPU kernels, weight matrices, models) is model-inference serving territory; anything above is classic system observability tasks. That's why kernel-level eBPF can give you a neutral vantage: it straddles both worlds without needing library hooks.

### Why Boundary Beats SDK

By shifting observability to the system-level boundary, we fundamentally change the game. This approach achieves framework neutrality, working seamlessly across all agent runtimes—LangChain, AutoGen, gemini-cli—without any modifications. The semantic stability comes from capturing prompt-level interactions at the point where they must pass through the OS, regardless of how the framework implements them internally.

Most importantly, boundary tracing provides trust and auditability through an independent observation layer that cannot be compromised by in-agent malware. It creates a universal causal graph that merges agent-level semantics with OS-level events, giving developers the complete picture of what their agents actually do, not just what they claim to do. And unlike SDK-based solutions, this approach requires zero maintenance—no updates needed when frameworks change their APIs or internal structures.

## Why eBPF Fits the Job

Traditional software observability is instrumentation-first (you insert logs, spans, and metrics into the code you write). But AI agents change their internal logic dynamically through prompts, instructions, reasoning paths, and spontaneous tool usage. This constant internal mutability means instrumentation is fragile.

### Technical Foundation: eBPF for TLS Interception

The core technical challenge lies in capturing all prompts and interactions from encrypted TLS traffic. Most LLM serving infrastructure uses TLS for communication and Server-Sent Events (SSE) for streaming responses. Traditional network packet capture tools like tcpdump or wireshark cannot decrypt this traffic. Proxy solutions require explicit configuration changes that may not work with third-party applications and introduce additional latency and complexity.

eBPF provides an elegant solution through uprobes that hook TLS read and write functions in userspace[^10]. This approach captures the traffic transparently by intercepting plaintext before encryption and after decryption. The technique works universally with any TLS library—OpenSSL, BoringSSL, GnuTLS—through CO-RE (Compile Once - Run Everywhere) technology. It handles streaming protocols like SSE without buffering issues and requires zero application changes or proxy configuration.

Recent implementations demonstrate the maturity of this approach. Keploy's work[^11] shows production-ready eBPF-based TLS tracing, while Pixie Labs[^12] has deployed similar technology at scale. The eunomia.dev tutorial[^13] provides detailed implementation guidance for those building SSL/TLS capture systems using eBPF.

## AgentSight: Architecture & Build

AgentSight implements a zero-instrumentation observability tool for AI agent systems built entirely on system-level tracing (eBPF) to achieve unified semantic and operational visibility independent of the rapidly-evolving agent runtimes and frameworks.

### System Architecture

The architecture centers on eBPF programs that provide the foundational data collection layer. The program captures SSL/TLS traffic using uprobe hooks, and monitors process lifecycle events and file operations. These kernel-level collectors feed into a Rust-based streaming analysis framework that processes events in real-time through a pipeline of pluggable analyzers. The framework includes specialized support for SSE stream reassembly and HTTP parsing, critical for understanding LLM communications.

Above this data layer sits a semantic analysis engine that applies AI-specific intelligence to the raw events. Using an LLM "sidecar" approach, it detects subtle anomalies like reasoning loops, contradictions, and persona shifts that would be invisible to traditional monitoring. The visualization layer provides a Next.js web interface for timeline-based exploration, allowing teams to understand agent behavior through intuitive, real-time displays.

### Implementation Details

AgentSight's implementation leverages eBPF uprobes to intercept TLS library functions at their most fundamental level. The system hooks directly into SSL_write and SSL_read functions for OpenSSL and equivalent functions in other TLS libraries. This positioning allows it to capture plaintext data at the perfect moment—after decryption on reads and before encryption on writes. Each captured event is correlated with process information for attribution and streamed as JSON for real-time processing.

The deployment architecture emphasizes operational simplicity. Binary embedding enables single-file deployment without complex dependencies. The system automatically manages kernel resource cleanup, preventing resource leaks even during abnormal termination. Operators can configure filtering by process name, port, or content patterns, focusing observation on specific agents or behaviors. Built-in log rotation and compression handle the high-volume data streams that agent monitoring generates.

## Limitations & Open Challenges

The AI Agent is fundamentally different from traditional software, it's more like a "user in the system" that can do anything. It can spawn subprocesses, use external tools, and even modify its own code. It can also be compromised by malicious prompts or self-modifying code.

One core challenge lies in the **semantic gap** between kernel-level signals and AI agent behaviors. While eBPF can capture comprehensive system-level data with minimal overhead (typically 2-3% CPU usage), translating this into meaningful insights about agent performance requires sophisticated correlation techniques. The challenge extends beyond mere data collection—we must bridge the conceptual distance between low-level system events and high-level agent intentions.

### Technical Limitations

Several technical boundaries constrain the current implementation. TLS capture becomes complex when dealing with statically linked Go binaries that use crypto/tls—these require USDT hooks to be enabled for visibility. The framework coverage remains focused on HTTP/TLS-speaking agents, while systems using gRPC pipes or Unix domain sockets require additional hook development.

Tamper resistance, while strong, isn't absolute. Kernel-level tracing is significantly harder to bypass than application-level monitoring, but determined attackers could still use container escape techniques or LD_PRELOAD tricks to hide activities. The semantic gap presents perhaps the most fundamental challenge: while eBPF captures comprehensive system-level data with minimal overhead (typically 2-3% CPU usage), translating these low-level signals into meaningful insights about agent behavior requires sophisticated correlation and analysis techniques.

### Research Opportunities

The intersection of eBPF and AI observability opens fascinating research directions. Semantic anomaly detection stands out as a prime opportunity—using LLMs to analyze captured agent conversations for reasoning loops, contradictions, or policy violations. This creates a recursive observation problem: using AI to monitor AI, with all the philosophical and practical implications that entails.

Cross-agent correlation presents another rich area for investigation. Building causal graphs that connect multiple agents' activities across process boundaries could reveal emergent behaviors invisible at the individual agent level. Performance optimization remains crucial—pushing overhead below the current 3% threshold while maintaining full semantic capture would enable deployment in even the most performance-sensitive environments.

Privacy-preserving analysis techniques could enable monitoring of agent behavior without exposing sensitive prompt content, addressing a critical concern for enterprise deployments. Finally, hardware acceleration through DPUs and SmartNICs could enable line-rate processing of agent traffic, removing CPU overhead entirely from the host system.

## Conclusion

AI agents represent a paradigm shift in software architecture that breaks traditional observability assumptions. AgentSight's boundary tracing approach, implemented through eBPF technology, provides a stable, tamper-resistant foundation for understanding agent behavior. By observing at the system boundary rather than within rapidly-evolving agent frameworks, we achieve both technical stability and semantic richness.

The open-source AgentSight implementation demonstrates the feasibility of this approach with less than 3% overhead. As AI agents become critical infrastructure, boundary-based observability will be essential for security, reliability, and trust.

## References

1. Meta AI prompt-exposure incident, January 2025. [Tom's Guide](https://www.tomsguide.com/computing/online-security/meta-ai-was-leaking-chatbot-prompts-and-answers-to-unauthorized-users)
2. IBM. "Cost of a Data Breach Report 2024." [IBM](https://www.ibm.com/think/insights/cost-of-a-data-breach-2024-financial-industry)
3. LangChain GitHub releases page, 2024. [GitHub](https://github.com/langchain-ai/langchain/releases)
4. eBPF uprobe documentation. [kernel.org](https://www.kernel.org/doc/html/latest/trace/uprobetracer.html)
5. Keploy. "eBPF for TLS Traffic Tracing: Secure & Efficient Observability," January 2025. [Keploy](https://keploy.io/blog/community/ebpf-for-tls-traffic-tracing-secure-efficient-observability)
6. Pixie Labs. "eBPF TLS Tracing: Past, Present & Future," September 2024. [blog.px.dev](https://blog.px.dev/ebpf-tls-tracing-past-present-future/)
7. Eunomia. "eBPF Practical Tutorial: Capturing SSL/TLS Plain Text Data," 2025. [eunomia.dev](https://eunomia.dev/en/tutorials/30-sslsniff/)
8. OWASP GenAI Security Project. "LLM01:2025 Prompt Injection," 2025. [OWASP](https://genai.owasp.org/llmrisk/llm01-prompt-injection/)
9. LangSmith Documentation. "Observability Quick Start." [LangSmith](https://docs.smith.langchain.com/observability)
10. Helicone. "LLM-Observability for Developers." [Helicone](https://www.helicone.ai/)
11. Traceloop. "LLM Reliability Platform." [Traceloop](https://www.traceloop.com/)
12. Arize Phoenix. "AI Observability & Evaluation." [Phoenix](https://phoenix.arize.com/)
13. Langfuse. "LLM Observability & Application Tracing." [Langfuse](https://langfuse.com/)
14. WhyLabs. "Large Language Model Monitoring." [WhyLabs](https://whylabs.ai/langkit)
15. PromptLayer. "Complete AI Observability." [PromptLayer](https://www.promptlayer.com/platform/observability)
16. Literal AI. "RAG LLM observability and evaluation platform." [Literal AI](https://www.literalai.com/)
17. Weights & Biases. "Enterprise-Level LLMOps: W&B Traces." [W&B](https://wandb.ai/site/traces/)
18. Honeycomb. "Observability for AI & LLMs." [Honeycomb](https://www.honeycomb.io/ai-llm-observability)
19. OpenTelemetry. "Semantic conventions for generative AI systems." [OpenTelemetry](https://opentelemetry.io/docs/specs/semconv/gen-ai/)
20. OpenInference. "OpenTelemetry Instrumentation for LLMs." [GitHub](https://github.com/Arize-ai/openinference)