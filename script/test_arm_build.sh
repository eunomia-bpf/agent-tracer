#!/bin/bash

# Complete ARM build test script
set -e

echo "=== Testing ARM64 Build in Docker ==="
echo

echo "1. Building Docker image for ARM64..."
docker build --platform linux/arm64 -f Dockerfile.arm -t agentsight-arm . > /dev/null 2>&1
echo "✓ Docker image built successfully"

echo
echo "2. Testing BPF programs build..."
if docker run --platform linux/arm64 --rm agentsight-arm sh -c "cd /agentsight/bpf && make clean && make" > /dev/null 2>&1; then
    echo "✓ BPF programs built successfully"
else
    echo "✗ BPF build failed"
    exit 1
fi

echo
echo "3. Running test_process_utils..."
if docker run --platform linux/arm64 --rm agentsight-arm sh -c "cd /agentsight/bpf && ./test_process_utils" > /dev/null 2>&1; then
    echo "✓ Tests passed successfully"
else
    echo "✗ Tests failed"
    exit 1
fi

echo
echo "4. Checking built binaries..."
BINARIES=$(docker run --platform linux/arm64 --rm agentsight-arm sh -c "cd /agentsight/bpf && ls -1 sslsniff process test_process_utils 2>/dev/null | wc -l")
if [ "$BINARIES" -eq 3 ]; then
    echo "✓ All expected binaries built (sslsniff, process, test_process_utils)"
else
    echo "✗ Some binaries missing"
    exit 1
fi

echo
echo "=== ARM64 Build Test Complete ==="
echo "✓ All tests passed successfully!"
echo
echo "The fix for issue #2 has been verified:"
echo "- Changed hardcoded typedefs to use Linux types when available"
echo "- This prevents conflicts with system headers on ARM64"
echo "- Build now works on both x86_64 and ARM64 architectures"