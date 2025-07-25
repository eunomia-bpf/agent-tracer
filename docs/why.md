# AgentSight: Why We Need eBPF for AI Agent Observability

## Abstract

AI agents introduce novel observability challenges due to their emergent behaviors, autonomous decision-making, and dynamic execution, making traditional monitoring tools fall short. The primary challenge lies in achieving the right data collection granularity—capturing granular SSL/TLS traffic and process behaviors without overwhelming system resources (typically maintaining less than 3% CPU usage). The second critical challenge is framework-neutral system event correlation—achieving framework independence while correlating low-level system activities with high-level agent interactions like prompts and tool calls. AgentSight proposes boundary tracing as the solution: a framework-agnostic approach that observes AI agents at the system boundary using eBPF technology. By operating at the kernel level, this approach provides critical insights into system events and enables the correlation of diverse data points, offering a robust foundation for understanding complex AI agent behavior across rapidly evolving frameworks.

## The Problem: The Observability Gap in AI Agents

AI agents represent a fundamental shift from traditional software. Unlike deterministic code, agents exhibit emergent behaviors, make autonomous decisions, and can dynamically modify their own execution paths. This introduces significant observability challenges that conventional Application Performance Monitoring (APM) tools are ill-equipped to handle. At the heart of these challenges lie two critical problems that must be solved simultaneously.

The first challenge is achieving the right data collection granularity. AI agents generate massive amounts of interaction data through their SSL/TLS communications with LLM providers, tool invocations, and system interactions. Capturing sufficient detail to understand agent behavior—including full prompts, responses, reasoning chains, and tool calls—can easily overwhelm system resources. Traditional approaches that capture everything quickly become impractical, with CPU usage spiking to 20% or more in production environments. Yet capturing too little data leaves blind spots that prevent effective debugging and monitoring. The challenge is finding the sweet spot: capturing just enough granular data to enable meaningful analysis while maintaining acceptable performance overhead, ideally below 3% CPU usage.

The second fundamental challenge is framework-neutral system event correlation. AI agents operate across multiple abstraction layers—from high-level framework APIs down to system calls and network communications. Modern agent frameworks like LangChain, AutoGen, and gemini-cli evolve rapidly, with frequent API changes and architectural updates. Building observability directly into these frameworks creates a maintenance nightmare and locks monitoring to specific implementations. Meanwhile, critical insights require correlating low-level system events (process spawning, file operations, network calls) with high-level agent semantics (prompts, tool invocations, reasoning steps). The challenge is developing an approach that remains independent of any specific framework while still providing the correlation capabilities needed to understand agent behavior holistically.

Traditional APM excels at monitoring predictable, stateless services, but AI agents are dynamic, stateful entities that learn, adapt, and evolve. The definition of 'failure' expands beyond crashes to include subtle semantic deviations like factual inaccuracies, logical loops, or unintended emergent behaviors. Consider the practical implications: The rapid evolution of AI frameworks, exemplified by LangChain's numerous updates[^3], complicates existing instrumentation efforts, leading to a constant maintenance burden. Furthermore, AI agents can initiate processes, modify code, and interact with systems in unpredictable ways that traditional monitoring solutions may not fully capture. This lack of comprehensive visibility can have substantial financial implications, with data breaches costing organizations millions[^2], and security vulnerabilities, such as prompt-injection attacks[^1], potentially exposing sensitive data if compromised agents disable their own logging. From a security perspective, consider a scenario where an LLM agent initially writes a bash file containing potentially malicious commands. While merely writing the file might appear benign, the agent could then execute it using commonly permitted tool calls. This attack vector highlights the necessity for system-wide observability and robust constraints that extend beyond typical application-level monitoring.

## From Deterministic Code to Autonomous Agents

The emergence of AI-powered agentic systems fundamentally reshapes modern software infrastructure. Frameworks like AutoGen, LangChain, and gemini-cli are increasingly used to orchestrate large language models (LLMs) for automating tasks in software engineering, data analysis, and multi-agent decision-making. Unlike traditional software components, which typically produce deterministic and easily observable behaviors, these AI-agent systems often generate open-ended, non-deterministic outputs. These outputs are frequently influenced by hidden internal states and complex interactions among multiple agents. This shift dramatically amplifies both the data collection granularity challenge and the framework-neutral correlation challenge.

The data collection challenge becomes acute because agents generate exponentially more meaningful interaction data than traditional software. Each agent conversation involves multiple prompts, reasoning chains, tool invocations, and responses—all transmitted through encrypted TLS connections. A single agent session can produce megabytes of interaction data within minutes. Capturing all this data naively would consume prohibitive amounts of CPU and storage, yet missing critical interactions could mean failing to detect harmful behaviors or debug complex failures. The challenge intensifies when multiple agents interact, as their combined data streams can easily overwhelm traditional monitoring infrastructure designed for simpler request-response patterns.

Simultaneously, the framework-neutral correlation challenge emerges from the architectural complexity of modern agent systems. These systems operate across multiple abstraction layers: high-level framework orchestration, mid-level tool invocations, and low-level system interactions. Each framework implements its own abstractions and APIs, which change frequently as the field evolves rapidly. Yet understanding agent behavior requires correlating events across all these layers—connecting a high-level prompt with the system calls it triggers, or linking a tool invocation with the network traffic it generates. Building this correlation capability into each framework would require constant maintenance and limit portability across different agent implementations.

This new paradigm necessitates a fundamental re-evaluation of our observability strategies. We are transitioning from monitoring predictable, stateless services to overseeing dynamic, stateful entities capable of learning, adapting, and evolving. The concept of 'failure' itself has broadened, now encompassing not only crashes and errors but also subtle semantic deviations such as factual inaccuracies, logical loops, or unintended emergent behaviors. These semantic failures often manifest through patterns that span multiple interactions and system boundaries, making them impossible to detect without comprehensive data collection and sophisticated correlation capabilities.


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

This table highlights how the two fundamental challenges permeate every aspect of AI agent observability. The data collection granularity challenge is evident in the shift from structured logs to unstructured TLS payloads—capturing and processing these high-volume, encrypted streams without overwhelming system resources requires careful engineering. Traditional APM tools can afford to capture every metric and log line, but agent systems generate orders of magnitude more semantic data through their conversations and reasoning chains. The framework-neutral correlation challenge manifests in the need to connect signals across vastly different abstraction layers—from plain-text reasoning within TLS streams to system-level tool executions—without depending on any specific agent framework's internal structure.

These differences crystallize into concrete engineering challenges. The **instrumentation gap** directly stems from the framework-neutral correlation challenge: as agent logic and algorithms evolve rapidly, relying on in-code hooks leads to constant maintenance overhead and framework lock-in. The solution requires a more stable observation point, such as kernel-side tracing, that remains consistent regardless of framework changes. The **semantic telemetry** challenge emerges from the data collection granularity problem: we need to capture rich attributes that reveal agent behavior (e.g., `model.temp`, `reasoning.loop_id`) while filtering out noise to maintain manageable data volumes. Most critically, **causal fusion**—merging low-level system events with high-level semantic spans into a unified timeline—represents the intersection of both challenges. It requires collecting sufficient granular data from multiple sources while maintaining the correlation capability to connect these disparate signals without framework-specific knowledge.

The data volume challenge is particularly acute in production environments. A single agent conversation can generate megabytes of TLS traffic containing prompts, responses, and reasoning chains. Multiply this by hundreds or thousands of concurrent agents, and traditional approaches that capture everything become untenable. Yet aggressive filtering risks missing critical behaviors—a malicious prompt injection might occupy just a few kilobytes within gigabytes of normal traffic. The engineering challenge lies in developing intelligent capture strategies that preserve essential semantic information while maintaining sub-3% CPU overhead.

In essence, AI-agent observability must solve these twin challenges simultaneously. The approach must be framework-agnostic to avoid constant maintenance as agent technologies evolve, while also being intelligent about data collection to capture meaningful signals without overwhelming system resources. Treating the agent runtime as a semi-trusted black box and observing its interactions at the system boundary offers a path forward that addresses both challenges through a unified architectural approach.

## Observability Gap in Today's Tooling

Current agent observability techniques predominantly rely on application-level instrumentation—callbacks, middleware hooks, or explicit logging—integrated within each agent framework. While seemingly intuitive, this approach fundamentally fails to address either the data collection granularity challenge or the framework-neutral correlation challenge, rendering it unsuitable for robust production AI systems.

The data collection granularity problem manifests severely in current tools. Application-level instrumentation typically captures data at points the framework designers deemed important, missing crucial details that emerge in production. These tools often capture either too much data—logging every function call and generating overwhelming noise—or too little, missing the actual content of agent conversations encrypted in TLS streams. Most SDK-based solutions lack intelligent filtering capabilities, leading to a stark choice: accept 15-20% CPU overhead from comprehensive logging, or miss critical agent behaviors by sampling too aggressively. The problem compounds when agents make rapid-fire API calls or engage in lengthy reasoning chains, where naive instrumentation can degrade agent performance to the point of impacting user experience.

The framework-neutral correlation challenge proves equally problematic for existing solutions. Each observability tool typically supports specific frameworks through custom integrations—LangSmith for LangChain, framework-specific SDKs for AutoGen or CrewAI. This creates multiple critical issues: teams using multiple agent frameworks need different observability stacks for each, making unified monitoring impossible; when frameworks update their APIs (which happens frequently in this rapidly evolving field), observability breaks until the integration is updated; and most importantly, these tools cannot correlate high-level agent behaviors with system-level events because they operate entirely within the application layer. When an agent spawns a subprocess or makes a system call, application-level instrumentation loses visibility entirely.

Perhaps most critically, the intersection of these challenges creates compound problems. Application-level instrumentation suffers from cross-boundary blindness—it cannot track agent interactions that span process boundaries, such as when an agent writes a script and then executes it. The maintenance overhead becomes overwhelming as teams must constantly update instrumentation for each framework change while trying to manage the performance impact of comprehensive data collection. These systems can even dynamically modify their own code to create new tools and behaviors, causing instrumentation to miss newly created execution paths. This lack of comprehensive, system-wide insight coupled with prohibitive resource consumption makes current approaches fundamentally inadequate for production agent monitoring.

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

Our analysis of the current landscape reveals a systematic failure to address the two fundamental challenges. Every tool suffers from severe limitations in data collection granularity. SDK-based solutions like LangSmith and Traceloop capture data at application-defined points, missing the actual content of encrypted TLS conversations between agents and LLM providers. Proxy-based approaches like Helicone can capture HTTP traffic but introduce latency and still miss system-level events. Most critically, none of these tools provide intelligent filtering to manage data volume—they either capture everything (leading to 15-20% overhead) or rely on crude sampling that misses important behaviors. The few tools that attempt comprehensive capture, like WhyLabs LangKit with its "HEAVY text-quality metrics," explicitly acknowledge the performance impact, making them unsuitable for production use at scale.

The framework-neutral correlation challenge is equally unaddressed. The landscape is fragmented by framework-specific tools: LangSmith for LangChain, framework-specific SDKs for others. This fragmentation means teams using multiple agent frameworks need entirely separate observability stacks, making unified monitoring impossible. More fundamentally, all these tools operate at the application layer, creating an insurmountable barrier to correlating high-level agent behaviors with system events. When an agent spawns a subprocess, writes a file, or makes a system call, these application-level tools are completely blind. They cannot answer critical questions like "what system resources did this prompt ultimately access?" or "which files were created as a result of this reasoning chain?"

The compound effect of these limitations is devastating for production deployments. Not a single tool in our survey can efficiently capture comprehensive agent behavior (maintaining <3% overhead) while providing framework-agnostic correlation between prompts and system events. OpenTelemetry's emergence as a data transmission standard is positive but doesn't solve the fundamental collection and correlation challenges. Most tools prioritize easily measurable metrics like latency and token costs while remaining blind to semantic behaviors and system interactions. Crucially, none perform kernel-level capture, leaving them vulnerable to evasion by compromised or self-modifying agents.

In summary, current agent observability techniques fail on both critical dimensions. They cannot manage data collection efficiently enough for production use, forcing untenable trade-offs between visibility and performance. They cannot provide framework-neutral correlation, locking teams into specific ecosystems and blinding them to system-level effects. Consider a concrete attack scenario: an LLM agent first writes a bash file with malicious commands (which might appear safe as it's only writing, not executing), and then executes it through basic tool calls. Current tools would miss this entirely—they might log the "write file" API call but remain blind to the actual file contents and subsequent execution at the system level. This gap between application-level monitoring and system reality underscores why a fundamentally different approach is needed.

### How This Motivates the "Boundary Tracing" Idea

The failure of current solutions to address either the data collection granularity challenge or the framework-neutral correlation challenge motivates a fundamentally different approach: observing agents at the system boundary rather than within their application code. This boundary tracing approach elegantly addresses both challenges simultaneously.

For the data collection granularity challenge, boundary tracing offers unprecedented efficiency. By intercepting data at the kernel level where TLS encryption/decryption occurs, we can capture the complete content of agent conversations without the overhead of application-level instrumentation. eBPF's efficient in-kernel filtering allows us to intelligently select which data to capture and process, maintaining the crucial <3% CPU overhead target even with comprehensive monitoring. Instead of instrumenting every function call and generating massive logs, boundary tracing captures exactly what matters: the actual prompts, responses, and system interactions that define agent behavior. The kernel's view provides natural data reduction—we see the final TLS writes, not every internal state change leading to them.

For the framework-neutral correlation challenge, boundary tracing provides a universal observation point that works identically regardless of agent framework. Whether an agent is built with LangChain, AutoGen, or a custom framework, they all must interact with the operating system through the same kernel interfaces. When they communicate with LLM providers, that traffic passes through TLS functions we can intercept. When they spawn processes or access files, those system calls are visible to eBPF. This creates a stable correlation layer: we can connect a prompt sent via TLS with the subprocess it triggers via execve(), or link an API response with the files it causes the agent to write. The correlation happens at the system level, independent of any framework's internal architecture.

The power of boundary tracing becomes clear through concrete examples. While an SDK-based tool might miss an agent directly spawning `curl`, a boundary tracer observes the `execve("curl", ...)` syscall and correlates it with the preceding prompt that triggered this action. When an agent modifies its own code or creates new tools dynamically, application instrumentation becomes useless, but boundary tracing continues to capture all system interactions. If an agent attempts to hide its activities by disabling logging, the kernel-level observer remains unaffected, capturing the raw TLS traffic and system calls regardless of application-level evasion attempts.

In essence, boundary tracing transforms the observability problem from "instrument every possible framework" to "observe at the universal system interface." This not only solves both fundamental challenges but does so with a stability that application-level approaches cannot match. As agent frameworks continue their rapid evolution, the system boundary remains constant, providing a solid foundation for production-grade observability.

## Boundary Tracing: Core Idea

All significant interactions within an AI agent system inherently cross two fundamental boundaries: the network and the operating system kernel. This observation leads to our core insight that directly addresses both fundamental challenges:

> AI agent observability must be decoupled from agent internals. **Observing from the boundary provides efficient data collection and framework-neutral correlation through a stable, universal interface.**

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

*   **LLM serving provider** – This layer handles token generation, non-deterministic reasoning, and chain-of-thought text, which may or may not be explicitly surfaced. Most system-level work is concentrated around the LLM serving layer.
*   **Agent runtime layer** – This layer orchestrates tasks by sequencing LLM calls and external tool invocations. It also manages transient "memories" for the agent.
*   **Outside world** – This encompasses the operating system, containers, and other external services.

This architecture reveals why boundary observation uniquely solves both fundamental challenges. For data collection granularity, the boundaries act as natural aggregation points—all the complex internal processing within an agent ultimately manifests as TLS communications (prompts and responses) and system calls (tool executions). Rather than tracking every internal state change, we capture the meaningful outputs at these boundaries, achieving comprehensive visibility with minimal overhead. The boundaries provide built-in data reduction: instead of logging every function call inside LangChain, we capture the final prompt sent to the LLM provider and the system resources it accesses.

For framework-neutral correlation, these boundaries serve as universal interfaces that remain constant across all agent implementations. Every agent framework—whether LangChain, AutoGen, or custom implementations—must cross these same boundaries. They all send prompts through TLS to communicate with LLMs. They all use system calls to spawn processes, read files, or make network connections. This creates a stable correlation layer: a TLS write containing a prompt can be definitively linked to subsequent system calls it triggers, regardless of the framework's internal architecture. The network boundary (TLS) captures high-level semantics while the system boundary (syscalls) captures low-level effects, and eBPF provides the mechanism to correlate them.

For **observability purposes**, the most effective interface is precisely these boundaries. The network boundary captures semantic information (e.g., a TLS write of a JSON inference request containing prompts and responses), while the system boundary captures operational effects (e.g., a syscall when the agent invokes commands like `curl` or `grep`). Anything below these layers (such as GPU kernels or model weights) falls within the domain of model serving. Conversely, anything above relates to classic system observability. This is why kernel-level eBPF offers the ideal vantage point: it efficiently observes both boundaries, bridging high-level agent semantics with low-level system operations without requiring any framework-specific instrumentation.

### Why Boundary Beats SDK

By shifting observability to the system-level boundary, we fundamentally alter the approach to monitoring. This method ensures framework neutrality, operating seamlessly across various agent runtimes—including LangChain, AutoGen, and gemini-cli—without requiring any modifications to the frameworks themselves. The semantic stability is derived from capturing prompt-level interactions at the point where they must interface with the operating system, irrespective of the framework's internal implementation.

Crucially, boundary tracing facilitates the creation of a unified causal graph, merging agent-level semantics with OS-level events. This provides developers with a comprehensive understanding of their agents' actual behavior, rather than just their reported actions. Furthermore, unlike SDK-based solutions, this approach requires minimal maintenance, as it remains unaffected by changes in framework APIs or internal structures. It also offers an independent observation layer, which is inherently more resilient to in-agent compromises.

## Why eBPF Fits the Job

Traditional software observability typically relies on an instrumentation-first approach, where logs, spans, and metrics are explicitly inserted into the code. However, AI agents dynamically alter their internal logic through prompts, instructions, reasoning paths, and spontaneous tool usage. This constant internal mutability renders traditional instrumentation methods fragile.

### Technical Foundation: eBPF for TLS Interception

The primary technical challenge involves capturing all prompts and interactions from encrypted TLS traffic. Most LLM serving infrastructure utilizes TLS for secure communication and Server-Sent Events (SSE) for streaming responses. Traditional network packet capture tools, such as tcpdump or Wireshark, are unable to decrypt this traffic. Proxy solutions, while an option, necessitate explicit configuration changes that may not be compatible with third-party applications and can introduce additional latency and complexity.

eBPF offers an elegant solution through uprobes that hook into TLS read and write functions in userspace[^10]. This approach transparently captures traffic by intercepting plaintext data before encryption and after decryption. The technique works universally with various TLS libraries—OpenSSL, BoringSSL, GnuTLS—leveraging CO-RE (Compile Once - Run Everywhere) technology. It effectively handles streaming protocols like SSE without buffering issues and requires no application changes or proxy configurations.

Recent implementations underscore the maturity of this approach. Keploy's work[^11] showcases production-ready eBPF-based TLS tracing, while Pixie Labs[^12] has successfully deployed similar technology at scale. The eunomia.dev tutorial[^13] provides detailed implementation guidance for developing SSL/TLS capture systems using eBPF.

## AgentSight: Architecture & Implementation

AgentSight is designed as a zero-instrumentation observability tool for AI agent systems. It leverages system-level tracing (eBPF) to provide unified semantic and operational visibility, independent of the rapidly evolving agent runtimes and frameworks.

### System Architecture

The architecture of AgentSight is centered on eBPF programs, which form the foundational data collection layer. These programs capture SSL/TLS traffic using uprobe hooks and monitor process lifecycle events and file operations. The kernel-level collectors feed this data into a Rust-based streaming analysis framework. This framework processes events in real-time through a pipeline of pluggable analyzers, including specialized support for SSE stream reassembly and HTTP parsing, which are crucial for understanding LLM communications.

Above this data layer, a semantic analysis engine applies AI-specific intelligence to the raw events. Utilizing an LLM "sidecar" approach, it identifies subtle anomalies such as reasoning loops, contradictions, and persona shifts that would otherwise be undetectable by traditional monitoring. The visualization layer, built with a Next.js web interface, enables timeline-based exploration, providing teams with intuitive, real-time insights into agent behavior.

### Implementation Details

AgentSight's implementation leverages eBPF uprobes to intercept TLS library functions at their most fundamental level. The system directly hooks into `SSL_write` and `SSL_read` functions for OpenSSL, and equivalent functions in other TLS libraries. This strategic positioning allows it to capture plaintext data at the optimal moment—after decryption on reads and before encryption on writes. Each captured event is correlated with process information for accurate attribution and streamed as JSON for real-time processing.

The deployment architecture emphasizes operational simplicity. Binary embedding enables single-file deployment without complex dependencies. The system automatically manages kernel resource cleanup, preventing resource leaks even during abnormal termination. Operators can configure filtering by process name, port, or content patterns, focusing observation on specific agents or behaviors. Built-in log rotation and compression handle the high-volume data streams that agent monitoring generates.

## Limitations & Open Challenges

AI agents fundamentally differ from traditional software, often acting more like a "user within the system" with broad capabilities. They can spawn subprocesses, utilize external tools, and even modify their own code. This flexibility also means they can be susceptible to compromise through malicious prompts or self-modifying code.

One significant challenge lies in bridging the **semantic gap** between kernel-level signals and AI agent behaviors. While eBPF can capture comprehensive system-level data with minimal overhead (typically 2-3% CPU usage), translating this raw data into meaningful insights about agent performance necessitates sophisticated correlation techniques. The challenge extends beyond mere data collection; it requires bridging the conceptual distance between low-level system events and high-level agent intentions.

### Technical Limitations

Several technical constraints impact the current implementation. TLS capture can be complex when dealing with statically linked Go binaries that utilize `crypto/tls`; these typically require USDT hooks for proper visibility. Furthermore, the current framework coverage primarily focuses on HTTP/TLS-speaking agents, meaning systems employing gRPC pipes or Unix domain sockets would necessitate additional hook development.

While tamper resistance is a significant strength of kernel-level tracing, it is not absolute. Bypassing it is considerably more difficult than circumventing application-level monitoring, but determined attackers might still employ techniques like container escape or `LD_PRELOAD` tricks to conceal their activities. The semantic gap remains perhaps the most fundamental challenge: although eBPF efficiently captures comprehensive system-level data with minimal overhead (typically 2-3% CPU usage), translating these low-level signals into meaningful insights about agent behavior requires sophisticated correlation and analysis techniques.

### Research Opportunities

The intersection of eBPF and AI observability presents compelling research avenues. Semantic anomaly detection stands out as a prime opportunity—leveraging LLMs to analyze captured agent conversations for patterns indicative of reasoning loops, contradictions, or policy violations. This introduces a recursive observation challenge: using AI to monitor AI, with its inherent philosophical and practical considerations.

Cross-agent correlation presents another rich area for investigation. Constructing causal graphs that connect the activities of multiple agents across process boundaries could reveal emergent behaviors that are not apparent at the individual agent level. Performance optimization remains crucial; pushing overhead below the current 3% threshold while maintaining full semantic capture would enable deployment in even the most performance-sensitive environments.

Privacy-preserving analysis techniques could enable monitoring of agent behavior without exposing sensitive prompt content, addressing a critical concern for enterprise deployments. Additionally, hardware acceleration through DPUs and SmartNICs could facilitate line-rate processing of agent traffic, effectively offloading CPU overhead from the host system.

## Conclusion

AI agents represent a significant paradigm shift in software architecture, challenging traditional observability assumptions. AgentSight's boundary tracing approach, implemented through eBPF technology, offers a stable and tamper-resistant foundation for understanding agent behavior. By observing at the system boundary rather than relying on rapidly evolving agent frameworks, we aim to achieve both technical stability and semantic richness.

The open-source AgentSight implementation demonstrates the feasibility of this approach with minimal overhead (less than 3% CPU usage). As AI agents become critical infrastructure, boundary-based observability will be essential for ensuring their security, reliability, and trustworthiness.

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