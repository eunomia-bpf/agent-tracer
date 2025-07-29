# Understand the repo

## env

```
# claude --version
1.0.62 (Claude Code)
```

on https://github.com/eunomia-bpf/bpf-developer-tutorial

## No agentsight

try 1

```
:~/yunwei37/tutorial1# time claude --permission-mode acceptEdits -p /init
Created CLAUDE.md with essential information about the eBPF Developer Tutorial repository, including build commands, architecture overview, and key development patterns.

real    1m54.948s
user    0m4.634s
sys     0m1.169s
:~/yunwei37/tutorial2# time claude --permission-mode acceptEdits -p /init
Created CLAUDE.md with essential information about this eBPF tutorial repository, including common build commands, high-level architecture, and development patterns.

real    2m17.785s
user    0m5.941s
sys     0m1.499s
:~/yunwei37/tutorial4# time claude --permission-mode acceptEdits 
-p /init
I've successfully analyzed the eBPF Developer Tutorial codebase and created a CLAUDE.md file with essential information for future Claude instances working with this repository. The file includes:

1. **Common Development Commands**: Build, run, clean, and test commands that follow a consistent pattern across examples
2. **High-level Architecture**: 
   - Directory structure with numbered tutorial examples
   - eBPF program structure (kernel-side .bpf.c, user-space .c, shared headers)
   - Build system flow using libbpf and bpftool
   - Key concepts like CO-RE and skeleton generation

The CLAUDE.md focuses on practical information needed to work effectively in this codebase without repeating obvious details from the README.

real    2m11.221s
user    0m5.500s
sys     0m1.557s
```

## With agentsight

```
# time claude --permission-mode acceptEdits -p /init
I've successfully created the CLAUDE.md file for this eBPF Developer Tutorial repository. The file includes:

1. **Project overview** - Explaining this is an eBPF tutorial collection with examples in multiple languages
2. **Build commands** - Simple `make` commands for building individual examples 
3. **Repository structure** - Organization of numbered examples under `src/`
4. **Architecture details** - How the build system handles different architectures and key components like BPF maps, CO-RE, and various BPF program types
5. **Development workflow** - The pattern of .bpf.c files for kernel space and .c files for user space

The CLAUDE.md focuses on practical information that future Claude instances will need to be productive in this codebase, avoiding generic advice while highlighting the specific build patterns and architectural decisions unique to this eBPF tutorial repository.

real    1m55.196s
user    0m5.177s
sys     0m1.541s
# time claude --permission-mode acceptEdits -p /init
I've created a CLAUDE.md file that captures the essential information about this eBPF tutorial repository. The file includes:

1. **Project overview** - Explaining it's an eBPF developer tutorial with examples in multiple languages
2. **Architecture** - How the codebase is organized into numbered lessons with different complexity levels
3. **Build commands** - Specific instructions for both libbpf-based and eunomia-bpf based examples
4. **Common development tasks** - Including testing, submodule management, and dependencies
5. **Key technical details** - Important aspects like CO-RE approach and file organization patterns

This will help future Claude instances quickly understand the repository structure and know how to build and test the eBPF examples.

real    1m51.916s
user    0m5.554s
sys     0m1.444s
# time claude --permission-mode acceptEdits -p /init
I've created the CLAUDE.md file with essential information about this eBPF tutorial repository. The file includes:

1. **Common Development Tasks**: How to build and run examples, including the standard make pattern and root privileges requirement
2. **High-level Architecture**: The build system flow (Clang → bpftool → GCC), dependency management, and the CO-RE approach for kernel compatibility

The file is concise and focuses on practical information that requires understanding multiple files, avoiding repetition of easily discoverable details.

real    2m49.884s
user    0m7.979s
sys     0m1.822s
```

## Analysis

The average time for CLAUDE initialization:

* **Without AgentSight**: \~128.0 seconds
* **With AgentSight**: \~132.3 seconds
* **Absolute overhead**: \~4.35 seconds
* **Relative overhead**: \~3.40%

This shows a slight increase in processing time (\~3.4%) when AgentSight is included in the repository.
