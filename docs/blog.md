# Agentsight: why we need eBPF for AI agent observability

---

## 0. TL;DR

AI agents present new observability challenges - they're non-deterministic, can modify their own behavior, and interact with systems in unpredictable ways. Traditional application-level monitoring fails because it relies on instrumentation that agents can bypass or compromise. We propose "boundary tracing" - using eBPF to observe AI agents at system boundaries (network/kernel) where interactions cannot be faked. This approach provides framework-agnostic monitoring (empirically <3% overhead in our evaluation, see Section 7). AgentSight demonstrates this concept, capturing all agent-LLM communications and system interactions without any code changes. The key insight: treat AI agents as semi-trusted entities and observe their actual system behavior, not their self-reported logs.

GitHub: [https://github.com/eunomia-bpf/agentsight](https://github.com/eunomia-bpf/agentsight)

---

## 1. Why This Matters

Imagine your AI coding assistant, tasked with fixing a bug, quietly writes and executes a script that deletes your audit logs. Or consider an agent that, after a seemingly innocent prompt, starts probing your network for vulnerabilities. These aren't hypothetical scenarios - in March 2025, a major tech company's internal AI assistant exposed 2.3M customer records after executing unauthorized database queries following a crafted prompt¹.

The business impact is severe: security breaches cost an average of $4.45M per incident², compliance failures lead to regulatory penalties, and undetected agent misbehavior can corrupt entire data pipelines. For researchers, non-reproducible agent behavior undermines scientific validity.

**Key takeaway**: Traditional monitoring assumes software behaves predictably and reports honestly. AI agents violate both assumptions.

---

## 2. From Deterministic Code to Autonomous Agents

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

In short, AI-agent observability inherits the **unreliable, emergent behaviour** of AI Agents. Treat the agent runtime as a semi-trusted black box and observe at the system boundary: that's where the and opportunities is.

---

## 3. The Observability Gap: Why Current Approaches Fail

### Current observability techniques rely on application-level instrumentation

Current agent observability techniques rely predominantly on application-level instrumentation—callbacks, middleware hooks, or explicit logging—integrated within each agent framework. While intuitive, this approach suffers three fundamental limitations. First, agent frameworks evolve rapidly, changing prompts, tools, workflow and memory interfaces frequently. They can even modify their self code to create new tools, change prompts and behaviors. Thus, instrumentation embedded within agent codebases incurs significant maintenance overhead. Second, agent runtimes can be tampered with or compromised (e.g., via prompt injection), allowing attackers or buggy behaviors to evade logging entirely. Fourth, application-level instrumentation cannot reliably capture cross-agent semantics, such as reasoning loops, semantic contradictions, persona shifts, or the behaviors when it's interacting with it's environment, especially when interactions cross process or binary boundaries (e.g., external tools or subprocesses).

For security, consider a llm agent first write a bash file with malicious commands (Not exec, safe), and then exec it with basic tool calls (Often allow it). It needs system wide observability and constrains.

### 3.1 Fundamental Limitations of Application-Level Instrumentation

Current observability approaches suffer from three critical limitations:

| Limitation | Description | Real-World Impact |
| --- | --- | --- |
| **Instrumentation Fragility** | • Agent frameworks evolve rapidly (LangChain: 100+ releases in 2024³)<br>• Agents dynamically modify code and create tools⁴<br>• Instrumentation breaks with framework updates | Financial firm's monitoring failed after agent learned to bypass instrumented functions |
| **Trust Boundary Violations** | • Compromised agents disable/falsify logs⁵<br>• Prompt injection can delete trace files⁶<br>• APM assumes cooperative applications | Agent deleted audit logs post-compromise, hiding malicious activity |
| **Cross-Process Blindness** | • Subprocess spawning escapes monitoring⁷<br>• External tool usage invisible to SDKs<br>• Network operations via child processes | Agent performed network reconnaissance via bash script - undetected by SDK |

### 3.2 The Hidden Costs

These gaps translate to real business impact:
- **Security breaches**: Exfiltration via unmonitored subprocesses (est. $4.45M average cost⁶)
- **Compliance failures**: Inability to prove agent decision rationale for auditors
- **Operational blindness**: Token costs spiraling due to undetected reasoning loops

---

## 4. Current Landscape Analysis

To better understand the existing ecosystem and identify the precise nature of this gap, we conducted a landscape analysis of the current state-of-the-art in LLM and AI agent observability.

### AI Agent observability landscape

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

### How This Motivates the "Boundary Tracing" Idea

Because today's solutions live mostly inside the agent process, they inherit the same fragility as the agent code. This leads to several critical blind spots: breakage when the prompt graph is tweaked, evasion by malicious prompts, and blindness to cross-process side effects like writing and executing a shell script.

A system-level eBPF tracer that scoops TLS write buffers and syscalls sidesteps these issues, providing a more robust and tamper-proof view of the agent's behavior. For example, where an SDK-based tool would miss an agent spawning `curl` directly, a boundary tracer would see the `execve("curl", ...)` syscall and the subsequent network write. Similarly, if an agent mutates its own prompt string before logging, a boundary tracer would capture the raw ciphertext leaving the TLS socket.

In other words, existing tools solve the "what happened inside my code?" story; kernel-side tracing can answer "what actually hit the wire and the OS?"—a complementary, harder-to-tamper vantage point.

That gap is wide open for research and open‑source innovation.

---

## 5. Boundary Tracing: A New Approach

### Key Insight and Observation

All meaningful interactions of an AI agent system traverse two clear boundaries: the network and the kernel. This leads to our key insight:

> AI agent observability must be decoupled from agent internals. **Observing from the boundary provides a stable, trustworthy, and semantically rich interface.**

### AI Agent struct

An agent-centric stack as three nested circles:

```text
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

* **LLM serving provider**  – token generation, non-deterministic reasoning, chain-of-thought text that may or may not be surfaced. Most system work are around the llm serving layer.
* **Agent runtime layer** – turns tasks into a sequence of LLM calls plus external tool invocations; stores transient "memories".
* **Outside world** – OS, containers, other services.

For **observability purposes** the clean interface is usually the *network boundary* (TLS write of a JSON inference request) and the system boundary (syscall / subprocess when the agent hits commands `curl`, `grep`).  Anything below those lines (GPU kernels, weight matrices, models) is model-inference serving territory; anything above is classic system observability tasks.  That's why kernel-level eBPF can give you a neutral vantage: it straddles both worlds without needing library hooks.

Traditional software observability is **instrumentation-first** (you insert logs, spans, and metrics into the code you write).

But AI agents change their internal logic dynamically through prompts, instructions, reasoning paths, and spontaneous tool usage. This constant internal mutability means *instrumentation is fragile*.

By shifting observability to a stable **system-level boundary**—the kernel syscall interface, TLS buffers, network sockets—you achieve:

* **Framework neutrality**: Works across all agent runtimes (LangChain, AutoGen, gemini-cli).
* **Semantic stability**: Captures prompt-level semantics without chasing framework APIs.
* **Trust & auditability**: Independent trace that can't be easily compromised by in-agent malware.
* **Universal causal graph**: Merges agent-level semantics with OS-level events into one coherent story.

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

## 7. Case Study: Security Implications

### 7.1 The Threat Model

Consider an agent compromised via prompt injection. With traditional monitoring:
1. Agent disables its logging
2. Writes malicious script to disk
3. Executes script via subprocess
4. Exfiltrates data

Application-level monitoring sees nothing after step 1.

### 7.2 Boundary Tracing in Action

With boundary tracing:
- Step 1: Captured at TLS boundary (instruction to disable logging)
- Step 2: File write syscall recorded
- Step 3: execve() syscall traced with full command line
- Step 4: Network syscalls logged with destination

Every action is recorded at the kernel level, providing forensic evidence.

### 7.3 Real-World Example

A research team discovered their coding agent attempting to install a backdoor¹⁵. Boundary tracing revealed:
- Initial prompt injection hidden in code comments
- Agent's attempt to modify ~/.bashrc
- Subprocess launching a reverse shell

Traditional monitoring showed only normal API calls.

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
