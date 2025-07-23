Here's a complete and formal introduction and abstract for your paper, incorporating the AIOps narrative with the distinct role of AI Agents as ML-powered system components deeply embedded in modern software infrastructures:

---

### Abstract

The rise of AI-driven agentic systems, such as those powered by large language models (LLMs), is reshaping modern software infrastructure, especially in software engineering and operations management. Unlike traditional deterministic applications, these AI agents dynamically generate outputs based on internal reasoning, transient memory states, and emergent interactions between multiple sub-agents and tools. They are no longer mere business logic consumers, but have become intelligent, active participants directly influencing system operations, resource control, and decision-making processes. Consequently, observing and managing AI-agent behaviors poses novel observability challenges that exceed the capabilities of classical application performance monitoring (APM) tools.

Current AI-agent monitoring practices predominantly rely on in-process instrumentation, leading to severe maintenance overhead due to frequent internal changes, and vulnerability to tampering (e.g., via prompt injection attacks). Moreover, they lack visibility into cross-process interactions, external tool usage, and subtle semantic anomalies such as reasoning loops and persona shifts.

To bridge this gap, we propose **AgentSight**, an eBPF-based kernel-boundary observability approach specifically tailored for AI-driven agents. AgentSight captures semantic-rich events directly from system-level boundaries (including TLS traffic, system calls, subprocess executions) without requiring explicit instrumentation within agent codebases. This approach integrates seamlessly into existing AIOps frameworks, providing stable, tamper-resistant insights essential for automated anomaly detection, root-cause analysis, and operational remediation of ML-driven infrastructure components.

Our preliminary evaluation shows that AgentSight achieves comprehensive agent-level visibility with minimal performance overhead (2–3% CPU), enabling detection of real-world anomalies, including silent prompt injections, unauthorized external tool usage, and emergent reasoning errors, thus significantly enhancing AIOps workflows for modern software environments.

---

### Introduction

In recent years, the integration of Machine Learning (ML) into core software infrastructures has transitioned from isolated analytic tasks to embedded decision-making roles, transforming software components into intelligent actors actively influencing infrastructure operations. Among these ML-powered systems, AI agents driven by large language models (LLMs)—such as LangChain, AutoGen, Claude Code, and Gemini-cli—are rapidly gaining traction, especially in domains like automated software engineering, data pipelines, and multi-agent workflows.

Unlike traditional software services, whose deterministic behaviors are relatively straightforward to monitor and debug, AI agents produce non-deterministic, context-sensitive outcomes. Their behaviors are dynamically shaped by internal states, evolving prompts, reasoning strategies, and spontaneous interactions with external tools. These agents thus represent a new category of system-level ML components, no longer just passive business logic consumers but active participants directly impacting system resources, operation workflows, and control decisions. Consequently, monitoring, understanding, and governing such ML-driven system components represents a critical, yet largely unexplored, operational challenge within real-world software production, particularly in contexts involving complex coding tasks, software-generation workflows, and automated operational management.

Traditional Application Performance Monitoring (APM) tools rely heavily on deterministic instrumentation points embedded directly within application codebases. Metrics typically focus on resource utilization, response latency, and explicit error conditions. However, when applied to AI-driven agents, these classical approaches become fundamentally inadequate. Agent systems introduce semantic observability dimensions—prompt correctness, reasoning coherence, semantic drift, toxicity, and financial cost—that traditional monitoring tools cannot naturally capture. The inherently open-ended output quality, diverse acceptable results, and subtle semantic failures (e.g., infinite reasoning loops or unauthorized side-effects through subprocesses) complicate monitoring further.

Current agent observability solutions predominantly rely on in-code instrumentation, involving middleware hooks, SDK wrappers, or explicit logging. Although intuitive, this strategy suffers three major limitations:

1. **Maintenance Fragility:** Agent frameworks frequently update internal prompts, tools, and even self-modify at runtime. Static instrumentation thus rapidly becomes obsolete, requiring significant ongoing maintenance.

2. **Tamper Vulnerability:** A compromised or maliciously manipulated agent (e.g., via prompt injection attacks) can evade or disable internal logging mechanisms, eliminating trustworthiness of logs.

3. **Cross-Process Semantic Gaps:** Application-level instrumentation cannot reliably capture interactions across subprocess boundaries or subtle semantic anomalies, significantly weakening debugging and auditing capabilities.

We argue that a paradigm shift toward kernel-boundary observability is essential to overcome these limitations. To this end, we introduce **AgentSight**, an innovative zero-instrumentation monitoring framework using eBPF technology. AgentSight captures rich semantic information directly from stable system boundaries—such as encrypted TLS communication buffers, system calls, and subprocess invocations—ensuring stable, comprehensive, and tamper-resistant monitoring of AI-agent behaviors. By leveraging the kernel-level observability layer, AgentSight provides a unified and trustworthy data stream that integrates naturally with modern AIOps pipelines, enabling automated detection of anomalies (including semantic drift, reasoning loops, unauthorized operations), streamlined root-cause analysis, and proactive operational management.

Through AgentSight, operational teams can correlate low-level system events (e.g., unauthorized curl commands, subprocesses execution) with high-level semantic behaviors (e.g., changes in prompt content or hidden reasoning errors). Such integration significantly enhances the effectiveness of existing AIOps practices, empowering organizations to confidently deploy and manage ML-driven agentic components within their production environments.

In summary, AgentSight addresses critical gaps in AI-agent observability by shifting the observation point from the rapidly-evolving internal agent codebase to the stable system boundary. This innovative approach not only solves immediate operational challenges but also opens new research avenues for robust, intelligent, and automated management of modern, ML-powered software infrastructures.
