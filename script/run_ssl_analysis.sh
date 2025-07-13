#!/bin/bash

# SSL Analysis Runner Script
# Usage: ./run_ssl_analysis.sh <ssl_log_file>

set -e

# Check if log file is provided
if [ $# -eq 0 ]; then
    echo "Usage: $0 <ssl_log_file>"
    echo "Example: $0 ../ssl_trace.log"
    exit 1
fi

LOG_FILE="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ANALYSIS_DIR="$SCRIPT_DIR/analysis"

# Create analysis directory if it doesn't exist
mkdir -p "$ANALYSIS_DIR"

echo "=== SSL Analysis Pipeline ==="
echo "Input file: $LOG_FILE"
echo "Output directory: $ANALYSIS_DIR"
echo ""

# Step 1: Run basic SSL log analyzer
echo "Step 1: Running SSL log analyzer..."
python3 "$SCRIPT_DIR/ssl_log_analyzer.py" "$LOG_FILE" -o "$ANALYSIS_DIR/ssl_timeline" --format both
echo "✓ Timeline analysis complete"

# Step 2: Run deep analysis (headers and metrics)
echo "Step 2: Running header and metrics analysis..."
python3 "$SCRIPT_DIR/ssl_header_metrics_analyzer.py" "$ANALYSIS_DIR/ssl_timeline.json" -o "$ANALYSIS_DIR/ssl_analysis_report.json"
echo "✓ Header and metrics analysis complete"

# Step 3: Generate data timeline without handshake
echo "Step 3: Generating clean data timeline..."
python3 "$SCRIPT_DIR/ssl_data_timeline.py" "$ANALYSIS_DIR/ssl_timeline.json" -o "$ANALYSIS_DIR/ssl_data_only.json"
echo "✓ Data timeline generation complete"

echo ""
echo "=== Analysis Complete ==="
echo "Output files:"
echo "  - Full timeline: $ANALYSIS_DIR/ssl_timeline.json"
echo "  - Simple timeline: $ANALYSIS_DIR/ssl_timeline_simple_timeline.json"
echo "  - Header & metrics report: $ANALYSIS_DIR/ssl_analysis_report.json"
echo "  - Clean data timeline (no headers): $ANALYSIS_DIR/ssl_data_only.json" 