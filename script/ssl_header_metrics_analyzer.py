#!/usr/bin/env python3
"""
SSL Header and Metrics Analyzer - Focused analysis of HTTP headers and response metrics

Provides targeted analysis including:
1. HTTP header analysis and patterns
2. Response time metrics and performance
3. Content type and size analysis
4. Communication protocol insights
5. Status code distribution
6. Request/response correlation metrics
"""

import json
import sys
import argparse
import os
import re
from collections import defaultdict, Counter, OrderedDict
from typing import Dict, List, Any, Optional, Tuple
from datetime import datetime
import statistics
import math

class SSLHeaderMetricsAnalyzer:
    def __init__(self, timeline_file: str, quiet: bool = False):
        self.timeline_file = timeline_file
        self.quiet = quiet
        self.timeline_data = None
        self.requests = []
        self.responses = []
        
    def debug_print(self, message: str):
        """Print debug message only if not in quiet mode"""
        if not self.quiet:
            print(message)
            
    def load_timeline_data(self):
        """Load timeline data from JSON file"""
        self.debug_print(f"Loading timeline data from: {self.timeline_file}")
        
        with open(self.timeline_file, 'r') as f:
            self.timeline_data = json.load(f)
            
        timeline = self.timeline_data.get('timeline', [])
        
        # Categorize entries
        for entry in timeline:
            entry_type = entry.get('type')
            if entry_type == 'request':
                self.requests.append(entry)
            elif entry_type == 'response':
                self.responses.append(entry)
        
        self.debug_print(f"Loaded {len(timeline)} timeline entries")
        self.debug_print(f"  - Requests: {len(self.requests)}")
        self.debug_print(f"  - Responses: {len(self.responses)}")
        
    def analyze_request_headers(self) -> Dict[str, Any]:
        """Analyze HTTP request headers"""
        self.debug_print("Analyzing request headers...")
        
        header_stats = defaultdict(Counter)
        common_headers = Counter()
        user_agents = Counter()
        content_types = Counter()
        methods = Counter()
        url_paths = Counter()
        hosts = Counter()
        authorization_types = Counter()
        
        for req in self.requests:
            headers = req.get('headers', {})
            method = req.get('method', 'UNKNOWN')
            path = req.get('path', '/')
            
            # Count methods and paths
            methods[method] += 1
            url_paths[path] += 1
            
            # Count header frequency
            for header_name, header_value in headers.items():
                header_name_lower = header_name.lower()
                common_headers[header_name_lower] += 1
                header_stats[header_name_lower][header_value] += 1
                
                # Special handling for specific headers
                if header_name_lower == 'user-agent':
                    user_agents[header_value] += 1
                elif header_name_lower == 'content-type':
                    content_types[header_value] += 1
                elif header_name_lower == 'host':
                    hosts[header_value] += 1
                elif header_name_lower == 'authorization':
                    # Extract authorization type (Bearer, Basic, etc.)
                    auth_type = header_value.split(' ')[0] if ' ' in header_value else header_value
                    authorization_types[auth_type] += 1
        
        # Convert to serializable format
        header_stats_dict = {}
        for header, values in header_stats.items():
            header_stats_dict[header] = dict(values.most_common(10))
        
        return {
            "total_requests": len(self.requests),
            "unique_headers": len(common_headers),
            "most_common_headers": dict(common_headers.most_common(20)),
            "methods": dict(methods.most_common(10)),
            "url_paths": dict(url_paths.most_common(20)),
            "hosts": dict(hosts.most_common(10)),
            "user_agents": dict(user_agents.most_common(10)),
            "content_types": dict(content_types.most_common(10)),
            "authorization_types": dict(authorization_types.most_common(10)),
            "header_value_distribution": header_stats_dict
        }
        
    def analyze_response_headers(self) -> Dict[str, Any]:
        """Analyze HTTP response headers"""
        self.debug_print("Analyzing response headers...")
        
        header_stats = defaultdict(Counter)
        common_headers = Counter()
        content_types = Counter()
        cache_headers = Counter()
        security_headers = Counter()
        status_codes = Counter()
        server_types = Counter()
        content_lengths = []
        encoding_types = Counter()
        
        security_header_names = [
            'strict-transport-security', 'content-security-policy',
            'x-frame-options', 'x-content-type-options',
            'x-xss-protection', 'referrer-policy'
        ]
        
        for resp in self.responses:
            headers = resp.get('headers', {})
            status_code = resp.get('status_code', 0)
            
            # Count status codes
            status_codes[status_code] += 1
            
            # Count header frequency
            for header_name, header_value in headers.items():
                header_name_lower = header_name.lower()
                common_headers[header_name_lower] += 1
                header_stats[header_name_lower][header_value] += 1
                
                # Special handling for specific headers
                if header_name_lower == 'content-type':
                    content_types[header_value] += 1
                elif header_name_lower in ['cache-control', 'expires', 'etag']:
                    cache_headers[f"{header_name_lower}: {header_value}"] += 1
                elif header_name_lower in security_header_names:
                    security_headers[header_name_lower] += 1
                elif header_name_lower == 'server':
                    server_types[header_value] += 1
                elif header_name_lower == 'content-length':
                    try:
                        content_lengths.append(int(header_value))
                    except ValueError:
                        pass
                elif header_name_lower == 'content-encoding':
                    encoding_types[header_value] += 1
        
        # Convert to serializable format
        header_stats_dict = {}
        for header, values in header_stats.items():
            header_stats_dict[header] = dict(values.most_common(10))
        
        # Calculate content length statistics
        content_length_stats = {}
        if content_lengths:
            content_length_stats = {
                'count': len(content_lengths),
                'min': min(content_lengths),
                'max': max(content_lengths),
                'mean': sum(content_lengths) / len(content_lengths),
                'total': sum(content_lengths)
            }
        
        return {
            "total_responses": len(self.responses),
            "unique_headers": len(common_headers),
            "most_common_headers": dict(common_headers.most_common(20)),
            "status_codes": dict(status_codes.most_common(10)),
            "content_types": dict(content_types.most_common(10)),
            "server_types": dict(server_types.most_common(10)),
            "encoding_types": dict(encoding_types.most_common(10)),
            "cache_headers": dict(cache_headers.most_common(10)),
            "security_headers": dict(security_headers),
            "content_length_stats": content_length_stats,
            "header_value_distribution": header_stats_dict
        }
        
    def analyze_response_metrics(self) -> Dict[str, Any]:
        """Analyze response time and size metrics"""
        self.debug_print("Analyzing response metrics...")
        
        # Build request-response correlation
        request_response_map = {}
        for req in self.requests:
            tid = req.get('tid')
            if tid:
                request_response_map[tid] = {'request': req}
        
        for resp in self.responses:
            tid = resp.get('tid')
            if tid in request_response_map:
                request_response_map[tid]['response'] = resp
        
        # Calculate metrics
        response_times = []
        response_sizes = []
        status_codes = Counter()
        
        for tid, pair in request_response_map.items():
            if 'request' in pair and 'response' in pair:
                req = pair['request']
                resp = pair['response']
                
                # Response time
                req_time = req.get('timestamp', 0)
                resp_time = resp.get('timestamp', 0)
                if resp_time > req_time:
                    response_time_ms = (resp_time - req_time) / 1_000_000
                    response_times.append(response_time_ms)
                
                # Response size
                headers = resp.get('headers', {})
                if 'content-length' in headers:
                    try:
                        size = int(headers['content-length'])
                        response_sizes.append(size)
                    except ValueError:
                        pass
                
                # Status codes
                status_code = resp.get('status_code', 0)
                status_codes[status_code] += 1
        
        # Calculate statistics
        metrics = {
            "total_request_response_pairs": len(request_response_map),
            "successful_correlations": len([p for p in request_response_map.values() if 'response' in p]),
            "status_code_distribution": dict(status_codes)
        }
        
        if response_times:
            metrics['response_time_ms'] = {
                'count': len(response_times),
                'mean': statistics.mean(response_times),
                'median': statistics.median(response_times),
                'min': min(response_times),
                'max': max(response_times),
                'stddev': statistics.stdev(response_times) if len(response_times) > 1 else 0,
                'p95': self._percentile(response_times, 95),
                'p99': self._percentile(response_times, 99)
            }
        
        if response_sizes:
            metrics['response_size_bytes'] = {
                'count': len(response_sizes),
                'mean': statistics.mean(response_sizes),
                'median': statistics.median(response_sizes),
                'min': min(response_sizes),
                'max': max(response_sizes),
                'total': sum(response_sizes)
            }
        
        return metrics
        
    def analyze_communication_patterns(self) -> Dict[str, Any]:
        """Analyze communication patterns and protocol usage"""
        self.debug_print("Analyzing communication patterns...")
        
        methods = Counter()
        protocols = Counter()
        paths = Counter()
        
        # Request patterns
        for req in self.requests:
            method = req.get('method', 'unknown')
            protocol = req.get('protocol', 'unknown')
            path = req.get('path', 'unknown')
            
            methods[method] += 1
            protocols[protocol] += 1
            
            # Normalize paths for pattern analysis
            normalized_path = self._normalize_path(path)
            paths[normalized_path] += 1
        
        # Response patterns
        content_types = Counter()
        sse_responses = 0
        
        for resp in self.responses:
            headers = resp.get('headers', {})
            content_type = headers.get('content-type', 'unknown')
            content_types[content_type] += 1
            
            if 'text/event-stream' in content_type:
                sse_responses += 1
        
        return {
            "request_methods": dict(methods.most_common(10)),
            "protocols": dict(protocols),
            "top_paths": dict(paths.most_common(20)),
            "response_content_types": dict(content_types.most_common(10)),
            "sse_responses": sse_responses,
            "communication_summary": {
                "unique_methods": len(methods),
                "unique_protocols": len(protocols),
                "unique_paths": len(paths),
                "unique_content_types": len(content_types)
            }
        }
        
    def _normalize_path(self, path: str) -> str:
        """Normalize paths for pattern analysis"""
        # Remove query parameters
        if '?' in path:
            path = path.split('?')[0]
        
        # Replace UUIDs and IDs with placeholders
        path = re.sub(r'/[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}', '/{{uuid}}', path)
        path = re.sub(r'/\d+', '/{{id}}', path)
        path = re.sub(r'/[a-f0-9]{24,}', '/{{hash}}', path)
        
        return path
        
    def _percentile(self, data: List[float], percentile: float) -> float:
        """Calculate percentile of a dataset"""
        sorted_data = sorted(data)
        k = (len(sorted_data) - 1) * percentile / 100
        f = math.floor(k)
        c = math.ceil(k)
        if f == c:
            return sorted_data[int(k)]
        return sorted_data[int(f)] * (c - k) + sorted_data[int(c)] * (k - f)
        
    def analyze_content_analysis(self) -> Dict[str, Any]:
        """Analyze request and response content"""
        self.debug_print("Analyzing content patterns...")
        
        request_content = {
            "requests_with_body": 0,
            "json_requests": 0,
            "form_requests": 0,
            "total_request_size": 0
        }
        
        response_content = {
            "responses_with_body": 0,
            "json_responses": 0,
            "html_responses": 0,
            "sse_responses": 0,
            "total_response_size": 0
        }
        
        # Analyze request content
        for req in self.requests:
            if req.get('body'):
                request_content["requests_with_body"] += 1
                body_size = len(req['body'])
                request_content["total_request_size"] += body_size
                
                if req.get('json_body'):
                    request_content["json_requests"] += 1
                    
                headers = req.get('headers', {})
                content_type = headers.get('content-type', '')
                if 'application/x-www-form-urlencoded' in content_type:
                    request_content["form_requests"] += 1
        
        # Analyze response content  
        for resp in self.responses:
            if resp.get('body'):
                response_content["responses_with_body"] += 1
                body_size = len(resp['body'])
                response_content["total_response_size"] += body_size
                
                if resp.get('json_body'):
                    response_content["json_responses"] += 1
                    
                headers = resp.get('headers', {})
                content_type = headers.get('content-type', '')
                if 'text/html' in content_type:
                    response_content["html_responses"] += 1
                elif 'text/event-stream' in content_type:
                    response_content["sse_responses"] += 1
        
        return {
            "request_content": request_content,
            "response_content": response_content
        }
        
    def analyze_endpoint_patterns(self) -> Dict[str, Any]:
        """Analyze endpoint access patterns and request/response correlation"""
        self.debug_print("Analyzing endpoint patterns...")
        
        # Build request-response correlation
        request_response_pairs = {}
        for req in self.requests:
            tid = req.get('tid')
            if tid:
                request_response_pairs[tid] = {'request': req}
        
        for resp in self.responses:
            tid = resp.get('tid')
            if tid in request_response_pairs:
                request_response_pairs[tid]['response'] = resp
        
        # Analyze endpoint patterns
        endpoint_patterns = defaultdict(lambda: {
            'request_count': 0,
            'methods': Counter(),
            'status_codes': Counter(),
            'avg_response_time': 0,
            'response_times': [],
            'request_headers': Counter(),
            'response_headers': Counter(),
            'hosts': Counter()
        })
        
        for tid, pair in request_response_pairs.items():
            if 'request' not in pair:
                continue
                
            req = pair['request']
            resp = pair.get('response')
            
            path = req.get('path', 'unknown')
            method = req.get('method', 'UNKNOWN')
            
            # Normalize path for grouping
            normalized_path = self._normalize_path(path)
            
            # Update pattern stats
            pattern = endpoint_patterns[normalized_path]
            pattern['request_count'] += 1
            pattern['methods'][method] += 1
            
            # Request headers analysis
            req_headers = req.get('headers', {})
            for header_name in req_headers.keys():
                pattern['request_headers'][header_name.lower()] += 1
            
            if 'host' in req_headers:
                pattern['hosts'][req_headers['host']] += 1
            
            # Response analysis
            if resp:
                status_code = resp.get('status_code', 0)
                pattern['status_codes'][status_code] += 1
                
                # Response headers analysis
                resp_headers = resp.get('headers', {})
                for header_name in resp_headers.keys():
                    pattern['response_headers'][header_name.lower()] += 1
                
                # Calculate response time
                req_time = req.get('timestamp', 0)
                resp_time = resp.get('timestamp', 0)
                if resp_time > req_time:
                    response_time_ms = (resp_time - req_time) / 1_000_000
                    pattern['response_times'].append(response_time_ms)
        
        # Calculate averages and convert to serializable format
        serializable_patterns = {}
        for endpoint, pattern in endpoint_patterns.items():
            if pattern['response_times']:
                avg_response_time = sum(pattern['response_times']) / len(pattern['response_times'])
                max_response_time = max(pattern['response_times'])
                min_response_time = min(pattern['response_times'])
            else:
                avg_response_time = 0
                max_response_time = 0
                min_response_time = 0
            
            serializable_patterns[endpoint] = {
                'request_count': pattern['request_count'],
                'methods': dict(pattern['methods']),
                'status_codes': dict(pattern['status_codes']),
                'avg_response_time_ms': avg_response_time,
                'max_response_time_ms': max_response_time,
                'min_response_time_ms': min_response_time,
                'unique_request_headers': len(pattern['request_headers']),
                'unique_response_headers': len(pattern['response_headers']),
                'most_common_request_headers': dict(pattern['request_headers'].most_common(10)),
                'most_common_response_headers': dict(pattern['response_headers'].most_common(10)),
                'hosts': dict(pattern['hosts'])
            }
        
        # Sort by request count
        sorted_patterns = sorted(serializable_patterns.items(), 
                               key=lambda x: x[1]['request_count'], reverse=True)
        
        return {
            "total_unique_endpoints": len(endpoint_patterns),
            "most_active_endpoints": sorted_patterns[:15],
            "endpoint_details": serializable_patterns
        }
        
    def generate_analysis_report(self) -> Dict[str, Any]:
        """Generate comprehensive header and metrics analysis report"""
        self.debug_print("Generating header and metrics analysis report...")
        
        report = {
            "analysis_metadata": {
                "timestamp": datetime.now().isoformat(),
                "source_file": self.timeline_file,
                "analyzer_version": "1.0.0",
                "focus": "HTTP headers and response metrics"
            },
            "summary": {},
            "detailed_analysis": {}
        }
        
        # Run focused analyses
        request_headers = self.analyze_request_headers()
        response_headers = self.analyze_response_headers()
        response_metrics = self.analyze_response_metrics()
        communication = self.analyze_communication_patterns()
        content_analysis = self.analyze_content_analysis()
        
        # Build summary
        report["summary"] = {
            "total_requests": len(self.requests),
            "total_responses": len(self.responses),
            "unique_request_headers": request_headers.get("unique_headers", 0),
            "unique_response_headers": response_headers.get("unique_headers", 0),
            "sse_responses": communication.get("sse_responses", 0),
            "avg_response_time_ms": response_metrics.get("response_time_ms", {}).get("mean", 0),
            "total_data_transferred": content_analysis.get("response_content", {}).get("total_response_size", 0)
        }
        
        # Add enhanced endpoint analysis
        endpoint_analysis = self.analyze_endpoint_patterns()
        
        # Store detailed analyses
        report["detailed_analysis"] = {
            "request_headers": request_headers,
            "response_headers": response_headers, 
            "response_metrics": response_metrics,
            "communication_patterns": communication,
            "content_analysis": content_analysis,
            "endpoint_patterns": endpoint_analysis
        }
        
        return report
        
    def analyze(self) -> Dict[str, Any]:
        """Main analysis entry point"""
        self.load_timeline_data()
        return self.generate_analysis_report()

def main():
    parser = argparse.ArgumentParser(
        description="SSL Header and Metrics Analyzer - Focused analysis of HTTP headers and response metrics",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Analysis Categories:
  - HTTP request header analysis
  - HTTP response header analysis  
  - Response time and size metrics
  - Communication pattern analysis
  - Content type and size analysis

Examples:
  python ssl_header_metrics_analyzer.py input_timeline.json
  python ssl_header_metrics_analyzer.py input_timeline.json -o analysis_report.json
  python ssl_header_metrics_analyzer.py input_timeline.json --quiet
        """
    )
    
    parser.add_argument('timeline_file', help='Timeline JSON file to analyze')
    parser.add_argument('-o', '--output', help='Output file path (default: <input>_header_metrics.json)')
    parser.add_argument('-q', '--quiet', action='store_true', help='Suppress debug output')
    
    args = parser.parse_args()
    
    try:
        analyzer = SSLHeaderMetricsAnalyzer(args.timeline_file, quiet=args.quiet)
        results = analyzer.analyze()
        
        # Determine output file
        if args.output:
            output_file = args.output
        else:
            base_name = args.timeline_file.replace('.json', '')
            output_file = f"{base_name}_header_metrics.json"
        
        # Write results
        with open(output_file, 'w') as f:
            json.dump(results, f, indent=2, ensure_ascii=False)
        
        if not args.quiet:
            print(f"Header and metrics analysis complete. Results written to: {output_file}")
            print(f"\nSummary:")
            summary = results['summary']
            print(f"  Total requests: {summary['total_requests']}")
            print(f"  Total responses: {summary['total_responses']}")
            print(f"  Unique request headers: {summary['unique_request_headers']}")
            print(f"  Unique response headers: {summary['unique_response_headers']}")
            print(f"  SSE responses: {summary['sse_responses']}")
            print(f"  Average response time: {summary['avg_response_time_ms']:.2f}ms")
            print(f"  Total data transferred: {summary['total_data_transferred']} bytes")
        
        return output_file
        
    except FileNotFoundError:
        print(f"Error: File '{args.timeline_file}' not found", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error: Invalid JSON in '{args.timeline_file}': {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == '__main__':
    main() 