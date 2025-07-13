#!/usr/bin/env python3
"""
SSL Data Timeline Generator - Creates a clean timeline with only data transfers

Filters out SSL handshake operations and focuses on:
1. HTTP request/response data only
2. Application-level communications
3. Clean chronological data flow
4. Actual payload transfers without protocol overhead
"""

import json
import sys
import argparse
import re
from collections import defaultdict
from typing import Dict, List, Any, Optional
from datetime import datetime

class SSLDataTimelineGenerator:
    def __init__(self, timeline_file: str, quiet: bool = False):
        self.timeline_file = timeline_file
        self.quiet = quiet
        self.timeline_data = None
        self.filtered_timeline = []
        
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
        self.debug_print(f"Loaded {len(timeline)} timeline entries")
        
        return timeline
        
    def is_handshake_or_protocol_overhead(self, entry: Dict[str, Any]) -> bool:
        """Check if entry represents handshake or protocol overhead"""
        entry_type = entry.get('type')
        
        # Skip non-request/response entries
        if entry_type not in ['request', 'response']:
            return True
            
        # Check for handshake-related paths
        path = entry.get('path', '')
        handshake_patterns = [
            r'\.well-known',
            r'/handshake',
            r'/negotiate',
            r'/connect',
            r'/ping',
            r'/health',
            r'/status',
            r'/version'
        ]
        
        for pattern in handshake_patterns:
            if re.search(pattern, path, re.IGNORECASE):
                return True
                
        # Check for protocol-level operations
        method = entry.get('method', '')
        if method in ['OPTIONS', 'HEAD']:
            return True
            
        # Check for empty responses (likely protocol overhead)
        if entry_type == 'response':
            body = entry.get('body', '')
            status_code = entry.get('status_code', 200)
            
            # Skip empty responses or protocol-level status codes
            if not body and status_code in [204, 304]:
                return True
                
        return False
        
    def is_data_transfer(self, entry: Dict[str, Any]) -> bool:
        """Check if entry represents actual data transfer"""
        entry_type = entry.get('type')
        
        if entry_type == 'request':
            # Requests with body or significant API calls
            body = entry.get('body', '')
            path = entry.get('path', '')
            method = entry.get('method', '')
            
            # Has payload
            if body:
                return True
                
            # Significant API endpoints
            data_endpoints = [
                r'/api/',
                r'/v1/',
                r'/v2/',
                r'/chat',
                r'/completion',
                r'/generate',
                r'/query',
                r'/search',
                r'/upload',
                r'/download'
            ]
            
            for pattern in data_endpoints:
                if re.search(pattern, path, re.IGNORECASE):
                    return True
                    
        elif entry_type == 'response':
            # Responses with actual content
            body = entry.get('body', '')
            headers = entry.get('headers', {})
            content_type = headers.get('content-type', '')
            
            # Has meaningful content
            if body:
                return True
                
            # Check for streaming responses
            if 'text/event-stream' in content_type:
                return True
                
        return False
        
    def clean_entry(self, entry: Dict[str, Any]) -> Dict[str, Any]:
        """Clean entry by removing unnecessary fields including headers"""
        cleaned = {
            'type': entry.get('type'),
            'timestamp': entry.get('timestamp')
        }
        
        # Add type-specific fields (excluding headers for clean data timeline)
        if entry.get('type') == 'request':
            cleaned.update({
                'method': entry.get('method'),
                'path': entry.get('path'),
                'body': entry.get('body', ''),
                'tid': entry.get('tid')
            })
            
            # Add JSON body if available
            if entry.get('json_body'):
                cleaned['json_body'] = entry['json_body']
                
        elif entry.get('type') == 'response':
            cleaned.update({
                'status_code': entry.get('status_code'),
                'status_text': entry.get('status_text', ''),
                'body': entry.get('body', ''),
                'tid': entry.get('tid')
            })
            
            # Add JSON body if available
            if entry.get('json_body'):
                cleaned['json_body'] = entry['json_body']
                
            # Keep SSE-related fields if present (but without headers)
            if entry.get('sse_raw_chunks'):
                cleaned['sse_raw_chunks'] = entry['sse_raw_chunks']
        
        return cleaned
        
    def filter_timeline(self, timeline: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Filter timeline to include only data transfers"""
        self.debug_print("Filtering timeline for data transfers...")
        
        filtered = []
        skipped_handshake = 0
        skipped_non_data = 0
        
        for entry in timeline:
            # Skip handshake and protocol overhead
            if self.is_handshake_or_protocol_overhead(entry):
                skipped_handshake += 1
                continue
                
            # Only include actual data transfers
            if self.is_data_transfer(entry):
                cleaned_entry = self.clean_entry(entry)
                filtered.append(cleaned_entry)
            else:
                skipped_non_data += 1
        
        self.debug_print(f"Filtering complete:")
        self.debug_print(f"  - Original entries: {len(timeline)}")
        self.debug_print(f"  - Filtered entries: {len(filtered)}")
        self.debug_print(f"  - Skipped handshake/protocol: {skipped_handshake}")
        self.debug_print(f"  - Skipped non-data: {skipped_non_data}")
        
        return filtered
        
    def add_data_flow_context(self, filtered_timeline: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Add context about data flow patterns"""
        self.debug_print("Adding data flow context...")
        
        # Group related requests and responses
        request_response_pairs = {}
        
        for entry in filtered_timeline:
            tid = entry.get('tid')
            entry_type = entry.get('type')
            
            if tid:
                if tid not in request_response_pairs:
                    request_response_pairs[tid] = {}
                    
                request_response_pairs[tid][entry_type] = entry
                
        # Add flow context to entries
        for entry in filtered_timeline:
            tid = entry.get('tid')
            entry_type = entry.get('type')
            
            if tid in request_response_pairs:
                pair = request_response_pairs[tid]
                
                # Add request context to response
                if entry_type == 'response' and 'request' in pair:
                    request = pair['request']
                    entry['request_context'] = {
                        'method': request.get('method'),
                        'path': request.get('path'),
                        'has_body': bool(request.get('body'))
                    }
                    
                # Add response context to request
                elif entry_type == 'request' and 'response' in pair:
                    response = pair['response']
                    entry['response_context'] = {
                        'status_code': response.get('status_code'),
                        'has_body': bool(response.get('body'))
                    }
        
        return filtered_timeline
        
    def generate_summary(self, filtered_timeline: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Generate summary statistics for the filtered timeline"""
        requests = [e for e in filtered_timeline if e.get('type') == 'request']
        responses = [e for e in filtered_timeline if e.get('type') == 'response']
        
        # Calculate data transfer sizes
        total_request_size = sum(len(req.get('body', '')) for req in requests)
        total_response_size = sum(len(resp.get('body', '')) for resp in responses)
        
        # Analyze content types (basic categorization without headers)
        content_types = defaultdict(int)
        for resp in responses:
            # Basic content type detection from body or response characteristics
            body = resp.get('body', '')
            if body:
                if body.strip().startswith('{') or body.strip().startswith('['):
                    content_types['json'] += 1
                elif '<html' in body.lower():
                    content_types['html'] += 1
                elif 'data:' in body and 'event:' in body:
                    content_types['text/event-stream'] += 1
                else:
                    content_types['text/plain'] += 1
            else:
                content_types['empty'] += 1
        
        # Calculate time span
        if filtered_timeline:
            timestamps = [entry.get('timestamp', 0) for entry in filtered_timeline]
            min_time = min(timestamps)
            max_time = max(timestamps)
            duration_seconds = (max_time - min_time) / 1_000_000_000
        else:
            duration_seconds = 0
        
        return {
            'total_data_entries': len(filtered_timeline),
            'total_requests': len(requests),
            'total_responses': len(responses),
            'data_transfer_sizes': {
                'total_request_bytes': total_request_size,
                'total_response_bytes': total_response_size,
                'total_data_bytes': total_request_size + total_response_size
            },
            'content_type_distribution': dict(content_types),
            'session_duration_seconds': duration_seconds,
            'average_data_rate_bps': (total_request_size + total_response_size) / duration_seconds if duration_seconds > 0 else 0
        }
        
    def generate_clean_timeline(self) -> Dict[str, Any]:
        """Generate clean data timeline"""
        self.debug_print("Generating clean data timeline...")
        
        # Load and filter timeline
        timeline = self.load_timeline_data()
        filtered_timeline = self.filter_timeline(timeline)
        
        # Add context
        filtered_timeline = self.add_data_flow_context(filtered_timeline)
        
        # Generate summary
        summary = self.generate_summary(filtered_timeline)
        
        # Sort by timestamp
        filtered_timeline.sort(key=lambda x: x.get('timestamp', 0))
        
        return {
            'analysis_metadata': {
                'timestamp': datetime.now().isoformat(),
                'source_file': self.timeline_file,
                'generator_version': '1.0.0',
                'focus': 'HTTP data transfers only (no handshake/protocol overhead)'
            },
            'summary': summary,
            'data_timeline': filtered_timeline
        }
        
    def generate(self) -> Dict[str, Any]:
        """Main generation entry point"""
        return self.generate_clean_timeline()

def main():
    parser = argparse.ArgumentParser(
        description="SSL Data Timeline Generator - Creates clean timeline with only data transfers",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Features:
  - Filters out SSL handshake operations
  - Removes protocol overhead (OPTIONS, HEAD, etc.)
  - Focuses on actual HTTP data transfers
  - Adds data flow context between requests/responses
  - Provides data transfer statistics

Examples:
  python ssl_data_timeline.py input_timeline.json
  python ssl_data_timeline.py input_timeline.json -o clean_data.json
  python ssl_data_timeline.py input_timeline.json --quiet
        """
    )
    
    parser.add_argument('timeline_file', help='Timeline JSON file to process')
    parser.add_argument('-o', '--output', help='Output file path (default: <input>_data_only.json)')
    parser.add_argument('-q', '--quiet', action='store_true', help='Suppress debug output')
    
    args = parser.parse_args()
    
    try:
        generator = SSLDataTimelineGenerator(args.timeline_file, quiet=args.quiet)
        results = generator.generate()
        
        # Determine output file
        if args.output:
            output_file = args.output
        else:
            base_name = args.timeline_file.replace('.json', '')
            output_file = f"{base_name}_data_only.json"
        
        # Write results
        with open(output_file, 'w') as f:
            json.dump(results, f, indent=2, ensure_ascii=False)
        
        if not args.quiet:
            print(f"Data timeline generation complete. Results written to: {output_file}")
            print(f"\nSummary:")
            summary = results['summary']
            print(f"  Total data entries: {summary['total_data_entries']}")
            print(f"  Total requests: {summary['total_requests']}")
            print(f"  Total responses: {summary['total_responses']}")
            print(f"  Total data transferred: {summary['data_transfer_sizes']['total_data_bytes']} bytes")
            print(f"  Session duration: {summary['session_duration_seconds']:.2f}s")
            print(f"  Average data rate: {summary['average_data_rate_bps']:.2f} bytes/s")
        
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