#!/usr/bin/env python3
"""
Lightweight observability analyzer for SSL JSON logs
Converts HTTP request/response data into spans and traces
"""

import json
import sys
from datetime import datetime
from typing import Dict, List, Optional
import argparse

class SpanAnalyzer:
    def __init__(self, json_file: str):
        self.json_file = json_file
        self.data = None
        self.spans = []
        self.load_data()
    
    def load_data(self):
        """Load JSON data from file"""
        try:
            with open(self.json_file, 'r') as f:
                self.data = json.load(f)
        except Exception as e:
            print(f"Error loading JSON: {e}")
            sys.exit(1)
    
    def nanoseconds_to_ms(self, ns_timestamp: int) -> float:
        """Convert nanosecond timestamp to milliseconds"""
        return ns_timestamp / 1_000_000
    
    def create_span(self, entry: Dict, span_id: str, parent_id: Optional[str] = None) -> Dict:
        """Create OpenTelemetry-style span from log entry"""
        timestamp_ms = self.nanoseconds_to_ms(entry['timestamp'])
        
        span = {
            "traceID": entry.get('tid', 'unknown'),
            "spanID": span_id,
            "parentSpanID": parent_id,
            "operationName": f"{entry.get('method', 'UNKNOWN')} {entry.get('path', '/')}",
            "startTime": int(timestamp_ms * 1000),  # microseconds
            "duration": 0,  # Will be calculated for request/response pairs
            "tags": {
                "http.method": entry.get('method', 'UNKNOWN'),
                "http.url": entry.get('path', '/'),
                "component": "ssl-interceptor",
                "span.kind": "client" if entry['type'] == 'request' else "server"
            },
            "logs": [],
            "process": {
                "serviceName": "agent-tracer",
                "tags": {
                    "tid": str(entry.get('tid', 'unknown'))
                }
            }
        }
        
        # Add body size information
        if 'body' in entry:
            span['tags']['http.request_size'] = len(entry['body'])
        
        # Add JSON body info if available
        if 'json_body' in entry:
            if entry['type'] == 'request' and 'model' in entry['json_body']:
                span['tags']['ai.model'] = entry['json_body']['model']
                span['tags']['ai.provider'] = 'anthropic'
            
        return span
    
    def analyze_spans(self):
        """Convert timeline data into spans"""
        if not self.data or 'data_timeline' not in self.data:
            print("No timeline data found")
            return
        
        timeline = self.data['data_timeline']
        request_spans = {}  # Track requests to match with responses
        
        for i, entry in enumerate(timeline):
            span_id = f"span-{i:04d}"
            
            if entry['type'] == 'request':
                span = self.create_span(entry, span_id)
                request_spans[entry.get('path', '/')] = span
                self.spans.append(span)
                
            elif entry['type'] == 'response':
                # Try to find matching request
                path = entry.get('path', '/')
                if path in request_spans:
                    # Create response span as child of request
                    parent_span = request_spans[path]
                    response_span = self.create_span(entry, f"{span_id}-response", parent_span['spanID'])
                    
                    # Calculate duration between request and response
                    req_time = self.nanoseconds_to_ms(parent_span['startTime'] / 1000)
                    resp_time = self.nanoseconds_to_ms(entry['timestamp'])
                    duration_ms = resp_time - req_time
                    
                    parent_span['duration'] = int(duration_ms * 1000)  # microseconds
                    response_span['duration'] = 1000  # 1ms default for response processing
                    
                    # Add response info to parent span
                    if 'status_code' in entry:
                        parent_span['tags']['http.status_code'] = entry['status_code']
                    
                    self.spans.append(response_span)
                    del request_spans[path]  # Remove processed request
                else:
                    # Orphaned response
                    span = self.create_span(entry, span_id)
                    self.spans.append(span)
    
    def get_summary(self) -> Dict:
        """Generate observability summary"""
        if not self.spans:
            return {}
        
        total_spans = len(self.spans)
        request_spans = [s for s in self.spans if s['tags'].get('span.kind') == 'client']
        response_spans = [s for s in self.spans if s['tags'].get('span.kind') == 'server']
        
        # Calculate latencies
        durations = [s['duration'] for s in request_spans if s['duration'] > 0]
        avg_latency = sum(durations) / len(durations) if durations else 0
        
        # Extract AI model info
        models = set()
        for span in request_spans:
            if 'ai.model' in span['tags']:
                models.add(span['tags']['ai.model'])
        
        return {
            "total_spans": total_spans,
            "request_spans": len(request_spans),
            "response_spans": len(response_spans),
            "average_latency_ms": avg_latency / 1000,  # Convert from microseconds
            "ai_models_used": list(models),
            "trace_ids": list(set(span['traceID'] for span in self.spans))
        }
    
    def export_jaeger_format(self) -> Dict:
        """Export spans in Jaeger JSON format"""
        traces = {}
        
        # Group spans by trace ID
        for span in self.spans:
            trace_id = span['traceID']
            if trace_id not in traces:
                traces[trace_id] = {
                    "traceID": trace_id,
                    "spans": [],
                    "processes": {}
                }
            
            traces[trace_id]['spans'].append(span)
            traces[trace_id]['processes'][span['process']['serviceName']] = span['process']
        
        return {
            "data": list(traces.values()),
            "total": len(traces),
            "limit": 0,
            "offset": 0,
            "errors": None
        }
    
    def print_timeline(self):
        """Print human-readable timeline"""
        print(f"\n📊 SSL Traffic Analysis Summary")
        print(f"{'=' * 50}")
        
        summary = self.get_summary()
        for key, value in summary.items():
            print(f"{key.replace('_', ' ').title()}: {value}")
        
        print(f"\n🔍 Span Timeline:")
        print(f"{'Time (ms)':<15} {'Type':<10} {'Operation':<40} {'Duration (ms)':<12}")
        print("-" * 80)
        
        for span in sorted(self.spans, key=lambda x: x['startTime']):
            start_time = span['startTime'] / 1000  # Convert to ms
            duration = span['duration'] / 1000 if span['duration'] > 0 else 0
            span_type = span['tags']['span.kind']
            operation = span['operationName'][:38]
            
            print(f"{start_time:<15.2f} {span_type:<10} {operation:<40} {duration:<12.2f}")

def main():
    parser = argparse.ArgumentParser(description='Analyze SSL logs for observability')
    parser.add_argument('json_file', help='Path to SSL JSON log file')
    parser.add_argument('--format', choices=['timeline', 'jaeger', 'summary'], 
                       default='timeline', help='Output format')
    parser.add_argument('--output', '-o', help='Output file (default: stdout)')
    
    args = parser.parse_args()
    
    analyzer = SpanAnalyzer(args.json_file)
    analyzer.analyze_spans()
    
    if args.format == 'timeline':
        analyzer.print_timeline()
    elif args.format == 'jaeger':
        result = analyzer.export_jaeger_format()
        output = json.dumps(result, indent=2)
        if args.output:
            with open(args.output, 'w') as f:
                f.write(output)
            print(f"Jaeger traces exported to {args.output}")
        else:
            print(output)
    elif args.format == 'summary':
        summary = analyzer.get_summary()
        print(json.dumps(summary, indent=2))

if __name__ == "__main__":
    main()