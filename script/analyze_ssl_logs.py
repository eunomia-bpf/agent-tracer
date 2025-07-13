#!/usr/bin/env python3
"""
SSL Log Analyzer for Claude-related events
Analyzes SSL log events and extracts Claude-related message content
"""

import json
import re
import sys
from urllib.parse import unquote
from datetime import datetime

def extract_http_content(data):
    """Extract HTTP request/response content from SSL data"""
    if not data or data == "null":
        return None
    
    # Look for HTTP headers and content
    lines = data.split('\r\n')
    content = []
    in_body = False
    
    for line in lines:
        if in_body:
            content.append(line)
        elif line == '' and not in_body:
            in_body = True
        elif line.startswith('POST') or line.startswith('GET') or line.startswith('HTTP'):
            content.append(f"[HTTP] {line}")
        elif line.startswith('Host:') or line.startswith('Content-Type:') or line.startswith('User-Agent:'):
            content.append(f"[HEADER] {line}")
    
    return '\n'.join(content) if content else None

def is_claude_related(event):
    """Check if event is Claude-related"""
    data = event.get('data', {})
    
    # Check process name
    if data.get('comm') == 'claude':
        return True
    
    # Check for Anthropic domains or Claude-related content
    ssl_data = data.get('data', '')
    if ssl_data and isinstance(ssl_data, str):
        claude_indicators = [
            'anthropic.com',
            'claude',
            'api.anthropic.com',
            'statsig.anthropic.com',
            'claude-sonnet',
            'claude-code'
        ]
        
        ssl_data_lower = ssl_data.lower()
        for indicator in claude_indicators:
            if indicator in ssl_data_lower:
                return True
    
    return False

def extract_json_content(data):
    """Extract JSON content from HTTP body"""
    if not data:
        return None
    
    # Find JSON content in HTTP body
    lines = data.split('\r\n')
    in_body = False
    json_content = []
    
    for line in lines:
        if in_body:
            json_content.append(line)
        elif line == '' and not in_body:
            in_body = True
    
    json_str = '\n'.join(json_content)
    
    # Try to parse JSON
    try:
        if json_str.strip():
            return json.loads(json_str)
    except json.JSONDecodeError:
        pass
    
    return json_str if json_str.strip() else None

def format_timestamp(timestamp_ns):
    """Format timestamp for display"""
    try:
        # Convert nanoseconds to seconds
        timestamp_s = timestamp_ns / 1_000_000_000
        dt = datetime.fromtimestamp(timestamp_s)
        return dt.strftime('%Y-%m-%d %H:%M:%S.%f')[:-3]
    except:
        return str(timestamp_ns)

def analyze_ssl_logs(log_file):
    """Analyze SSL logs and extract Claude-related content"""
    claude_events = []
    
    try:
        with open(log_file, 'r') as f:
            for line_num, line in enumerate(f, 1):
                line = line.strip()
                if not line:
                    continue
                
                try:
                    event = json.loads(line)
                    
                    if is_claude_related(event):
                        claude_events.append({
                            'line_num': line_num,
                            'event': event
                        })
                        
                except json.JSONDecodeError as e:
                    print(f"Warning: Could not parse JSON on line {line_num}: {e}")
                    continue
    
    except FileNotFoundError:
        print(f"Error: File {log_file} not found")
        return []
    except Exception as e:
        print(f"Error reading file: {e}")
        return []
    
    return claude_events

def print_claude_events(claude_events):
    """Print Claude-related events in a formatted way"""
    if not claude_events:
        print("No Claude-related events found.")
        return
    
    print(f"Found {len(claude_events)} Claude-related events:")
    print("=" * 80)
    
    for i, event_data in enumerate(claude_events, 1):
        event = event_data['event']
        line_num = event_data['line_num']
        
        data = event.get('data', {})
        timestamp = event.get('timestamp', 0)
        
        print(f"\n[Event {i}] Line {line_num} - {format_timestamp(timestamp)}")
        print(f"Process: {data.get('comm', 'unknown')}")
        print(f"Function: {data.get('function', 'unknown')}")
        print(f"PID: {data.get('pid', 'unknown')}")
        
        # Extract and display SSL data
        ssl_data = data.get('data', '')
        if ssl_data and ssl_data != "null":
            print(f"SSL Data Length: {len(ssl_data)} bytes")
            
            # Extract HTTP content
            http_content = extract_http_content(ssl_data)
            if http_content:
                print("HTTP Content:")
                print("-" * 40)
                print(http_content)
                
                # Try to extract JSON from HTTP body
                json_content = extract_json_content(ssl_data)
                if json_content and isinstance(json_content, dict):
                    print("\nJSON Content:")
                    print("-" * 40)
                    print(json.dumps(json_content, indent=2))
                elif json_content and isinstance(json_content, str):
                    print("\nBody Content:")
                    print("-" * 40)
                    print(json_content)
        
        print("-" * 80)

def main():
    """Main function"""
    if len(sys.argv) > 1:
        log_file = sys.argv[1]
    else:
        log_file = "collector/ssl.log"
    
    print(f"Analyzing SSL logs from: {log_file}")
    print("Looking for Claude-related events...")
    
    claude_events = analyze_ssl_logs(log_file)
    print_claude_events(claude_events)

if __name__ == "__main__":
    main() 