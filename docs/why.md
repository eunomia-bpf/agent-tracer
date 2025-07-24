# AgentSight: Why We Need eBPF for AI Agent Observability

## Executive Summary

The rise of AI-powered agents represents a fundamental shift in software architecture. Unlike traditional deterministic services, AI agents exhibit emergent behaviors, make autonomous decisions, and can modify their own execution patterns. This creates critical observability gaps that existing tools cannot address.

Consider the security implications: A recent prompt-injection vulnerability in Meta AI's assistant exposed user conversations to unauthorized parties in January 2025[^1]. Traditional application-level monitoring would miss such attacks because compromised agents can disable their own logging. The financial stakes are significant—security breaches cost organizations an average of $4.88 million per incident according to IBM's 2024 report[^2].

**AgentSight introduces boundary tracing**: a framework-agnostic approach that observes AI agents at the system boundary using eBPF technology. By capturing SSL/TLS traffic and process behaviors at the kernel level, we achieve:

- **Framework independence**: Works across LangChain (100+ releases in 2024[^3]), AutoGen, Claude Code, and emerging frameworks
- **Tamper resistance**: Kernel-level tracing that compromised agents cannot evade
- **Semantic visibility**: Full prompt/response capture including streaming Server-Sent Events (SSE)
- **Low overhead**: Less than 3% CPU impact through efficient eBPF programs

## How Agents Break Traditional Monitoring

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

## AI Agent Observability Landscape

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

### What the Landscape Tells Us

Our analysis of the current landscape reveals several key trends. The vast majority of existing solutions hook into the SDK layer, requiring developers to wrap or proxy function calls. While this approach is suitable for proof-of-concepts, it becomes brittle when agents dynamically change their behavior. On the positive side, OpenTelemetry is emerging as the de-facto wire format, simplifying backend integration. However, semantic evaluation is still in its early stages, with most tools focusing on latency and cost rather than the quality of the agent's output. Most importantly, none of the surveyed tools perform kernel-level capture; they all trust the application layer to be a reliable source of information. This leaves a significant blind spot for prompt-injection or self-modifying agents, a gap that a zero-instrumentation eBPF tracer is perfectly positioned to fill.

In summary, the current generation of tools provides essential visibility into the application layer, but they operate under the assumption that the application is a reliable narrator of its own story. For AI agents, this assumption is no longer safe.

## The Observability Gap

Current agent observability techniques rely predominantly on application-level instrumentation—callbacks, middleware hooks, or explicit logging—integrated within each agent framework. While intuitive, this approach suffers three fundamental limitations. First, agent frameworks evolve rapidly, changing prompts, tools, workflow and memory interfaces frequently. They can even modify their self code to create new tools, change prompts and behaviors. Thus, instrumentation embedded within agent codebases incurs significant maintenance overhead. Second, agent runtimes can be tampered with or compromised (e.g., via prompt injection), allowing attackers or buggy behaviors to evade logging entirely. Third, application-level instrumentation cannot reliably capture cross-agent semantics, such as reasoning loops, semantic contradictions, persona shifts, or the behaviors when it's interacting with its environment, especially when interactions cross process or binary boundaries (e.g., external tools or subprocesses).

For security, consider a LLM agent first write a bash file with malicious commands (not exec, safe), and then exec it with basic tool calls (often allow it). It needs system wide observability and constraints.

### How This Motivates the "Boundary Tracing" Idea

Because today's solutions live mostly inside the agent process, they inherit the same fragility as the agent code. This leads to several critical blind spots: breakage when the prompt graph is tweaked, evasion by malicious prompts, and blindness to cross-process side effects like writing and executing a shell script.

A system-level eBPF tracer that scoops TLS write buffers and syscalls sidesteps these issues, providing a more robust and tamper-proof view of the agent's behavior. For example, where an SDK-based tool would miss an agent spawning `curl` directly, a boundary tracer would see the `execve("curl", ...)` syscall and the subsequent network write. Similarly, if an agent mutates its own prompt string before logging, a boundary tracer would capture the raw ciphertext leaving the TLS socket.

In other words, existing tools solve the "what happened inside my code?" story; kernel-side tracing can answer "what actually hit the wire and the OS?"—a complementary, harder-to-tamper vantage point.

That gap is wide open for research and open‑source innovation.

## Boundary Tracing with eBPF

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

Traditional software observability is **instrumentation-first** (you insert logs, spans, and metrics into the code you write).

But AI agents change their internal logic dynamically through prompts, instructions, reasoning paths, and spontaneous tool usage. This constant internal mutability means *instrumentation is fragile*.

By shifting observability to a stable **system-level boundary**—the kernel syscall interface, TLS buffers, network sockets—you achieve:

* **Framework neutrality**: Works across all agent runtimes (LangChain, AutoGen, gemini-cli) without modification
* **Semantic stability**: Captures prompt-level semantics without chasing framework APIs
* **Trust & auditability**: Independent trace that can't be easily compromised by in-agent malware
* **Universal causal graph**: Merges agent-level semantics with OS-level events into one coherent story

### Technical Foundation: eBPF for TLS Interception

One core challenge lies in capturing all prompts and interactions with backend servers from encrypted TLS traffic. Most LLM serving uses TLS to communicate with backend servers, and uses Server-Sent Events (SSE) to stream responses. Using traditional network packet capture tools like tcpdump or wireshark is not sufficient because the traffic is encrypted. Proxy solutions require explicit configuration changes to route agent traffic through the proxy, which may not work with third-party applications or frameworks and can introduce additional latency and complexity.

By using eBPF uprobes to hook TLS read and write functions in userspace[^10], we can capture the traffic and decrypt it transparently. This approach:

- Works with any TLS library (OpenSSL, BoringSSL, GnuTLS) through CO-RE (Compile Once - Run Everywhere)
- Captures plaintext before encryption and after decryption
- Supports streaming protocols like SSE without buffering issues
- Requires no application changes or proxy configuration

Recent work by Keploy[^11] and Pixie Labs[^12] demonstrates mature eBPF-based TLS tracing capabilities. The eunomia.dev tutorial[^13] provides implementation details for SSL/TLS capture using eBPF.

## System Build and Architecture

AgentSight implements:

1. A zero-instrumentation observability tool for AI agent systems built entirely on **system-level tracing (eBPF)** to achieve unified semantic and operational visibility independent of the rapidly-evolving agent runtimes and frameworks.
2. A LLM "sidecar" approach to detect subtle semantic anomalies (e.g., reasoning loops, contradictions, persona shifts) together with the system logs.

The architecture consists of:

- **eBPF programs** for TLS interception and process monitoring
- **Streaming analysis framework** for real-time event processing
- **Semantic analysis engine** for detecting agent-specific anomalies
- **Visualization frontend** for timeline-based exploration

## Open Challenges and Research Directions

The AI Agent is fundamentally different from traditional software, it's more like a "user in the system" that can do anything. It can spawn subprocesses, use external tools, and even modify its own code. It can also be compromised by malicious prompts or self-modifying code.

One core challenge lies in the **semantic gap** between kernel-level signals and AI agent behaviors. While eBPF can capture comprehensive system-level data with minimal overhead (typically 2-3% CPU usage), translating this into meaningful insights about agent performance requires sophisticated correlation techniques.

Another challenge is capture all prompts and interactions witrh backend server is from encrypted TLS traffic. most llm serving are using TLS to communicate with backend server, and using SSE to stream the response. Using traditional network packet capture tools like tcpdump or wireshark is not enough, because the traffic is encrypted. Proxy the traffic can be a alternative solution, but proxy solutions require explicit configuration changes to route agent traffic through the proxy, which may not work with third party applications or frameworks and can introduce additional latency and complexity. Even if existing eBPF tools can capture the traffic, it lacks support for SSE stream API support.

By using eBPF uprobe to hook the TLS read and write in userspace, we can capture the traffic and decrypt it.

### Technical Limitations

1. **TLS capture complexity**: While eBPF can intercept TLS, it fails for statically linked Go binaries using crypto/tls unless USDT hooks are enabled
2. **Framework coverage**: Current approach works for HTTP/TLS-speaking agents; systems using gRPC pipes or Unix domain sockets require additional hooks
3. **Tamper resistance boundaries**: Kernel-level tracing is harder but not impossible to bypass; container escape or LD_PRELOAD can still hide activities

### Research Opportunities

- **Semantic anomaly detection**: Using LLMs to analyze captured agent conversations for reasoning loops, contradictions, or policy violations
- **Cross-agent correlation**: Building causal graphs that connect multiple agents' activities across process boundaries
- **Performance optimization**: Reducing overhead below 3% while maintaining full semantic capture
- **Privacy-preserving analysis**: Techniques for monitoring agent behavior without exposing sensitive prompt content

## Conclusion

AI agents represent a paradigm shift in software architecture that breaks traditional observability assumptions. AgentSight's boundary tracing approach, implemented through eBPF technology, provides a stable, tamper-resistant foundation for understanding agent behavior. By observing at the system boundary rather than within rapidly-evolving agent frameworks, we achieve both technical stability and semantic richness.

The open-source AgentSight implementation demonstrates the feasibility of this approach with less than 3% overhead. As AI agents become critical infrastructure, boundary-based observability will be essential for security, reliability, and trust.

## References

[^1]: Meta AI prompt-exposure incident, January 2025. [Tom's Guide](https://www.tomsguide.com/computing/online-security/meta-ai-was-leaking-chatbot-prompts-and-answers-to-unauthorized-users)
[^2]: IBM. "Cost of a Data Breach Report 2024." [IBM](https://www.ibm.com/think/insights/cost-of-a-data-breach-2024-financial-industry)
[^3]: LangChain GitHub releases page, 2024. [GitHub](https://github.com/langchain-ai/langchain/releases)
[^10]: eBPF uprobe documentation. [kernel.org](https://www.kernel.org/doc/html/latest/trace/uprobetracer.html)
[^11]: Keploy. "eBPF for TLS Traffic Tracing: Secure & Efficient Observability," January 2025. [Keploy](https://keploy.io/blog/community/ebpf-for-tls-traffic-tracing-secure-efficient-observability)
[^12]: Pixie Labs. "eBPF TLS Tracing: Past, Present & Future," September 2024. [blog.px.dev](https://blog.px.dev/ebpf-tls-tracing-past-present-future/)
[^13]: Eunomia. "eBPF Practical Tutorial: Capturing SSL/TLS Plain Text Data," 2025. [eunomia.dev](https://eunomia.dev/en/tutorials/30-sslsniff/)
[^14]: OWASP GenAI Security Project. "LLM01:2025 Prompt Injection," 2025. [OWASP](https://genai.owasp.org/llmrisk/llm01-prompt-injection/)

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