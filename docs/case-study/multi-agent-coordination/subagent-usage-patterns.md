# AgentSight Subagent Usage Patterns

This document provides practical workflows and usage patterns for leveraging Claude Code subagents effectively within the AgentSight observability framework.

## Quick Start Guide

### 1. Setting Up Subagents

```bash
# In Claude Code, create each subagent using /agents command
/agents create "eBPF Kernel Engineer" --description "eBPF development specialist for AgentSight"
/agents create "Rust Framework Architect" --description "Streaming framework expert"
/agents create "Frontend Visualization Expert" --description "Next.js and visualization specialist"
/agents create "Security Performance Auditor" --description "Security and performance analysis"
/agents create "Documentation Architecture Reviewer" --description "Technical documentation and architecture"
/agents create "Integration Testing Orchestrator" --description "End-to-end testing and CI/CD"
```

### 2. System Prompt Configuration

Copy the content from each subagent configuration file in `subagents/` directory into the respective subagent's system prompt when creating them.

## Common Workflow Patterns

### Pattern 1: Feature Development Pipeline

**Scenario**: Adding SSL traffic filtering with custom expressions

**Workflow**:
```
1. Documentation Reviewer → Analyze requirements and create specification
   Input: "Analyze the need for custom SSL filtering expressions"
   
2. Rust Architect → Design framework extension
   Input: "Design SSL filter analyzer with expression parsing"
   
3. eBPF Engineer → Implement kernel-side optimizations
   Input: "Optimize sslsniff.bpf.c for filter expression performance"
   
4. Frontend Expert → Create filter configuration UI  
   Input: "Build SSL filter expression builder component"
   
5. Security Auditor → Review security implications
   Input: "Analyze security risks of custom SSL filtering"
   
6. Integration Tester → Validate end-to-end functionality
   Input: "Test SSL filtering with various expression patterns"
   
7. Documentation Reviewer → Update user guides
   Input: "Document SSL filter expression syntax and examples"
```

**Handoff Pattern**:
- Each subagent completes their work and creates summary
- Next subagent reviews previous work before starting
- Security Auditor reviews at multiple checkpoints
- Integration Tester validates after each major component

### Pattern 2: Bug Investigation and Resolution

**Scenario**: Memory leak in collector streaming pipeline

**Workflow**:
```
1. Security Auditor → Assess impact and criticality
   Input: "Analyze memory leak in collector for security implications"
   
2. Rust Architect → Investigate streaming pipeline
   Input: "Debug memory leak in analyzer chain processing"
   
3. eBPF Engineer → Check kernel space resource management
   Input: "Verify eBPF program memory usage and cleanup"
   
4. Integration Tester → Create reproduction scenario
   Input: "Build test case to reproduce memory leak consistently"
   
5. Rust Architect → Implement fix
   Input: "Fix memory leak in streaming analyzer pipeline"
   
6. Integration Tester → Validate fix across scenarios
   Input: "Test memory leak fix with various workloads"
   
7. Documentation Reviewer → Update troubleshooting guide
   Input: "Document memory leak resolution and prevention"
```

### Pattern 3: Performance Optimization Campaign

**Scenario**: Reduce eBPF overhead below 2% target

**Workflow**:
```
1. Security Auditor → Baseline performance measurement
   Input: "Measure current eBPF overhead and identify bottlenecks"
   
2. eBPF Engineer → Optimize kernel programs
   Input: "Reduce sslsniff.bpf.c overhead through algorithm optimization"
   
3. Rust Architect → Optimize data pipeline
   Input: "Optimize streaming framework for lower CPU usage"
   
4. Frontend Expert → Optimize visualization rendering
   Input: "Reduce frontend CPU usage for large datasets"
   
5. Integration Tester → Validate performance across platforms
   Input: "Test performance improvements on x86, arm64, riscv"
   
6. Security Auditor → Verify optimization doesn't compromise security
   Input: "Ensure performance optimizations maintain security posture"
   
7. Documentation Reviewer → Update performance benchmarks
   Input: "Document new performance characteristics and benchmarks"
```

### Pattern 4: Security Enhancement Review

**Scenario**: Comprehensive security audit before release

**Workflow**:
```
1. Security Auditor → Complete security assessment
   Input: "Perform comprehensive security audit of all components"
   
2. eBPF Engineer → Address kernel-space vulnerabilities
   Input: "Fix privilege escalation risks in eBPF programs"
   
3. Rust Architect → Secure data pipeline
   Input: "Implement secure data handling in collector framework"
   
4. Frontend Expert → Secure web interface
   Input: "Add authentication and input validation to frontend"
   
5. Integration Tester → Security testing
   Input: "Test security controls with penetration testing scenarios"
   
6. Documentation Reviewer → Security documentation
   Input: "Create security guide and best practices documentation"
```

## Advanced Coordination Techniques

### 1. Parallel Development with Synchronization Points

```
Time T0: Start parallel development
├── eBPF Engineer: Kernel enhancements
├── Rust Architect: Framework updates  
└── Frontend Expert: UI improvements

Time T1: Synchronization checkpoint
├── Security Auditor: Review all changes
├── Integration Tester: Test integration points
└── Documentation Reviewer: Update specs

Time T2: Continue parallel development with feedback
├── eBPF Engineer: Address security feedback
├── Rust Architect: Fix integration issues
└── Frontend Expert: Implement UI feedback

Time T3: Final validation
└── Integration Tester: End-to-end validation
```

### 2. Context Bridging Between Subagents

**Communication Pattern**:
```
Subagent A → Creates summary document → Subagent B reads summary → Continues work
```

**Example**:
```
eBPF Engineer creates:
---
## SSL Filter Kernel Implementation Summary
- Added new filter expression parser in sslsniff.bpf.c:line 245
- Performance impact: <0.5% overhead measured
- Security considerations: Validated input sanitization
- Integration points: JSON output format updated
---

Rust Architect reads summary and continues:
"Based on the kernel implementation, I'll now integrate the filter expressions into the collector framework..."
```

### 3. Quality Gates and Validation Checkpoints

**Mandatory Checkpoints**:
1. **Security Gate**: Security Auditor must approve before deployment
2. **Integration Gate**: Integration Tester validates all component interactions
3. **Documentation Gate**: Documentation Reviewer ensures completeness
4. **Performance Gate**: Performance benchmarks must meet targets

**Checkpoint Commands**:
```bash
# Security checkpoint
Security Auditor: "Review all changes for security implications"

# Integration checkpoint  
Integration Tester: "Run full test suite: make test && cargo test && npm run build"

# Documentation checkpoint
Documentation Reviewer: "Verify all changes are documented with examples"

# Performance checkpoint
Security Auditor: "Validate performance meets <3% eBPF overhead target"
```

## Subagent-Specific Usage Patterns

### eBPF Kernel Engineer Patterns

**Common Tasks**:
- New eBPF program development
- Kernel compatibility fixes
- Performance optimization
- Memory management improvements

**Typical Workflow**:
```
1. Analyze kernel requirements
2. Implement eBPF program changes
3. Test with different kernel versions
4. Optimize for performance
5. Validate security implications
6. Document kernel-specific considerations
```

**Key Commands**:
```bash
cd bpf/
make build
make test
sudo ./sslsniff -p 8080  # Test execution
```

### Rust Framework Architect Patterns

**Common Tasks**:
- Framework architecture design
- Analyzer implementation
- Runner optimization
- Event pipeline design

**Typical Workflow**:
```
1. Design framework extensions
2. Implement analyzer/runner changes
3. Test streaming performance
4. Validate async behavior
5. Optimize resource usage
6. Document framework patterns
```

**Key Commands**:
```bash
cd collector/
cargo check
cargo test
cargo run trace --ssl --process
```

### Frontend Visualization Expert Patterns

**Common Tasks**:
- Timeline component development
- Real-time data visualization
- UI/UX improvements
- TypeScript interface design

**Typical Workflow**:
```
1. Design UI component architecture
2. Implement visualization components
3. Test with real data
4. Optimize rendering performance
5. Validate responsive design
6. Document component usage
```

**Key Commands**:
```bash
cd frontend/
npm run lint
npm run build
npm run dev  # Development server
```

### Security & Performance Auditor Patterns

**Common Tasks**:
- Vulnerability assessment
- Performance analysis
- Code review
- Compliance validation

**Typical Workflow**:
```
1. Analyze code for security issues
2. Measure performance characteristics
3. Identify optimization opportunities
4. Review privilege usage
5. Validate data protection
6. Document findings and recommendations
```

**Analysis Focus**:
- Read-only code analysis
- Performance measurement
- Security vulnerability identification
- Compliance requirement validation

## Troubleshooting Subagent Coordination

### Common Issues

**1. Context Loss Between Subagents**
- **Symptom**: Subagent doesn't understand previous work
- **Solution**: Create explicit handoff summaries
- **Prevention**: Use structured communication templates

**2. Conflicting Changes**
- **Symptom**: Subagents make incompatible modifications
- **Solution**: Use clear synchronization checkpoints
- **Prevention**: Define clear boundaries and interfaces

**3. Tool Permission Issues**
- **Symptom**: Subagent cannot access required tools
- **Solution**: Verify tool configuration for each subagent
- **Prevention**: Test tool access during subagent setup

**4. Performance Degradation**
- **Symptom**: Multiple subagents slow down development
- **Solution**: Use parallel workflows with sync points
- **Prevention**: Optimize subagent usage patterns

### Best Practices for Coordination

**1. Clear Role Definition**
- Each subagent has specific, non-overlapping responsibilities
- Define handoff points explicitly
- Create accountability for deliverables

**2. Structured Communication**
- Use standardized summary formats
- Include context for next subagent
- Document decisions and rationale

**3. Regular Synchronization** 
- Schedule integration checkpoints
- Validate assumptions between subagents
- Resolve conflicts early in process

**4. Quality Assurance**
- Security review for all changes
- Integration testing for component interactions
- Documentation updates for all modifications

## Measuring Subagent Effectiveness

### Key Performance Indicators (KPIs)

**Development Velocity**:
- Features delivered per sprint
- Bug resolution time
- Code review completion time

**Quality Metrics**:
- Security vulnerabilities detected
- Integration test pass rate
- Documentation completeness

**Coordination Efficiency**:
- Handoff time between subagents
- Context preservation success rate
- Rework due to miscommunication

### Success Metrics

**Target Performance**:
- 50% reduction in development time for complex features
- 90% fewer integration issues
- 100% security review coverage
- 95% documentation completeness

This comprehensive usage pattern guide ensures effective coordination of Claude Code subagents within the AgentSight observability framework, maximizing productivity while maintaining high code quality and security standards.