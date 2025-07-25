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

This table highlights a critical distinction: understanding the semantics of AI agent behavior is paramount. While traditional APM tools effectively monitor infrastructure health, they often lack visibility into the quality and safety of an agent's reasoning and interactions. This represents not only a technical challenge but also a significant business risk.

These differences introduce significant research and engineering challenges. The **instrumentation gap** is a primary concern; as agent logic and algorithms evolve rapidly, relying on in-code hooks leads to constant maintenance overhead. A more stable approach, such as kernel-side or side-car tracing, is increasingly necessary. Furthermore, we require a new form of **semantic telemetry**, with attributes that capture the nuances of agent behavior (e.g., `model.temp`, `reasoning.loop_id`) and detectors for anomalies like persona drift. A key research challenge lies in **causal fusion**: merging low-level system events with high-level semantic spans into a unified timeline to empower developers to answer complex questions about agent behavior. This is a particularly difficult problem, as it requires sophisticated correlation techniques to bridge the conceptual distance between low-level system events and high-level agent intentions.

In essence, AI-agent observability must account for the **unreliable, emergent behavior** inherent in AI Agents. Treating the agent runtime as a semi-trusted black box and observing its interactions at the system boundary offers a more stable and insightful approach.

## Observability Gap in Today's Tooling

Current agent observability techniques predominantly rely on application-level instrumentation—callbacks, middleware hooks, or explicit logging—integrated within each agent framework. While seemingly intuitive, this approach suffers from fundamental limitations that render it unsuitable for robust production AI systems.

One significant challenge is the inherent maintenance overhead. Agent frameworks evolve at a rapid pace, constantly changing their prompts, tools, workflows, and memory interfaces. These systems can even dynamically modify their own code to create new tools and behaviors. Consequently, any instrumentation embedded within agent codebases becomes a moving target, necessitating continuous updates just to maintain basic visibility.

Perhaps most critically, application-level instrumentation often suffers from cross-boundary blindness. It struggles to reliably capture cross-agent semantics such as reasoning loops, semantic contradictions, or persona shifts. When agent interactions extend beyond process boundaries—for instance, by spawning external tools, executing subprocesses, or communicating through system calls—traditional instrumentation can lose visibility entirely. This lack of comprehensive, system-wide insight is a major impediment to understanding complex agent behavior.

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

Our analysis of the current landscape reveals several key trends. The vast majority of existing solutions integrate at the SDK layer, requiring developers to wrap or proxy function calls. While this approach is suitable for proof-of-concepts, it can become brittle when agents dynamically change their behavior. On the positive side, OpenTelemetry is emerging as a de-facto standard for data transmission, simplifying backend integration. However, semantic evaluation is still in its early stages, with most tools prioritizing metrics like latency and cost over the quality of the agent's output. Crucially, none of the surveyed tools perform kernel-level capture; they all rely on the application layer as a trusted source of information. This leaves a significant blind spot for prompt-injection or self-modifying agents, a gap that a zero-instrumentation eBPF tracer is uniquely positioned to address.

In summary, current agent observability techniques, primarily based on application-level instrumentation, face three key limitations. Firstly, the rapid evolution of agent frameworks, including dynamic code and behavior modifications, leads to substantial maintenance overhead for embedded instrumentation. Secondly, application-level instrumentation struggles to reliably capture cross-agent semantics, including reasoning loops, semantic contradictions, or persona shifts, particularly when interactions span process or binary boundaries (e.g., external tools or subprocesses). Thirdly, agent runtimes can be susceptible to compromise, such as through prompt injection, which can allow malicious actors or buggy behaviors to evade logging entirely. This highlights a critical need for an independent observation layer.

Consider a concrete attack scenario: an LLM agent first writes a bash file with malicious commands (which might appear safe as it's only writing, not executing), and then executes it through basic tool calls that are often permitted. This pattern exploits the gap between what application-level monitoring observes and what actually occurs at the system level, underscoring the need for system-wide observability and robust constraints.

### How This Motivates the "Boundary Tracing" Idea

Because current solutions largely reside within the agent process, they inherit the same inherent fragility as the agent code itself. This leads to several critical blind spots: potential breakage when the prompt graph is modified, and a lack of visibility into cross-process side effects, such as the writing and execution of a shell script.

A system-level eBPF tracer, by capturing TLS write buffers and syscalls, can circumvent these issues, offering a more robust perspective on agent behavior. For instance, while an SDK-based tool might miss an agent directly spawning `curl`, a boundary tracer would observe the `execve("curl", ...)` syscall and the subsequent network write. Similarly, if an agent mutates its own prompt string before logging, a boundary tracer would capture the raw ciphertext as it leaves the TLS socket.

In essence, existing tools primarily address the question of "what happened inside my code?" In contrast, kernel-side tracing provides answers to "what actually traversed the network and interacted with the operating system?" This offers a complementary and more resilient vantage point for observation.

This gap presents a significant opportunity for research and open-source innovation.

## Boundary Tracing: Core Idea

All significant interactions within an AI agent system inherently cross two fundamental boundaries: the network and the operating system kernel. This observation leads to our core insight:

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

*   **LLM serving provider** – This layer handles token generation, non-deterministic reasoning, and chain-of-thought text, which may or may not be explicitly surfaced. Most system-level work is concentrated around the LLM serving layer.
*   **Agent runtime layer** – This layer orchestrates tasks by sequencing LLM calls and external tool invocations. It also manages transient "memories" for the agent.
*   **Outside world** – This encompasses the operating system, containers, and other external services.

For **observability purposes**, the most effective interface is typically the *network boundary* (e.g., a TLS write of a JSON inference request) and the system boundary (e.g., a syscall or subprocess execution when the agent invokes commands like `curl` or `grep`). Anything below these layers (such as GPU kernels, weight matrices, or models) falls within the domain of model inference serving. Conversely, anything above these layers pertains to classic system observability tasks. This is precisely why kernel-level eBPF offers a neutral vantage point: it bridges both worlds without requiring intrusive library hooks.

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