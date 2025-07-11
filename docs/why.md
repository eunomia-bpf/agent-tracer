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

Why the difference matters for research

**Instrumentation gap** – Agent logic and algorithm changes daily (new prompts, tools) or by itself at runtime. Relying on in-code hooks means constant churn; kernel-side or side-car tracing stays stable.

**Semantic telemetry** – We need new span attributes (“model.temp”, “tool.role”, “reasoning.loop_id”) and new anomaly detectors (contradiction, persona drift).

**Causal fusion** – Research challenge: merge low-level events with high-level semantic spans into a single timeline so SREs can answer “why my code is not work? what system is it run on and what command have you tried?”

**Tamper resistance** – If prompt-injection turns the agent malicious it may silence its own logs. Out-of-process and kernel level tracing provides an independent audit channel.

In short, AI-agent observability inherits the **unreliable, emergent behaviour** of AI Agents.  Treat the agent runtime as a semi-trusted black box and observe at the system boundary: that’s where the and opportunities is.

1. **“Current observability techniques rely on application-level instrumentation”**

Current agent observability techniques rely predominantly on application-level instrumentation—callbacks, middleware hooks, or explicit logging—integrated within each agent framework. While intuitive, this approach suffers three fundamental limitations. First, agent frameworks evolve rapidly, changing prompts, tools, workflow and memory interfaces frequently. They can even modify their self code to create new tools, change prompts and behaviors. Thus, instrumentation embedded within agent codebases incurs significant maintenance overhead. Second, agent runtimes can be tampered with or compromised (e.g., via prompt injection), allowing attackers or buggy behaviors to evade logging entirely.  Fourth, application-level instrumentation cannot reliably capture cross-agent semantics, such as reasoning loops, semantic contradictions, persona shifts, or the behaviors when it’s interacting with it’s environment, especially when interactions cross process or binary boundaries (e.g., external tools or subprocesses).

For security, consider a llm agent first write a bash file with malicious commands (Not exec, safe), and then exec it with basic tool calls (Often allow it). It  needs system wide observability and constrains.

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

1. A zero-instrumentation observability layer for AI agent systems built entirely on **system-level tracing (eBPF)** to achieve unified semantic and operational visibility independent of the rapidly-evolving agent runtimes.
2. A llm “sidecar” approach to detect subtle semantic anomalies (e.g., reasoning loops, contradictions, persona shifts) together with the system logs.

The core challenge lies in the **semantic gap** between kernel-level signals and AI agent behaviors. While eBPF can capture comprehensive system-level data with minimal overhead (typically 2-3% CPU usage), translating this into meaningful insights about agent performance requires sophisticated correlation techniques.

Another challenge is capture all prompts and interactions witrh backend server is from encrypted TLS traffic.
