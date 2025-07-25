# OSDI Review for "AgentSight: System-Level Observability for AI Agents Using eBPF"

## Summary

This paper presents AgentSight, a system-level observability framework for AI agents that uses eBPF to monitor agent behavior at system boundaries (kernel and network interfaces) rather than within application code. The key insight is "boundary tracing"—observing agents where they interact with the system at stable interfaces that cannot be bypassed or modified by the agents themselves. The authors implement this approach to capture both LLM interactions (via TLS interception) and system behavior (via syscall monitoring) with <3% overhead.

## Strengths

### S1: Compelling Problem Framing

The paper excellently motivates why AI agent observability is fundamentally different from traditional software monitoring. The comparison table in Section 2.3 effectively contrasts deterministic software with non-deterministic AI agents across multiple dimensions. The insight that agents behave "more like users than programs" is powerful and well-articulated.

### S2: Novel "Boundary Tracing" Concept

The core contribution—observing at system boundaries rather than within applications—is elegant and well-justified. The authors make a convincing case that while agent frameworks evolve rapidly, system interfaces (POSIX syscalls, TLS protocols) remain stable. This architectural insight addresses a real pain point in the rapidly evolving AI agent ecosystem.

### S3: Comprehensive Technical Approach

The dual-perspective monitoring (network + kernel boundaries) provides semantic completeness that neither approach alone could achieve. The paper shows how correlating LLM interactions with system actions enables understanding agent behavior across abstraction levels.

### S4: Strong Survey of Existing Solutions

The landscape analysis in Section 3.2 (table of 12 existing solutions) provides valuable context and clearly identifies gaps in current approaches. The systematic analysis of why these solutions fail for AI agents strengthens the motivation.

### S5: Practical Implementation

The paper goes beyond concept to implementation, with concrete code examples and a working open-source system. The case studies demonstrate real detection capabilities for prompt injection, reasoning loops, and multi-agent coordination.

## Weaknesses

### W1: Limited Discussion of Privacy/Security Implications

While the paper mentions that AgentSight can capture "potentially sensitive data," there's insufficient discussion of the security implications of intercepting all LLM communications and system calls. In production environments, this could expose API keys, personal data, or proprietary prompts. The paper needs a dedicated section on privacy-preserving techniques or deployment considerations.

### W2: Scalability Story Unclear

The evaluation focuses on single-machine deployments with individual agents. However, production AI systems often involve multiple agents across distributed systems. How does AgentSight scale to hundreds of agents across multiple machines? The correlation engine design seems to assume local state—how would this work in distributed settings?

### W3: Semantic Gap Still Partially Unaddressed

While the paper claims to bridge the semantic gap between system events and agent intentions, the semantic analysis (Section 5.5.4) feels underdeveloped. The "multi-layer analysis approach" is described abstractly without concrete algorithms or evaluation of its effectiveness. How accurately can the system actually determine agent intent from observed behaviors?

### W4: Missing Comparison with Hybrid Approaches

The paper presents boundary tracing as superior to application-level instrumentation but doesn't explore hybrid approaches. Could combining lightweight application hooks with system-level monitoring provide better semantic understanding while maintaining stability? This seems like a missed opportunity.

### W5: Insufficient Discussion of Limitations

The paper briefly mentions challenges but doesn't adequately discuss fundamental limitations. For example:
- Can boundary tracing detect issues in agent reasoning that don't manifest as system actions?
- How does it handle agents that primarily operate on in-memory data?
- What about agents using local LLMs where no network boundary exists?

## Detailed Comments

### Story and Narrative Flow

The paper tells a compelling story: AI agents are different → existing monitoring fails → observe at system boundaries → implement with eBPF → achieve comprehensive visibility. The progression is logical and well-paced. However, the narrative could be strengthened by:

1. **Earlier introduction of boundary tracing**: Currently not introduced until Section 4. Consider foreshadowing this concept in the introduction.

2. **Better integration of eBPF**: Section 6 feels somewhat disconnected. Consider integrating eBPF details within the boundary tracing discussion (Section 4) to maintain narrative flow.

3. **Clearer contribution statement**: While the contributions are listed, they could be more crisply stated. What exactly is novel beyond using eBPF for AI agents?

### Technical Depth vs. Accessibility

The paper strikes a reasonable balance, but some sections could be clearer:
- The eBPF code examples are helpful but could use more explanation for readers unfamiliar with eBPF
- The streaming analysis framework (Section 5.3) introduces many components without sufficient context

### Missing Related Work

The paper should discuss:
- Runtime verification systems that use system-level monitoring
- Work on anomaly detection in distributed systems
- Privacy-preserving monitoring techniques

## Questions for Authors

1. How does AgentSight handle agents that use local LLMs or direct model APIs that bypass standard TLS libraries?

2. What is the false positive rate for the anomaly detection? The case studies show successful detection but don't discuss false alarms.

3. How do you envision AgentSight being deployed in production? As a debugging tool, security monitor, or compliance system?

4. Could adversarial agents deliberately generate chaff system calls to overwhelm the monitoring system?

5. How does the correlation engine handle time skew between different observation points?

## Minor Comments

- Figure in Section 4.2 could be clearer—consider adding data flow arrows
- Some citations are missing publication years
- The conclusion could be more forward-looking about the implications for AI system design

## Recommendation

**Score: Weak Accept**

This paper addresses a timely and important problem with a novel approach. The "boundary tracing" concept is insightful and the implementation demonstrates feasibility. However, the paper needs to better address scalability, privacy concerns, and fundamental limitations. The semantic analysis components feel underdeveloped compared to the systems contributions.

The work would benefit from:
1. Expanded evaluation on distributed multi-agent systems
2. Deeper treatment of privacy/security implications  
3. More rigorous evaluation of semantic understanding capabilities
4. Discussion of deployment experiences or user studies

Despite these limitations, the paper makes solid contributions to an important emerging area. The open-source release and comprehensive survey of existing solutions provide value to the community. With revisions addressing the major concerns, this could be a strong addition to OSDI.

## Suitability for OSDI

The paper fits OSDI's scope as it presents a systems approach to a significant problem in modern computing. The use of eBPF for AI observability is novel and the implementation challenges are non-trivial. However, the evaluation could be more comprehensive for a top-tier systems venue—particularly around scalability and production deployment experiences.