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
from collections import defaultdict, OrderedDict
from typing import Dict, List, Any, Optional, Tuple
from datetime import datetime

class SSLLogAnalyzer:
    def __init__(self, log_file: str):
        self.log_file = log_file
        self.all_entries = []  # All parsed entries with timestamps
        self.timeline = []  # Simple chronological timeline
        
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
        
        for entry in sorted_entries:
            entry_type = entry.get('type')
            timestamp = entry.get('timestamp', 0)
            tid = entry.get('tid')
            
            if entry_type == 'request':
                # Finalize any pending SSE response before processing new request
                if current_sse_response:
                    self._finalize_sse_response(current_sse_response)
                    current_sse_response = None
                    
                # Add request to timeline
                timeline.append(entry)
                
            elif entry_type == 'response':
                # Finalize any pending SSE response before processing new response
                if current_sse_response:
                    self._finalize_sse_response(current_sse_response)
                    current_sse_response = None
                    
                if self.is_sse_response(entry):
                    # This is an SSE response, prepare for chunk merging
                    entry['sse_text_parts'] = []
                    # Extract any initial SSE events from the response body
                    if 'body' in entry:
                        self._extract_sse_events_from_body(entry)
                    current_sse_response = entry
                    
                # Add response to timeline
                timeline.append(entry)
                
            elif entry_type == 'sse_chunk':
                # Merge SSE chunk into the current SSE response
                if current_sse_response and tid == current_sse_response.get('tid'):
                    time_since_response = timestamp - current_sse_response.get('timestamp', 0)
                    
                    if time_since_response <= sse_merge_timeout:
                        # Extract text content from SSE events in this chunk
                        for event in entry.get('sse_events', []):
                            if event.get('event') == 'content_block_delta':
                                if 'parsed_data' in event:
                                    delta = event['parsed_data'].get('delta', {})
                                    if delta.get('type') == 'text_delta':
                                        text = delta.get('text', '')
                                        if text:
                                            current_sse_response['sse_text_parts'].append(text)
                        
                        # Update response timestamp to latest chunk
                        current_sse_response['timestamp'] = timestamp
                    else:
                        # Timeout reached, finalize current response
                        self._finalize_sse_response(current_sse_response)
                        current_sse_response = None
                else:
                    # No current SSE response or different TID, finalize if exists
                    if current_sse_response:
                        self._finalize_sse_response(current_sse_response)
                        current_sse_response = None
        
        # Finalize any remaining SSE response
        if current_sse_response:
            self._finalize_sse_response(current_sse_response)
        
        # Final pass: finalize any remaining SSE responses that weren't processed
        for entry in timeline:
            if entry.get('type') == 'response' and 'sse_text_parts' in entry:
                self._finalize_sse_response(entry)
            
        self.timeline = timeline
        
    def _extract_sse_events_from_body(self, response):
        """Extract SSE events from the initial response body"""
        body = response.get('body', '')
        if not body:
            return
            
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
        
        # Parse SSE events from the content
        events = self.parse_sse_events_from_chunk(full_content)
        for event in events:
            if event.get('event') == 'content_block_delta':
                if 'parsed_data' in event:
                    delta = event['parsed_data'].get('delta', {})
                    if delta.get('type') == 'text_delta':
                        text = delta.get('text', '')
                        if text:
                            response['sse_text_parts'].append(text)
        
    def _finalize_sse_response(self, sse_response):
        """Finalize an SSE response by merging text parts into body"""
        if 'sse_text_parts' in sse_response:
            merged_text = ''.join(sse_response['sse_text_parts'])
            if merged_text:
                # Update the response body with merged content
                sse_response['body'] = merged_text
                try:
                    # Try to parse as JSON if it looks like JSON
                    if merged_text.strip().startswith('{'):
                        sse_response['json_body'] = json.loads(merged_text)
                except json.JSONDecodeError:
                    pass
            
            # Clean up SSE-specific fields
            del sse_response['sse_text_parts']
            # Remove any old SSE-specific fields that might be present
            for field in ['sse_events', 'merged_content', 'sse_chunks', 'merged_sse_events', 'conversation_info', 'sse_summary']:
                if field in sse_response:
                    del sse_response[field]
        
    def analyze(self) -> Dict[str, Any]:
        """Analyze the SSL log file"""
        with open(self.log_file, 'r') as f:
            for line_num, line in enumerate(f, 1):
                try:
                    entry = json.loads(line.strip())
                    self.process_log_entry(entry)
                except json.JSONDecodeError as e:
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
    if len(sys.argv) != 2:
        print("Usage: python ssl_log_analyzer.py <ssl_log_file>", file=sys.stderr)
        sys.exit(1)
        
    log_file = sys.argv[1]
    
    try:
        analyzer = SSLLogAnalyzer(log_file)
        results = analyzer.analyze()
        
        # Write results to new JSON file
        output_file = log_file.replace('.log', '_analyzed.json')
        with open(output_file, 'w') as f:
            json.dump(results, f, indent=2, ensure_ascii=False)
            
        print(f"Analysis complete. Results written to: {output_file}")
        print(f"Total timeline entries: {results['analysis_metadata']['total_timeline_entries']}")
        print(f"Total requests: {results['analysis_metadata']['total_requests']}")
        print(f"Total responses: {results['analysis_metadata']['total_responses']}")
        print(f"SSE responses: {results['analysis_metadata']['sse_responses']}")
        print(f"Total entries processed: {results['analysis_metadata']['total_entries_processed']}")
        
    except FileNotFoundError:
        print(f"Error: File '{log_file}' not found", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == '__main__':
    main()