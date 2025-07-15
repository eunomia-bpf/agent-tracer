# AgentSight: Evaluating eBPF-based AI Agent Observability as a Research Paper

This document outlines a comprehensive evaluation framework for AgentSight, a novel observability system that leverages eBPF technology to provide tamper-resistant, kernel-level monitoring of AI agents. We analyze the research novelty, identify key evaluation dimensions, propose experimental methodologies, and define success metrics for assessing this system's contribution to the field of AI agent observability.

## 1. Research Problem and Motivation

### 1.1 The Core Problem: Observability Gap in AI Agent Systems

AI agents represent a paradigm shift in software development, introducing unique observability challenges that traditional monitoring solutions cannot address:

**The Fundamental Challenge:**
Unlike traditional software components that produce deterministic, easily observable behaviors, AI-agent systems generate open-ended, non-deterministic outputs, often conditioned on hidden internal states and emergent interactions between multiple agents. These systems can modify their own behavior, create new tools, and interact with the environment in ways that bypass traditional instrumentation.

**How AI-agent observability differs from classic software observability:**

| Dimension | Traditional app / micro-service | LLM or multi-agent system |
| --- | --- | --- |
| **What you try to "see"** | Latency, errors, CPU, GC, SQL counts, request paths | *Semantics* — prompt / tool trace, reasoning steps, toxicity, hallucination rate, persona drift, token / money you spend |
| **Ground truth** | Deterministic spec: given X you must produce Y or an exception | Open-ended output: many "acceptable" Y's; quality judged by similarity, helpfulness, or policy compliance |
| **Failure modes** | Crashes, 5xx, memory leaks, deadlocks | Wrong facts, infinite reasoning loops, forgotten instructions, emergent mis-coordination between agents |
| **Time scale** | Millisecond spans; state usually dies at request end | Dialogue history and scratch memories can live for hours or days; "state" hides in vector DB rows and system prompts |
| **Signal source** | Structured logs and metrics you emit on purpose | Often *inside plain-text TLS payloads*; and tools exec logs |
| **Fix workflow** | Reproduce, attach debugger, patch code | Re-prompt, fine-tune, change tool wiring, tweak guardrails—code may be fine but "thought process" is wrong |
| **Safety / audit** | Trace shows what code ran | Need evidence of *why* the model said something for compliance / incident reviews |

### 1.2 Current Solutions and Their Limitations

**Current observability techniques rely predominantly on application-level instrumentation:**

* **Instrumentation gap** – Agent logic and algorithm changes daily (new prompts, tools) or by itself at runtime. Relying on in-code hooks means constant churn; kernel-side or side-car tracing stays stable.
* **Semantic telemetry** – We need new span attributes ("model.temp", "tool.role", "reasoning.loop_id") and new anomaly detectors (contradiction, persona drift).
* **Tamper resistance** – If prompt-injection turns the agent malicious it may silence its own logs. Out-of-process and kernel level tracing provides an independent audit channel.

**Landscape Analysis of Current Solutions:**

| Tool | Integration Path | Limitations |
|------|------------------|-------------|
| **LangSmith** | Application-level SDK | Framework-specific, can be bypassed |
| **Helicone** | Reverse proxy | Requires configuration changes, adds latency |
| **Traceloop** | OTel instrumentation | Still application-level, fragile |
| **Arize Phoenix** | SDK wrapper | Limited to instrumented code paths |

**Key Insight:**
> AI agent observability must be decoupled from agent internals. **Observing from the boundary provides a stable semantic interface**.

### 1.3 Research Hypothesis

**The AgentSight Hypothesis:**
By operating at the kernel level using eBPF, we can provide tamper-resistant, comprehensive observability that captures agent behavior at the system boundary, where it cannot be easily circumvented or modified.

## 2. Technical Innovation and Novelty

### 2.1 Core Technical Contributions

**Primary Innovation: eBPF-based Agent Monitoring**
* First system to apply eBPF technology specifically to AI agent observability
* Kernel-level interception of SSL/TLS traffic to capture unencrypted agent communications
* Process-level monitoring of agent system interactions with minimal overhead (<3%)
* Tamper-resistant architecture that operates below the application layer

**Secondary Innovations:**
* **Streaming Analysis Framework**: Real-time event processing with pluggable analyzers
* **Semantic Event Extraction**: Automated parsing of agent communications into meaningful events
* **Multi-modal Observability**: Simultaneous monitoring of network traffic, process lifecycle, and file operations
* **Zero-instrumentation Deployment**: No code changes required for agent applications

### 2.2 Research Novelty Assessment

**Comparison with Existing Work:**

| Aspect | Traditional APM | LLM Observability Tools | AgentSight |
|--------|-----------------|-------------------------|------------|
| **Monitoring Level** | Application | Application/Framework | Kernel |
| **Tamper Resistance** | Low | Low | High |
| **Agent Agnostic** | Partial | Framework-specific | Yes |
| **Real-time Analysis** | Limited | Limited | Yes |
| **Overhead** | 5-15% | 10-20% | <3% |
| **Encrypted Traffic** | No | No | Yes (via eBPF) |

**Novel Research Directions Enabled:**

1. **Cognitive Tracing**: Reconstructing agent reasoning from system-level observations
2. **Behavioral Baselining**: Establishing normal agent behavior patterns for anomaly detection
3. **Multi-agent Coordination Analysis**: Understanding inter-agent communication patterns
4. **Emergent Behavior Detection**: Identifying unexpected agent behaviors at scale

### 2.3 Technical Challenges Addressed

**The Semantic Gap Challenge:**
The core challenge lies in the **semantic gap** between kernel-level signals and AI agent behaviors. While eBPF can capture comprehensive system-level data with minimal overhead, translating this into meaningful insights about agent performance requires sophisticated correlation techniques.

**Encrypted Traffic Analysis:**
Most LLM serving uses TLS to communicate with backend servers, and uses SSE to stream responses. Traditional network packet capture tools are insufficient because traffic is encrypted. AgentSight solves this by using eBPF uprobe to hook TLS read and write in userspace, capturing and decrypting traffic.

## 3. Evaluation Framework

### 3.1 Research Questions

**Primary Research Questions:**

1. **RQ1**: How effectively can eBPF-based monitoring capture agent behavior compared to application-level instrumentation?
2. **RQ2**: What is the performance overhead of kernel-level agent monitoring versus traditional approaches?
3. **RQ3**: How tamper-resistant is the system against sophisticated agent attempts to evade monitoring?
4. **RQ4**: Can system-level observations enable semantic understanding of agent behavior?

**Secondary Research Questions:**

1. **RQ5**: How does the system perform across different agent architectures and frameworks?
2. **RQ6**: What novel insights about agent behavior can be discovered through kernel-level monitoring?
3. **RQ7**: How does the approach scale to multi-agent systems and enterprise deployments?

### 3.2 Experimental Design

#### 3.2.1 Comparative Evaluation

**Baseline Comparisons:**
* **LangSmith**: Application-level LLM tracing
* **Helicone**: API-level monitoring
* **OpenTelemetry**: Distributed tracing approach
* **Traditional APM**: New Relic, DataDog for AI applications

**Evaluation Metrics:**
* **Coverage**: Percentage of agent behaviors captured
* **Accuracy**: Precision of event detection and classification
* **Overhead**: CPU, memory, and network impact
* **Tamper Resistance**: Success rate of evasion attempts
* **Latency**: End-to-end monitoring delay

#### 3.2.2 Controlled Experiments

**Experiment 1: Behavior Coverage Analysis**
* Deploy identical agent workloads with different monitoring approaches
* Measure completeness of captured behavior traces
* Analyze missed events and false positives

**Experiment 2: Performance Overhead Assessment**
* Benchmark agent performance with and without monitoring
* Measure resource utilization across different system loads
* Compare overhead against baseline monitoring solutions

**Experiment 3: Tamper Resistance Evaluation**
* Design adversarial agents that attempt to evade monitoring
* Test various evasion techniques (process hiding, network obfuscation, etc.)
* Measure detection success rates

**Experiment 4: Semantic Analysis Capability**
* Implement automated behavior classification using captured events
* Evaluate accuracy of agent intent recognition
* Compare with human-annotated ground truth

#### 3.2.3 Real-world Case Studies

**Case Study 1: Code Generation Agent**
* Monitor Claude Code or similar development assistant
* Analyze patterns in code generation, review, and execution
* Identify potential security risks and optimization opportunities

**Case Study 2: Multi-agent System**
* Deploy multiple interacting agents in a controlled environment
* Monitor coordination patterns and emergent behaviors
* Analyze system-level interactions missed by application monitoring

**Case Study 3: Production Deployment**
* Long-term monitoring of production AI agents
* Incident response and debugging effectiveness
* User experience impact assessment

### 3.3 Evaluation Datasets and Benchmarks

**Synthetic Benchmarks:**
* **AgentBench**: Standardized tasks for agent behavior evaluation
* **MMLU**: Knowledge-intensive tasks for reasoning analysis
* **HumanEval**: Code generation for technical agent monitoring

**Real-world Datasets:**
* **Production agent logs** from enterprise deployments
* **Multi-agent simulation** environments
* **Adversarial agent behaviors** for security testing

**Novel Evaluation Metrics:**
* **Behavioral Completeness Index (BCI)**: Measures percentage of agent decision points captured
* **Tamper Resistance Score (TRS)**: Quantifies system resilience against evasion attempts
* **Semantic Fidelity Metric (SFM)**: Evaluates accuracy of behavior interpretation

## 4. Technical Implementation Evaluation

### 4.1 System Architecture Assessment

**Evaluation Criteria:**

1. **Scalability**: Ability to handle increasing agent populations
2. **Reliability**: System stability under various load conditions
3. **Maintainability**: Ease of extending and modifying the system
4. **Portability**: Cross-platform and kernel version compatibility

**Stress Testing Scenarios:**
* High-frequency agent interactions
* Large-scale multi-agent systems
* Resource-constrained environments
* Adversarial conditions

### 4.2 eBPF Implementation Quality

**Technical Validation:**
* **Code correctness**: Formal verification of eBPF programs
* **Memory safety**: Validation of kernel-space operations
* **Performance optimization**: Efficiency of event collection and filtering
* **Compatibility**: Testing across different kernel versions and architectures

## 5. Impact and Significance Assessment

### 5.1 Scientific Impact

**Theoretical Contributions:**
* Novel application of eBPF to AI agent monitoring
* Framework for kernel-level AI observability
* Methodology for tamper-resistant agent monitoring
* Evaluation framework for AI agent observability systems

**Practical Contributions:**
* Production-ready observability system for AI agents
* Open-source framework for eBPF-based monitoring
* Industry best practices for AI agent deployment
* Security framework for autonomous agent systems

### 5.2 Broader Impact

**Industry Applications:**
* Enterprise AI deployment monitoring
* Regulatory compliance for AI systems
* AI safety and security research
* DevOps tooling for AI applications

**Research Enablement:**
* Platform for AI agent behavior research
* Benchmarking framework for observability systems
* Tool for studying emergent AI behaviors
* Foundation for future AI monitoring research

## 6. Validation and Reproducibility

### 6.1 Reproducibility Framework

**Open Source Components:**
* Complete system implementation on GitHub
* Comprehensive documentation and tutorials
* Automated testing and deployment scripts
* Benchmark datasets and evaluation tools

**Experimental Reproducibility:**
* Detailed experimental protocols
* Containerized evaluation environments
* Standardized performance measurement tools
* Public datasets for validation

### 6.2 Community Validation

**Peer Review Process:**
* Academic conference submission (SOSP, NSDI, OSDI)
* Industry workshop presentations
* Open-source community feedback
* Security research community review

**Validation Studies:**
* Independent reproduction of results
* Third-party security assessments
* Performance validation on different hardware
* Usability studies with practitioners

## 7. Research Paper Structure

### 7.1 Proposed Paper Organization

**Title**: "AgentSight: Tamper-Resistant Observability for AI Agents via eBPF"

**Abstract**:
Problem statement, approach, key results, and contributions in 200 words.

**1. Introduction**
* Problem motivation and significance
* Limitations of current approaches
* Contributions and paper organization

**2. Background and Related Work**
* Traditional software observability
* AI agent monitoring landscape
* eBPF technology overview
* Gap analysis

**3. System Design**
* Architecture overview
* eBPF implementation details
* Streaming analysis framework
* Security considerations

**4. Implementation**
* Core components
* Integration challenges
* Performance optimizations
* Deployment considerations

**5. Evaluation**
* Experimental methodology
* Baseline comparisons
* Performance analysis
* Case studies

**6. Discussion**
* Insights and implications
* Limitations and future work
* Broader impact

**7. Conclusion**
* Summary of contributions
* Future research directions

### 7.2 Key Contributions for Paper

**Primary Contributions:**

1. **Novel technical approach**: First kernel-level AI agent monitoring system
2. **Tamper-resistant architecture**: Security-focused design for autonomous agents
3. **Comprehensive evaluation**: Rigorous assessment across multiple dimensions
4. **Open-source framework**: Enabling reproducible research and adoption

**Secondary Contributions:**

1. **Semantic analysis pipeline**: Automated behavior classification from system events
2. **Multi-agent coordination monitoring**: Novel insights into agent interactions
3. **Performance benchmarking**: Overhead analysis for production deployment
4. **Security validation**: Tamper resistance against adversarial agents

## 8. Limitations and Future Work

### 8.1 Current Limitations

**Technical Limitations:**
* Kernel-level access requirements
* eBPF compatibility constraints
* Limited interpretability of encrypted data
* Scalability bounds for very large deployments

**Methodological Limitations:**
* Evaluation on specific agent types
* Limited real-world deployment data
* Potential performance impacts on older systems
* Privacy considerations for sensitive agent data

### 8.2 Future Research Directions

**Immediate Extensions:**
* Enhanced semantic analysis capabilities
* Automated anomaly detection algorithms
* Integration with existing observability platforms
* Performance optimization for high-scale deployments

**Long-term Research:**
* Formal verification of agent behavior properties
* Predictive modeling of agent behavior patterns
* Integration with AI safety frameworks
* Cross-platform observability standards

## 9. Success Criteria and Metrics

### 9.1 Technical Success Metrics

**Performance Targets:**
* <3% overhead compared to unmonitored systems
* >95% event capture accuracy
* <100ms monitoring latency
* >99% tamper resistance success rate

**Functionality Targets:**
* Support for 5+ major agent frameworks
* Real-time semantic analysis of agent behavior
* Automated anomaly detection with <5% false positive rate
* Scalability to 1000+ concurrent agents

### 9.2 Research Impact Metrics

**Academic Impact:**
* Publication in top-tier systems conference
* Citation by follow-up research
* Adoption by other researchers
* Integration into academic curricula

**Industry Impact:**
* Enterprise adoption for production AI monitoring
* Integration into commercial observability platforms
* Influence on industry standards and best practices
* Security research community recognition

## 10. Conclusion

AgentSight represents a significant advancement in AI agent observability, addressing critical gaps in current monitoring solutions through innovative application of eBPF technology. The proposed evaluation framework provides a comprehensive approach to validating the system's technical contributions, practical impact, and research significance.

The success of this evaluation would establish AgentSight as a foundational technology for trustworthy AI deployment, enabling new research directions in AI agent behavior analysis and contributing to the broader goal of safe and reliable AI systems.

**Key Research Contributions:**

1. **Novel technical approach**: First kernel-level AI agent monitoring system using eBPF
2. **Tamper-resistant architecture**: Security-focused design for autonomous agents
3. **Comprehensive evaluation framework**: Rigorous methodology for assessing AI observability systems
4. **Open-source implementation**: Enabling reproducible research and industry adoption

**Research Significance:**
* Addresses fundamental challenges in AI agent observability
* Enables new research directions in AI safety and security
* Provides practical solution for enterprise AI deployment
* Establishes foundation for future AI monitoring research

This evaluation framework positions AgentSight as a significant contribution to both the systems and AI communities, with clear pathways for validation, reproduction, and impact assessment in the rapidly evolving field of AI agent technology.

---

## Original Content (Preserved)

### AI Agent observability

#### Problem / Gap

1. **"AI Agents are evolve rapidly and different from traditional software"**

The rise of AI-powered agentic systems is transforming modern software infrastructure. Frameworks like AutoGen, LangChain, Claude Code, and gemini-cli orchestrate large language models (LLMs) to automate software engineering tasks, data analysis pipelines, and multi-agent decision-making. Unlike traditional software components that produce deterministic, easily observable behaviors, these AI-agent systems generate open-ended, non-deterministic outputs, often conditioned on hidden internal states and emergent interactions between multiple agents. Consequently, debugging and monitoring agentic software pose unprecedented observability challenges that classic application performance monitoring (APM) tools cannot address adequately.

Why the difference matters for research?

**Instrumentation gap** – Agent logic and algorithm changes daily (new prompts, tools) or by itself at runtime. Relying on in-code hooks means constant churn; kernel-side or side-car tracing stays stable.

**Semantic telemetry** – We need new span attributes ("model.temp", "tool.role", "reasoning.loop_id") and new anomaly detectors (contradiction, persona drift).

**Causal fusion** – Research challenge: merge low-level events with high-level semantic spans into a single timeline so SREs can answer "why my code is not work? what system is it run on and what command have you tried?"

**Tamper resistance** – If prompt-injection turns the agent malicious it may silence its own logs. Out-of-process and kernel level tracing provides an independent audit channel.

In short, AI-agent observability inherits the **unreliable, emergent behaviour** of AI Agents.  Treat the agent runtime as a semi-trusted black box and observe at the system boundary: that's where the and opportunities is.

2. **"Current observability techniques rely on application-level instrumentation"**

Current agent observability techniques rely predominantly on application-level instrumentation—callbacks, middleware hooks, or explicit logging—integrated within each agent framework. While intuitive, this approach suffers three fundamental limitations. First, agent frameworks evolve rapidly, changing prompts, tools, workflow and memory interfaces frequently. They can even modify their self code to create new tools, change prompts and behaviors. Thus, instrumentation embedded within agent codebases incurs significant maintenance overhead. Second, agent runtimes can be tampered with or compromised (e.g., via prompt injection), allowing attackers or buggy behaviors to evade logging entirely.  Fourth, application-level instrumentation cannot reliably capture cross-agent semantics, such as reasoning loops, semantic contradictions, persona shifts, or the behaviors when it's interacting with it's environment, especially when interactions cross process or binary boundaries (e.g., external tools or subprocesses).

For security, consider a llm agent first write a bash file with malicious commands (Not exec, safe), and then exec it with basic tool calls (Often allow it). It  needs system wide observability and constrains.

#### AI Agent observability landscape

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

##### What the landscape tells us

* **Almost everyone hooks at the SDK layer.** 11 of 12 options require you to wrap or proxy function calls. That's fine for proof‑of‑concepts but breaks when an agent hot‑swaps prompts or spawns new tools that bypass the wrapper.
* **OpenTelemetry is becoming the de‑facto wire format.** Traceloop, Honeycomb, Langfuse, PromptLayer, Phoenix (via OpenInference) all speak OTel, which simplifies backend choice.
* **Semantic evaluation is still early.** Only Phoenix, LangSmith, Langfuse, and Literal ship built‑in LLM‑powered quality checks (toxicity, relevance, hallucination score). Most others focus on latency + cost.
* **No one does kernel‑level capture.** None of the listed tools observe encrypted TLS buffers or `execve()` calls directly; they trust the application layer to be honest. That leaves a blind spot for prompt‑injection or self‑modifying agents—exactly the gap a zero‑instrumentation eBPF tracer could close.
* **Specs vs. platforms.** OpenTelemetry GenAI and OpenInference lower the integration tax but don't store or visualize anything; you still need a backend. Conversely, SaaS platforms bundle storage, query, and eval but lock you into their data shape.

##### How this motivates the "boundary tracing" idea

Because today's solutions *mostly* live inside the agent process, they inherit the same fragility as the agent code:

* **Breakage when you tweak the prompt graph** – each new node needs a decorator.
* **Evasion by malicious prompts** – compromised agent can drop or fake logs.
* **Blind to cross‑process side effects** – e.g., writing a shell script then `execve()`‑ing it.

A system‑level eBPF tracer that scoops TLS write buffers and syscalls sidesteps those issues:

| Where today's SDKs stop                            | What boundary tracing would still see |
| -------------------------------------------------- | ------------------------------------- |
| Missing span when agent spawns `curl` directly     | `execve("curl", …)` + network write   |
| Agent mutates its own prompt string before logging | Raw ciphertext leaving the TLS socket |
| Sub‑process mis‑uses GPU                           | `ioctl` + CUDA driver calls           |

In other words, existing tools solve the "what happened inside my code?" story; kernel‑side tracing can answer "what actually hit the wire and the OS?"—a complementary, harder‑to‑tamper vantage point.

That gap is wide open for research and open‑source innovation.

#### **Key Insight and observation**

All meaningful interactions of existing AI-agent system has two clear traverse boundaries:

> AI agent observability must be decoupled from agent internals. **observing from the boundary provides a stable semantic interface**.

##### AI Agent struct

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

#### System build

1. A zero-instrumentation observability tool for AI agent systems built entirely on **system-level tracing (eBPF)** to achieve unified semantic and operational visibility independent of the rapidly-evolving agent runtimes and frameworks.
2. A llm "sidecar" approach to detect subtle semantic anomalies (e.g., reasoning loops, contradictions, persona shifts) together with the system logs.

#### Challenges

The core challenge lies in the **semantic gap** between kernel-level signals and AI agent behaviors. While eBPF can capture comprehensive system-level data with minimal overhead (typically 2-3% CPU usage), translating this into meaningful insights about agent performance requires sophisticated correlation techniques.

Another challenge is capture all prompts and interactions witrh backend server is from encrypted TLS traffic. most llm serving are using TLS to communicate with backend server, and using SSE to stream the response. Using traditional network packet capture tools like tcpdump or wireshark is not enough, because the traffic is encrypted. Proxy the traffic can be a alternative solution, but proxy solutions require explicit configuration changes to route agent traffic through the proxy, which may not work with third party applications or frameworks and can introduce additional latency and complexity. Even if existing eBPF tools can capture the traffic, it lacks support for SSE stream API support.

By using eBPF uprobe to hook the TLS read and write in userspace, we can capture the traffic and decrypt it.

#### References

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
