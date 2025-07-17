#!/bin/bash

# Asset Loading Test Script
# This script tests the asset loading functionality of the collector server

echo "=== AgentSight Asset Loading Test ==="
echo ""

# Check if Python is available
if ! command -v python3 &> /dev/null; then
    echo "Error: python3 is required but not installed"
    exit 1
fi

# Check if requests module is available
if ! python3 -c "import requests" 2>/dev/null; then
    echo "Warning: requests module not found. Installing..."
    pip3 install requests
fi

# Get the directory of this script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "Project root: $PROJECT_ROOT"
echo "Test script: $SCRIPT_DIR/debug_assets.py"
echo ""

# Check if collector server is running
if pgrep -f "collector.*server" >/dev/null; then
    echo "✓ Collector server is running"
else
    echo "⚠ Collector server is not running"
    echo "  Start it with: cd $PROJECT_ROOT/collector && cargo run server"
    echo ""
    
    # Ask if user wants to start the server
    read -p "Start the server now? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "Starting collector server..."
        cd "$PROJECT_ROOT/collector"
        cargo run server &
        SERVER_PID=$!
        echo "Server started with PID: $SERVER_PID"
        echo "Waiting 5 seconds for server to start..."
        sleep 5
    fi
fi

echo ""
echo "Running asset debug test..."
echo ""

# Run the Python debug script
cd "$SCRIPT_DIR"
python3 debug_assets.py "$@"

echo ""
echo "Test completed. Check the results above."
echo "If you see 'Asset not found' errors, try:"
echo "1. cd $PROJECT_ROOT/frontend && npm run build"
echo "2. cd $PROJECT_ROOT/collector && cargo run server"
echo "3. Check http://localhost:8080/timeline"