# AI Agent observability

## Problem / Gap:

1. **“AI Agents are evolve rapidly and different from traditional software”**

The rise of AI-powered agentic systems is transforming modern software infrastructure. Frameworks like AutoGen, LangChain, Claude Code, and gemini-cli orchestrate large language models (LLMs) to automate software engineering tasks, data analysis pipelines, and multi-agent decision-making. Unlike traditional software components that produce deterministic, easily observable behaviors, these AI-agent systems generate open-ended, non-deterministic outputs, often conditioned on hidden internal states and emergent interactions between multiple agents. Consequently, debugging and monitoring agentic software pose unprecedented observability challenges that classic application performance monitoring (APM) tools cannot address adequately.

### How AI-agent observability differs from classic software observability

| Dimension | Traditional app / micro-service | LLM or multi-agent system |
| --- | --- | --- |
| **What you try to “see”** | Latency, errors, CPU, GC, SQL counts, request paths | *Semantics* — prompt / tool trace, reasoning steps, toxicity, hallucination rate, persona drift, token / money you spend |
| **Ground truth** | Deterministic spec: given X you must produce Y or an exception | Open-ended output: many “acceptable” Y’s; quality judged by similarity, helpfulness, or policy compliance |
| **Failure modes** | Crashes, 5xx, memory leaks, deadlocks | Wrong facts, infinite reasoning loops, forgotten instructions, emergent mis-coordination between agents |
| **Time scale** | Millisecond spans; state usually dies at request end | Dialogue history and scratch memories can live for hours or days; “state” hides in vector DB rows and system prompts |
| **Signal source** | Structured logs and metrics you emit on purpose | Often *inside plain-text TLS payloads*; and tools exec logs |
| **Fix workflow** | Reproduce, attach debugger, patch code | Re-prompt, fine-tune, change tool wiring, tweak guardrails—code may be fine but “thought process” is wrong |
| **Safety / audit** | Trace shows what code ran | Need evidence of *why* the model said something for compliance / incident reviews |

Why the difference matters for research?

**Instrumentation gap** – Agent logic and algorithm changes daily (new prompts, tools) or by itself at runtime. Relying on in-code hooks means constant churn; kernel-side or side-car tracing stays stable.

**Semantic telemetry** – We need new span attributes (“model.temp”, “tool.role”, “reasoning.loop_id”) and new anomaly detectors (contradiction, persona drift).

**Causal fusion** – Research challenge: merge low-level events with high-level semantic spans into a single timeline so SREs can answer “why my code is not work? what system is it run on and what command have you tried?”

**Tamper resistance** – If prompt-injection turns the agent malicious it may silence its own logs. Out-of-process and kernel level tracing provides an independent audit channel.

In short, AI-agent observability inherits the **unreliable, emergent behaviour** of AI Agents.  Treat the agent runtime as a semi-trusted black box and observe at the system boundary: that’s where the and opportunities is.

1. **“Current observability techniques rely on application-level instrumentation”**

Current agent observability techniques rely predominantly on application-level instrumentation—callbacks, middleware hooks, or explicit logging—integrated within each agent framework. While intuitive, this approach suffers three fundamental limitations. First, agent frameworks evolve rapidly, changing prompts, tools, workflow and memory interfaces frequently. They can even modify their self code to create new tools, change prompts and behaviors. Thus, instrumentation embedded within agent codebases incurs significant maintenance overhead. Second, agent runtimes can be tampered with or compromised (e.g., via prompt injection), allowing attackers or buggy behaviors to evade logging entirely.  Fourth, application-level instrumentation cannot reliably capture cross-agent semantics, such as reasoning loops, semantic contradictions, persona shifts, or the behaviors when it’s interacting with it’s environment, especially when interactions cross process or binary boundaries (e.g., external tools or subprocesses).

For security, consider a llm agent first write a bash file with malicious commands (Not exec, safe), and then exec it with basic tool calls (Often allow it). It  needs system wide observability and constrains.

## AI Agent observability landscape

Below is a quick landscape scan of LLM / AI‑agent observability tooling as of July 2025. I focused on offerings that (a) expose an SDK, proxy, or spec you can wire into an agent stack today and (b) ship some way to trace / evaluate / monitor model calls in production.

| #  | Tool / SDK (year first shipped)                     | Integration path                                                   | What it gives you                                                                          | License / model                | Notes                                                                                                         |
| -- | --------------------------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------ | ------------------------------ | ------------------------------------------------------------------------------------------------------------- |
| 1  | **LangSmith** (2023)                                | Add `import langsmith` to any LangChain / LangGraph app            | Request/response traces, prompt & token stats, built‑in evaluation jobs                    | SaaS, free tier                | Tightest integration with LangChain; OTel export in beta. ([LangSmith][1])                                    |
| 2  | **Helicone** (2023)                                 | Drop‑in reverse‑proxy or Python/JS SDK                             | Logs every OpenAI‑style HTTP call; live cost & latency dashboards; “smart” model routing   | OSS core (MIT) + hosted        | Proxy model keeps app code unchanged. ([Helicone.ai][2], [Helicone.ai][3])                                    |
| 3  | **Traceloop** (2024)                                | One‑line AI‑SDK import → OTel                                      | Full OTel spans for prompts, tools, sub‑calls; replay & A/B test flows                     | SaaS, generous free tier       | Uses standard OTel data; works with any backend. ([AI SDK][4], [traceloop.com][5])                            |
| 4  | **Arize Phoenix** (2024)                            | `pip install arize-phoenix`; OpenInference tracer                  | Local UI + vector‑store for traces; automatic evals (toxicity, relevance) with another LLM | Apache‑2.0, self‑host or cloud | Ships its own open‑source UI; good for offline debugging. ([Phoenix][6], [GitHub][7])                         |
| 5  | **Langfuse** (2024)                                 | Langfuse SDK *or* send raw OTel OTLP                               | Nested traces, cost metrics, prompt mgmt, evals; self‑host in Docker                       | OSS (MIT) + cloud              | Popular in RAG / multi‑agent projects; OTLP endpoint keeps you vendor‑neutral. ([Langfuse][8], [Langfuse][9]) |
| 6  | **WhyLabs LangKit** (2023)                          | Wrapper that extracts text metrics                                 | Drift, toxicity, sentiment, PII flags; sends to WhyLabs platform                           | Apache‑2.0 core, paid cloud    | Adds HEAVY text‑quality metrics rather than request tracing. ([WhyLabs][10], [docs.whylabs.ai][11])           |
| 7  | **PromptLayer** (2022)                              | Decorator / context‑manager or proxy                               | Timeline view of prompt chains; diff & replay; built on OTel spans                         | SaaS                           | Early mover; minimal code changes but not open source. ([PromptLayer][12], [PromptLayer][13])                 |
| 8  | **Literal AI** (2024)                               | Python SDK + UI                                                    | RAG‑aware traces, eval experiments, datasets                                               | OSS core + SaaS                | Aimed at product teams shipping chatbots. ([literalai.com][14], [literalai.com][15])                          |
| 9  | **W\&B Weave / Traces** (2024)                      | `import weave` or W\&B SDK                                         | Deep link into existing W\&B projects; captures code, inputs, outputs, user feedback       | SaaS                           | Nice if you already use W\&B for ML experiments. ([Weights & Biases][16])                                     |
| 10 | **Honeycomb Gen‑AI views** (2024)                   | Send OTel spans; Honeycomb UI                                      | Heat‑map + BubbleUp on prompt spans, latency, errors                                       | SaaS                           | Built atop Honeycomb’s mature trace store; no eval layer. ([Honeycomb][17])                                   |
| 11 | **OpenTelemetry GenAI semantic‑conventions** (2024) | Spec + contrib Python lib (`opentelemetry-instrumentation-openai`) | Standard span/metric names for models, agents, prompts                                     | Apache‑2.0                     | Gives you a lingua‑franca; several tools above emit it. ([OpenTelemetry][18])                                 |
| 12 | **OpenInference spec** (2023)                       | Tracer wrapper (supports LangChain, LlamaIndex, Autogen…)          | JSON schema for traces + plug‑ins; Phoenix uses it                                         | Apache‑2.0                     | Spec, not a hosted service; pairs well with any OTel backend. ([GitHub][19])                                  |


### What the landscape tells us

* **Almost everyone hooks at the SDK layer.** 11 of 12 options require you to wrap or proxy function calls. That’s fine for proof‑of‑concepts but breaks when an agent hot‑swaps prompts or spawns new tools that bypass the wrapper.
* **OpenTelemetry is becoming the de‑facto wire format.** Traceloop, Honeycomb, Langfuse, PromptLayer, Phoenix (via OpenInference) all speak OTel, which simplifies backend choice.
* **Semantic evaluation is still early.** Only Phoenix, LangSmith, Langfuse, and Literal ship built‑in LLM‑powered quality checks (toxicity, relevance, hallucination score). Most others focus on latency + cost.
* **No one does kernel‑level capture.** None of the listed tools observe encrypted TLS buffers or `execve()` calls directly; they trust the application layer to be honest. That leaves a blind spot for prompt‑injection or self‑modifying agents—exactly the gap a zero‑instrumentation eBPF tracer could close.
* **Specs vs. platforms.** OpenTelemetry GenAI and OpenInference lower the integration tax but don’t store or visualize anything; you still need a backend. Conversely, SaaS platforms bundle storage, query, and eval but lock you into their data shape.

### How this motivates the “boundary tracing” idea

Because today’s solutions *mostly* live inside the agent process, they inherit the same fragility as the agent code:

* **Breakage when you tweak the prompt graph** – each new node needs a decorator.
* **Evasion by malicious prompts** – compromised agent can drop or fake logs.
* **Blind to cross‑process side effects** – e.g., writing a shell script then `execve()`‑ing it.

A system‑level eBPF tracer that scoops TLS write buffers and syscalls sidesteps those issues:

| Where today’s SDKs stop                            | What boundary tracing would still see |
| -------------------------------------------------- | ------------------------------------- |
| Missing span when agent spawns `curl` directly     | `execve("curl", …)` + network write   |
| Agent mutates its own prompt string before logging | Raw ciphertext leaving the TLS socket |
| Sub‑process mis‑uses GPU                           | `ioctl` + CUDA driver calls           |

In other words, existing tools solve the “what happened inside my code?” story; kernel‑side tracing can answer “what actually hit the wire and the OS?”—a complementary, harder‑to‑tamper vantage point.

That gap is wide open for research and open‑source innovation.

## **Key Insight and observation**

All meaningful interactions of existing AI-agent system has two clear traverse boundaries:

> AI agent observability must be decoupled from agent internals. **observing from the boundary provides a stable semantic interface**.
> 

### AI Agent struct

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

- **LLM serving provider**  – token generation, non-deterministic reasoning, chain-of-thought text that may or may not be surfaced. Most system work are around the llm serving layer.
- **Agent runtime layer** – turns tasks into a sequence of LLM calls plus external tool invocations; stores transient “memories”.
- **Outside world** – OS, containers, other services.

For **observability purposes** the clean interface is usually the *network boundary* (TLS write of a JSON inference request) and the system boundary (syscall / subprocess when the agent hits commands `curl`, `grep`).  Anything below those lines (GPU kernels, weight matrices, models) is model-inference serving territory; anything above is classic system observability tasks.  That’s why kernel-level eBPF can give you a neutral vantage: it straddles both worlds without needing library hooks.

Traditional software observability is **instrumentation-first** (you insert logs, spans, and metrics into the code you write).

But AI agents change their internal logic dynamically through prompts, instructions, reasoning paths, and spontaneous tool usage. This constant internal mutability means *instrumentation is fragile*.

By shifting observability to a stable **system-level boundary**—the kernel syscall interface, TLS buffers, network sockets—you achieve:

- **Framework neutrality**: Works across all agent runtimes (LangChain, AutoGen, gemini-cli).
- **Semantic stability**: C aptures prompt-level semantics without chasing framework APIs.
- **Trust & auditability**: Independent trace that can’t be easily compromised by in-agent malware.
- **Universal causal graph**: Merges agent-level semantics with OS-level events into one coherent story.

---

## System build

1. A zero-instrumentation observability tool for AI agent systems built entirely on **system-level tracing (eBPF)** to achieve unified semantic and operational visibility independent of the rapidly-evolving agent runtimes and frameworks.
2. A llm “sidecar” approach to detect subtle semantic anomalies (e.g., reasoning loops, contradictions, persona shifts) together with the system logs.

## Challenges

The core challenge lies in the **semantic gap** between kernel-level signals and AI agent behaviors. While eBPF can capture comprehensive system-level data with minimal overhead (typically 2-3% CPU usage), translating this into meaningful insights about agent performance requires sophisticated correlation techniques.

Another challenge is capture all prompts and interactions witrh backend server is from encrypted TLS traffic. most llm serving are using TLS to communicate with backend server, and using SSE to stream the response. Using traditional network packet capture tools like tcpdump or wireshark is not enough, because the traffic is encrypted. Proxy the traffic can be a alternative solution, but proxy solutions require explicit configuration changes to route agent traffic through the proxy, which may not work with third party applications or frameworks and can introduce additional latency and complexity.

By using eBPF uprobe to hook the TLS read and write in userspace, we can capture the traffic and decrypt it.

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
