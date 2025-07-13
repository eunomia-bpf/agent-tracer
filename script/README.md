# SSL Analysis Pipeline

A comprehensive toolkit for analyzing SSL/TLS traffic logs with focus on HTTP headers, metrics, and data flows.

## 🚀 Quick Start

```bash
# Run the complete analysis pipeline
./run_ssl_analysis.sh /path/to/ssl_trace.log

# Output files will be created in script/analysis/
```

## 📁 Components

### 1. `run_ssl_analysis.sh` - Main Pipeline Runner

**Simple bash script that orchestrates the entire analysis workflow**

- **Step 1**: Creates timeline with SSE merging
- **Step 2**: Analyzes headers and metrics
- **Step 3**: Generates clean data timeline
- **Output**: All files organized in `script/analysis/`

### 2. `ssl_log_analyzer.py` - Timeline Generator

**Creates chronological timeline with merged Server-Sent Events (SSE)**

**Features:**

- Parses JSON log entries
- Merges SSE chunks into complete responses
- Creates chronological timeline
- Handles chunked encoding

**Outputs:**

- **Full timeline**: Complete analysis with metadata
- **Simple timeline**: Simplified format like `POST /v1/messages?beta=true HTTP/1.1 host: api.anthropic.com`

### 3. `ssl_header_metrics_analyzer.py` - Header & Metrics Analysis

**Focused analysis of HTTP headers and response metrics**

**New Features:**

- **Request Analysis**: Methods, URL paths, hosts, authorization types
- **Response Analysis**: Status codes, server types, content lengths, encoding
- **Endpoint Patterns**: Per-endpoint header analysis and response times
- **Enhanced Metrics**: Content length statistics, security headers

**Detailed Analysis:**

- HTTP request/response header patterns
- Response time and size metrics
- Communication protocol insights
- Security header analysis
- Per-endpoint performance metrics

### 4. `ssl_data_timeline.py` - Clean Data Timeline

**Generates clean timeline with only actual data transfers**

**Features:**

- **Filters out**: SSL handshake operations, protocol overhead
- **Removes**: OPTIONS/HEAD requests, empty responses, handshake ops
- **Excludes**: HTTP headers (clean data only)
- **Focuses on**: HTTP requests/responses with actual payloads
- **Adds**: Request-response correlation and data flow context

## 📊 Output Files

### `script/analysis/ssl_timeline.json`

**Full timeline with SSE merging**

- Complete request/response data
- Merged SSE content
- All headers and metadata

### `script/analysis/ssl_timeline_simple_timeline.json`

**Simple timeline format**

- Simplified entries like: `POST /v1/messages?beta=true HTTP/1.1 host: api.anthropic.com`
- HTTP response format: `HTTP/1.1 200 OK content-type: text/event-stream`
- Easy to read and process

### `script/analysis/ssl_analysis_report.json`

**Comprehensive header and metrics analysis**

- Request header analysis (methods, paths, hosts, auth types)
- Response header analysis (status codes, servers, content lengths)
- Endpoint pattern analysis with per-endpoint metrics
- Security header analysis
- Performance metrics and statistics

### `script/analysis/ssl_data_only.json`

**Clean data timeline (no headers, no handshake)**

- Only actual HTTP data transfers
- No protocol overhead or handshake operations
- No headers (clean payload data only)
- Request-response correlation
- Data transfer statistics

## 🔧 Manual Usage

### Individual Components

```bash
# Step 1: Generate timeline
python3 ssl_log_analyzer.py input.log -o analysis/timeline --format both

# Step 2: Analyze headers and metrics
python3 ssl_header_metrics_analyzer.py analysis/timeline.json -o analysis/report.json

# Step 3: Generate clean data timeline
python3 ssl_data_timeline.py analysis/timeline.json -o analysis/data_only.json
```

### Advanced Options

```bash
# Quiet mode (suppress debug output)
python3 ssl_log_analyzer.py input.log -q

# Custom output location
python3 ssl_header_metrics_analyzer.py timeline.json -o custom_report.json

# Different timeline formats
python3 ssl_log_analyzer.py input.log --format timeline  # Simple timeline only
python3 ssl_log_analyzer.py input.log --format both     # Both formats
```

## 📈 Analysis Categories

### Header Analysis

- Request methods and URL patterns
- Host and authorization patterns
- User agents and content types
- Security headers
- Server types and encodings

### Metrics Analysis

- Response time statistics (mean, median, p95, p99)
- Content length analysis
- Status code distribution
- Per-endpoint performance metrics

### Data Flow Analysis

- Clean HTTP data transfers only
- Request-response correlation
- Data transfer rates and sizes
- Communication patterns without protocol overhead

## 🎯 Use Cases

1. **Performance Analysis**: Response times, endpoint performance
2. **Security Analysis**: Header patterns, authorization types
3. **API Usage Analysis**: Endpoint popularity, request patterns
4. **Data Flow Analysis**: Clean payload analysis without protocol noise
5. **Troubleshooting**: Timeline analysis with SSE content reconstruction

## 📋 Requirements

- Python 3.6+
- Standard library only (no external dependencies)
- Input: SSL log files in JSON format
- Output: JSON analysis files

## 🔍 Example Analysis Output

### Simple Timeline Entry

```
POST /v1/messages?beta=true HTTP/1.1 host: api.anthropic.com
HTTP/1.1 200 OK content-type: text/event-stream
```

### Header Analysis Summary

```json
{
  "methods": {"POST": 15, "GET": 3},
  "hosts": {"api.anthropic.com": 18},
  "authorization_types": {"Bearer": 15},
  "status_codes": {"200": 16, "201": 2}
}
```

### Clean Data Timeline

```json
{
  "type": "request",
  "method": "POST",
  "path": "/v1/messages",
  "body": "{\"model\":\"claude-3-sonnet\",\"messages\":[...]}",
  "timestamp": 1234567890
}
```
