#!/usr/bin/env python3
"""
SSL Log Analyzer - Creates a simple chronological timeline with merged SSE content

Analyzes SSL log files to:
1. Parse JSON log entries 
2. Create a chronological timeline of requests and responses
3. Merge Server-Sent Events (SSE) into plain text responses
4. Output a simple timeline with merged content
"""

import json
import sys
import re
import argparse
import os
from collections import defaultdict, OrderedDict
from typing import Dict, List, Any, Optional, Tuple
from datetime import datetime

class SSLLogAnalyzer:
    def __init__(self, log_file: str, quiet: bool = False):
        self.log_file = log_file
        self.quiet = quiet
        self.all_entries = []  # All parsed entries with timestamps
        self.timeline = []  # Simple chronological timeline
        
    def debug_print(self, message: str):
        """Print debug message only if not in quiet mode"""
        if not self.quiet:
            print(message)
        
    def parse_http_data(self, data: str) -> Dict[str, Any]:
        """Parse HTTP request/response data"""
        lines = data.split('\r\n')
        
        if not lines:
            return {}
            
        # Parse first line (request line or status line)
        first_line = lines[0]
        result = {'raw_data': data}
        
        # Check if this is a chunked SSE event (starts with hex chunk size)
        if re.match(r'^[0-9a-fA-F]+$', first_line.strip()):
            # This is a chunked SSE event
            result['type'] = 'sse_chunk'
            result['chunk_size'] = int(first_line.strip(), 16)
            
            # Parse the SSE content from the chunk
            if len(lines) > 1:
                # Join all lines except the first (chunk size) and last (empty)
                sse_content = '\r\n'.join(lines[1:])
                if sse_content.endswith('\r\n'):
                    sse_content = sse_content[:-2]
                    
                # Parse SSE events from this chunk
                sse_events = self.parse_sse_events_from_chunk(sse_content)
                result['sse_events'] = sse_events
                
            return result
        
        if first_line.startswith('HTTP/'):
            # Response
            parts = first_line.split(' ', 2)
            result['type'] = 'response'
            result['status_code'] = int(parts[1]) if len(parts) > 1 else 0
            result['status_text'] = parts[2] if len(parts) > 2 else ''
        else:
            # Request
            parts = first_line.split(' ')
            result['type'] = 'request'
            result['method'] = parts[0] if parts else ''
            result['path'] = parts[1] if len(parts) > 1 else ''
            result['protocol'] = parts[2] if len(parts) > 2 else ''
            
        # Parse headers
        headers = {}
        body_start = None
        
        for i, line in enumerate(lines[1:], 1):
            if line == '':
                body_start = i + 1
                break
            if ':' in line:
                key, value = line.split(':', 1)
                headers[key.lower().strip()] = value.strip()
                
        result['headers'] = headers
        
        # Parse body if present
        if body_start and body_start < len(lines):
            body = '\r\n'.join(lines[body_start:])
            if body.strip():
                result['body'] = body
                # Try to parse JSON body
                try:
                    result['json_body'] = json.loads(body)
                except json.JSONDecodeError:
                    pass
                    
        return result
        
    def parse_sse_events_from_chunk(self, chunk_content: str) -> List[Dict[str, Any]]:
        """Parse SSE events from a single chunk"""
        events = []
        
        # Split by double newlines to separate events
        event_blocks = re.split(r'\n\s*\n', chunk_content)
        
        for block in event_blocks:
            if not block.strip():
                continue
                
            event = {}
            data_lines = []
            
            for line in block.split('\n'):
                line = line.strip()
                if line.startswith('event:'):
                    event['event'] = line[6:].strip()
                elif line.startswith('data:'):
                    data_lines.append(line[5:].strip())
                elif line.startswith('id:'):
                    event['id'] = line[3:].strip()
                    
            if data_lines:
                combined_data = '\n'.join(data_lines)
                event['data'] = combined_data
                
                # Try to parse as JSON
                try:
                    event['parsed_data'] = json.loads(combined_data)
                except json.JSONDecodeError:
                    event['raw_data'] = combined_data
                    
            if event:
                events.append(event)
                
        return events
        
    def process_log_entry(self, entry: Dict[str, Any]):
        """Process a single log entry"""
        ssl_data = entry.get('data', {})
        if 'data' not in ssl_data:
            return
            
        # Parse HTTP data
        parsed_data = self.parse_http_data(ssl_data['data'])
        
        # Add metadata from SSL data
        parsed_data['timestamp'] = entry.get('timestamp')
        parsed_data['function'] = ssl_data.get('function')
        parsed_data['pid'] = ssl_data.get('pid')
        parsed_data['tid'] = ssl_data.get('tid')
        parsed_data['comm'] = ssl_data.get('comm')
        
        # Add to all entries for timeline processing
        self.all_entries.append(parsed_data)
        
    def is_sse_response(self, parsed_data: Dict[str, Any]) -> bool:
        """Check if this is a Server-Sent Events response"""
        headers = parsed_data.get('headers', {})
        content_type = headers.get('content-type', '')
        return 'text/event-stream' in content_type
        
    def group_by_timeline(self):
        """Create a simple chronological timeline, merging SSE chunks into responses"""
        # Sort all entries by timestamp
        sorted_entries = sorted(self.all_entries, key=lambda x: x.get('timestamp', 0))
        
        timeline = []
        current_sse_response = None
        sse_merge_timeout = 5000000000  # 5 seconds in nanoseconds
        
        self.debug_print(f"Processing {len(sorted_entries)} entries...")
        
        for entry in sorted_entries:
            entry_type = entry.get('type')
            timestamp = entry.get('timestamp', 0)
            tid = entry.get('tid')
            
            if entry_type == 'request':
                # Finalize any pending SSE response before processing new request
                if current_sse_response:
                    self.debug_print(f"[DEBUG] Finalizing SSE response due to new request")
                    self._finalize_sse_response(current_sse_response)
                    current_sse_response = None
                    
                # Add request to timeline
                timeline.append(entry)
                
            elif entry_type == 'response':
                # Finalize any pending SSE response before processing new response
                if current_sse_response:
                    self.debug_print(f"[DEBUG] Finalizing SSE response due to new response")
                    self._finalize_sse_response(current_sse_response)
                    current_sse_response = None
                    
                if self.is_sse_response(entry):
                    # This is an SSE response, prepare for chunk merging
                    self.debug_print(f"[DEBUG] Found SSE response at timestamp {timestamp}")
                    entry['sse_text_parts'] = []
                    entry['sse_raw_chunks'] = []  # Store original chunks for debugging
                    # Extract any initial SSE events from the response body
                    if 'body' in entry:
                        initial_text = self._extract_sse_events_from_body(entry)
                        if initial_text:
                            self.debug_print(f"[DEBUG] Extracted initial text from response: '{initial_text}'")
                    current_sse_response = entry
                    
                # Add response to timeline
                timeline.append(entry)
                
            elif entry_type == 'sse_chunk':
                # Merge SSE chunk into the current SSE response
                if current_sse_response and tid == current_sse_response.get('tid'):
                    time_since_response = timestamp - current_sse_response.get('timestamp', 0)
                    
                    if time_since_response <= sse_merge_timeout:
                        self.debug_print(f"[DEBUG] Processing SSE chunk at timestamp {timestamp} (TID: {tid})")
                        
                        # Store the raw chunk for debugging
                        current_sse_response['sse_raw_chunks'].append({
                            'timestamp': timestamp,
                            'raw_data': entry.get('raw_data', ''),
                            'sse_events': entry.get('sse_events', [])
                        })
                        
                        # Extract text content from SSE events in this chunk
                        chunk_text_parts = []
                        for event in entry.get('sse_events', []):
                            if event.get('event') == 'content_block_delta':
                                if 'parsed_data' in event:
                                    delta = event['parsed_data'].get('delta', {})
                                    text = ''
                                    if delta.get('type') == 'text_delta':
                                        text = delta.get('text', '')
                                    elif delta.get('type') == 'thinking_delta':
                                        text = delta.get('thinking', '')
                                    
                                    if text:
                                        chunk_text_parts.append(text)
                                        current_sse_response['sse_text_parts'].append(text)
                        
                        if chunk_text_parts:
                            self.debug_print(f"[DEBUG] Extracted text from chunk: {chunk_text_parts}")
                        
                        # Update response timestamp to latest chunk
                        current_sse_response['timestamp'] = timestamp
                    else:
                        # Timeout reached, finalize current response
                        self.debug_print(f"[DEBUG] SSE timeout reached ({time_since_response}ns > {sse_merge_timeout}ns), finalizing")
                        self._finalize_sse_response(current_sse_response)
                        current_sse_response = None
                else:
                    # No current SSE response or different TID, finalize if exists
                    if current_sse_response:
                        self.debug_print(f"[DEBUG] TID mismatch or no current SSE response, finalizing")
                        self._finalize_sse_response(current_sse_response)
                        current_sse_response = None
        
        # Finalize any remaining SSE response
        if current_sse_response:
            self.debug_print(f"[DEBUG] Finalizing remaining SSE response")
            self._finalize_sse_response(current_sse_response)
        
        # Final pass: finalize any remaining SSE responses that weren't processed
        sse_count = 0
        for entry in timeline:
            if entry.get('type') == 'response' and 'sse_text_parts' in entry:
                self._finalize_sse_response(entry)
                sse_count += 1
        
        if sse_count > 0:
            self.debug_print(f"[DEBUG] Final pass: finalized {sse_count} remaining SSE responses")
            
        self.timeline = timeline
        
    def _extract_sse_events_from_body(self, response):
        """Extract SSE events from the initial response body"""
        body = response.get('body', '')
        if not body:
            return ''
            
        self.debug_print(f"[DEBUG] Extracting SSE events from response body (length: {len(body)})")
            
        # Handle chunked encoding - extract actual content from chunks
        content_parts = []
        lines = body.split('\r\n')
        
        i = 0
        while i < len(lines):
            line = lines[i].strip()
            
            # Check if this is a chunk size (hex number)
            if re.match(r'^[0-9a-fA-F]+$', line):
                chunk_size = int(line, 16)
                if chunk_size == 0:
                    break
                    
                # Get the chunk content (next line)
                i += 1
                if i < len(lines):
                    content_parts.append(lines[i])
            i += 1
            
        # Join all content and parse SSE events
        full_content = '\n'.join(content_parts)
        self.debug_print(f"[DEBUG] Extracted chunked content (length: {len(full_content)})")
        
        # Parse SSE events from the content
        events = self.parse_sse_events_from_chunk(full_content)
        extracted_texts = []
        
        for event in events:
            if event.get('event') == 'content_block_delta':
                if 'parsed_data' in event:
                    delta = event['parsed_data'].get('delta', {})
                    text = ''
                    if delta.get('type') == 'text_delta':
                        text = delta.get('text', '')
                    elif delta.get('type') == 'thinking_delta':
                        text = delta.get('thinking', '')
                    
                    if text:
                        extracted_texts.append(text)
                        response['sse_text_parts'].append(text)
        
        merged_text = ''.join(extracted_texts)
        if extracted_texts:
            self.debug_print(f"[DEBUG] Found {len(extracted_texts)} text deltas in response body: {extracted_texts}")
        else:
            self.debug_print(f"[DEBUG] No text deltas found in response body")
            
        return merged_text
        
    def _finalize_sse_response(self, sse_response):
        """Finalize an SSE response by merging text parts into body"""
        if 'sse_text_parts' in sse_response:
            merged_text = ''.join(sse_response['sse_text_parts'])
            raw_chunks_count = len(sse_response.get('sse_raw_chunks', []))
            
            self.debug_print(f"[DEBUG] Finalizing SSE response:")
            self.debug_print(f"  - Text parts: {sse_response['sse_text_parts']}")
            self.debug_print(f"  - Merged text: '{merged_text}'")
            self.debug_print(f"  - Raw chunks count: {raw_chunks_count}")
            
            if merged_text:
                # Update the response body with merged content
                sse_response['body'] = merged_text
                try:
                    # Try to parse as JSON if it looks like JSON
                    if merged_text.strip().startswith('{'):
                        sse_response['json_body'] = json.loads(merged_text)
                        self.debug_print(f"  - Parsed as JSON body")
                except json.JSONDecodeError:
                    self.debug_print(f"  - Not valid JSON, keeping as text")
                    pass
            else:
                self.debug_print(f"  - No merged text, keeping original body")
            
            # Clean up temporary SSE-specific fields but keep raw chunks for debugging
            del sse_response['sse_text_parts']
            # Remove any old SSE-specific fields that might be present
            for field in ['sse_events', 'merged_content', 'sse_chunks', 'merged_sse_events', 'conversation_info', 'sse_summary']:
                if field in sse_response:
                    del sse_response[field]
                    
            self.debug_print(f"  - Finalization complete")
        
    def _create_simple_timeline(self, timeline: List[Dict[str, Any]]) -> List[str]:
        """Create simplified timeline with just essential request/response info"""
        simple_entries = []
        
        for entry in timeline:
            entry_type = entry.get('type')
            
            if entry_type == 'request':
                method = entry.get('method', 'UNKNOWN')
                path = entry.get('path', '/')
                protocol = entry.get('protocol', 'HTTP/1.1')
                headers = entry.get('headers', {})
                host = headers.get('host', 'unknown-host')
                
                simple_entry = f"{method} {path} {protocol} host: {host}"
                simple_entries.append(simple_entry)
                
            elif entry_type == 'response':
                status_code = entry.get('status_code', 0)
                status_text = entry.get('status_text', '')
                headers = entry.get('headers', {})
                content_type = headers.get('content-type', 'unknown')
                
                simple_entry = f"HTTP/1.1 {status_code} {status_text} content-type: {content_type}"
                simple_entries.append(simple_entry)
        
        return simple_entries
        
    def analyze(self) -> Dict[str, Any]:
        """Analyze the SSL log file"""
        with open(self.log_file, 'r') as f:
            for line_num, line in enumerate(f, 1):
                try:
                    entry = json.loads(line.strip())
                    self.process_log_entry(entry)
                except json.JSONDecodeError as e:
                    if not self.quiet:
                        print(f"Warning: Failed to parse line {line_num}: {e}", file=sys.stderr)
                    continue
                    
        # Create chronological timeline and merge SSE events
        self.group_by_timeline()
            
        # Calculate statistics
        requests = sum(1 for entry in self.timeline if entry.get('type') == 'request')
        responses = sum(1 for entry in self.timeline if entry.get('type') == 'response')
        sse_responses = sum(1 for entry in self.timeline if entry.get('type') == 'response' and 'text/event-stream' in entry.get('headers', {}).get('content-type', ''))
        
        # Prepare final results
        results = {
            'analysis_metadata': {
                'timestamp': datetime.now().isoformat(),
                'source_file': self.log_file,
                'total_timeline_entries': len(self.timeline),
                'total_requests': requests,
                'total_responses': responses,
                'sse_responses': sse_responses,
                'total_entries_processed': len(self.all_entries)
            },
            'timeline': self.timeline
        }
        
        return results

def main():
    parser = argparse.ArgumentParser(
        description="SSL Log Analyzer - Creates chronological timeline with merged SSE content",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Output formats:
  json          - Full analysis results with metadata (default)
  timeline      - Timeline-only JSON file
  both          - Both full results and timeline-only files

Examples:
  python ssl_log_analyzer.py input.log
  python ssl_log_analyzer.py input.log -o results.json
  python ssl_log_analyzer.py input.log --format timeline -q
  python ssl_log_analyzer.py input.log --format both -o analysis
        """
    )
    
    parser.add_argument('log_file', help='SSL log file to analyze')
    parser.add_argument('-o', '--output', help='Output file path (without extension for --format both)')
    parser.add_argument('--format', choices=['json', 'timeline', 'both'], default='json',
                        help='Output format (default: json)')
    parser.add_argument('-q', '--quiet', action='store_true',
                        help='Suppress debug output')
    
    args = parser.parse_args()
    
    try:
        analyzer = SSLLogAnalyzer(args.log_file, quiet=args.quiet)
        results = analyzer.analyze()
        
        # Determine output file names
        if args.output:
            base_output = args.output
            if base_output.endswith('.json'):
                base_output = base_output[:-5]
        else:
            base_output = args.log_file.replace('.log', '_analyzed')
        
        # Write output based on format
        output_files = []
        
        if args.format in ['json', 'both']:
            full_output_file = f"{base_output}.json"
            with open(full_output_file, 'w') as f:
                json.dump(results, f, indent=2, ensure_ascii=False)
            output_files.append(full_output_file)
            
        if args.format in ['timeline', 'both']:
            timeline_output_file = f"{base_output}_simple_timeline.json"
            # Create simplified timeline with just essential info
            simple_timeline = analyzer._create_simple_timeline(results['timeline'])
            timeline_data = {
                'analysis_metadata': results['analysis_metadata'],
                'simple_timeline': simple_timeline
            }
            with open(timeline_output_file, 'w') as f:
                json.dump(timeline_data, f, indent=2, ensure_ascii=False)
            output_files.append(timeline_output_file)
        
        # Print summary
        if not args.quiet:
            print(f"Analysis complete. Output written to: {', '.join(output_files)}")
            print(f"Total timeline entries: {results['analysis_metadata']['total_timeline_entries']}")
            print(f"Total requests: {results['analysis_metadata']['total_requests']}")
            print(f"Total responses: {results['analysis_metadata']['total_responses']}")
            print(f"SSE responses: {results['analysis_metadata']['sse_responses']}")
            print(f"Total entries processed: {results['analysis_metadata']['total_entries_processed']}")
        
        return output_files
        
    except FileNotFoundError:
        print(f"Error: File '{args.log_file}' not found", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == '__main__':
    main()