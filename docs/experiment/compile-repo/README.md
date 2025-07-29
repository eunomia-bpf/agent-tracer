# compile repo



## env

```
# claude --version
1.0.62 (Claude Code)
```

on https://github.com/eunomia-bpf/bpf-developer-tutorial

time git submodule update --init --recursive && cd src && make -j8

## Without agentsight

```

```

## With agentsight

```
python3 /root/yunwei37/build_benchmark.py
=== BPF Developer Tutorial Build Benchmark ===

Phase 1: Cloning repositories...
Cloning repository to /root/yunwei37/bpf-tutorial-1...
Successfully cloned to /root/yunwei37/bpf-tutorial-1
Cloning repository to /root/yunwei37/bpf-tutorial-2...
Successfully cloned to /root/yunwei37/bpf-tutorial-2
Cloning repository to /root/yunwei37/bpf-tutorial-3...
Successfully cloned to /root/yunwei37/bpf-tutorial-3

Phase 1 completed: All repositories cloned.

Phase 2: Building repositories and measuring time...

Build 1/3:
Building in /root/yunwei37/bpf-tutorial-1...
Build completed in 90.54 seconds

Build 2/3:
Building in /root/yunwei37/bpf-tutorial-2...
Build completed in 91.53 seconds

Build 3/3:
Building in /root/yunwei37/bpf-tutorial-3...
Build completed in 89.43 seconds

Phase 2 completed: All builds finished.

=== Build Time Results ===
Build 1: 90.54 seconds
Build 2: 91.53 seconds
Build 3: 89.43 seconds

Average build time: 90.50 seconds
Standard deviation: 1.05 seconds
Min time: 89.43 seconds
Max time: 91.53 seconds

Phase 3: Cleaning up...
Cleaning up /root/yunwei37/bpf-tutorial-1...
Removed /root/yunwei37/bpf-tutorial-1
Cleaning up /root/yunwei37/bpf-tutorial-2...
Removed /root/yunwei37/bpf-tutorial-2
Cleaning up /root/yunwei37/bpf-tutorial-3...
Removed /root/yunwei37/bpf-tutorial-3

Phase 3 completed: All directories cleaned up.

=== Benchmark completed ===
(base) root@gpu01:~/yunwei37# 
```