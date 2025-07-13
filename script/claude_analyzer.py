#!/usr/bin/env python3
"""
Claude SSL Log Analyzer

This script analyzes SSL log events from the chunk merger and extracts 
Claude-related message content. It processes both raw SSL events and 
merged chunk_merger events to provide a clean view of Claude conversations.

Usage:
    python3 claude_analyzer.py [ssl_log_file] [--process PROCESS_NAME]
    
Options:
    --process PROCESS_NAME  Filter events by specific process name (e.g., 'claude')
    -p PROCESS_NAME        Short form of --process
    
If no file is specified, it will look for 'ssl.log' in the current directory.

Examples:
    python3 claude_analyzer.py                    # Analyze default log with Claude filtering
    python3 claude_analyzer.py ssl.log            # Analyze specific log file
    python3 claude_analyzer.py --process claude   # Filter only 'claude' process events
    python3 claude_analyzer.py ssl.log -p python  # Filter only 'python' process events
"""

import json
import sys
import os
from typing import Dict, List, Optional, Any
from datetime import datetime
from collections import defaultdict

class ClaudeAnalyzer:
    def __init__(self, log_file: str, process_filter: Optional[str] = None):
        self.log_file = log_file
        self.process_filter = process_filter
        self.events = []
        self.claude_events = []
        self.chunk_merger_events = []
        self.conversations = defaultdict(list)
        
    def parse_log_file(self) -> None:
        """Parse the SSL log file and extract all events"""
        try:
            with open(self.log_file, 'r', encoding='utf-8') as f:
                for line_num, line in enumerate(f, 1):
                    line = line.strip()
                    if not line:
                        continue
                        
                    try:
                        event = json.loads(line)
                        self.events.append(event)
                        
                        # Filter Claude-related events
                        if self.is_claude_event(event):
                            self.claude_events.append(event)
                            
                        # Filter chunk_merger events
                        if self.is_chunk_merger_event(event):
                            self.chunk_merger_events.append(event)
                            
                    except json.JSONDecodeError as e:
                        print(f"Warning: Invalid JSON on line {line_num}: {e}")
                        continue
                        
        except FileNotFoundError:
            print(f"Error: Log file '{self.log_file}' not found")
            sys.exit(1)
        except Exception as e:
            print(f"Error reading log file: {e}")
            sys.exit(1)
            
    def is_claude_event(self, event: Dict[str, Any]) -> bool:
        """Check if an event is Claude-related"""
        data = event.get('data', {})
        comm = data.get('comm', '')
        
        # If specific process filter is provided, use it
        if self.process_filter:
            return self.process_filter.lower() in comm.lower()
        
        # Default Claude filtering logic
        # Check process name (comm field)
        if comm == 'claude':
            return True
            
        # Check for Claude-related processes and content
        claude_indicators = [
            'claude',
            'claude-code',
            'claude-cli',
            '@anthropic-ai/claude-code'
        ]
        
        # Check command line or process name
        if any(indicator in comm.lower() for indicator in claude_indicators):
            return True
            
        # For SSL events, also check the data content for Claude-related domains
        if event.get('source') == 'ssl':
            ssl_data = data.get('data', '')
            if ssl_data and isinstance(ssl_data, str):
                claude_domains = [
                    'anthropic.com',
                    'api.anthropic.com',
                    'statsig.anthropic.com',
                    'claude-sonnet',
                    'claude-code'
                ]
                
                ssl_data_lower = ssl_data.lower()
                if any(domain in ssl_data_lower for domain in claude_domains):
                    return True
        
        return False
        
    def is_chunk_merger_event(self, event: Dict[str, Any]) -> bool:
        """Check if an event is from chunk_merger"""
        return event.get('source') == 'chunk_merger'
        
    def group_conversations(self) -> None:
        """Group chunk merger events by connection/message ID"""
        for event in self.chunk_merger_events:
            data = event.get('data', {})
            connection_id = data.get('connection_id', 'unknown')
            message_id = data.get('message_id')
            
            # Use message_id if available, otherwise connection_id
            key = message_id if message_id else connection_id
            self.conversations[key].append(event)
            
    def format_timestamp(self, timestamp_ns: Optional[int]) -> str:
        """Format timestamp from nanoseconds to readable format"""
        if not timestamp_ns:
            return "Unknown"
        try:
            # Convert nanoseconds to seconds
            timestamp_s = timestamp_ns / 1_000_000_000
            dt = datetime.fromtimestamp(timestamp_s)
            return dt.strftime("%Y-%m-%d %H:%M:%S.%f")[:-3]  # Remove last 3 digits
        except:
            return f"Invalid timestamp: {timestamp_ns}"
            
    def extract_http_request_info(self, data_str: str) -> Dict[str, str]:
        """Extract HTTP request information from SSL data"""
        lines = data_str.split('\\r\\n')
        if not lines:
            return {}
            
        # Parse first line (request line)
        request_line = lines[0]
        parts = request_line.split(' ')
        if len(parts) >= 3:
            method = parts[0]
            url = parts[1]
            version = parts[2]
            
            return {
                'method': method,
                'url': url,
                'http_version': version,
                'is_request': True
            }
            
        # Check if it's a response
        if request_line.startswith('HTTP/'):
            parts = request_line.split(' ', 2)
            if len(parts) >= 2:
                return {
                    'http_version': parts[0],
                    'status_code': parts[1],
                    'status_text': parts[2] if len(parts) > 2 else '',
                    'is_response': True
                }
                
        return {}
        
    def print_claude_messages(self) -> None:
        """Print all Claude-related merged content in a readable format"""
        print("=" * 80)
        print("🤖 CLAUDE MESSAGE CONTENT ANALYSIS")
        print("=" * 80)
        
        if not self.chunk_merger_events:
            print("❌ No chunk merger events found!")
            print("\nRaw Claude SSL events found:", len(self.claude_events))
            
            # Show some raw SSL events for debugging
            if self.claude_events:
                print("\n📡 Sample Raw SSL Events:")
                for i, event in enumerate(self.claude_events[:3]):
                    data = event.get('data', {})
                    print(f"\n{i+1}. Function: {data.get('function', 'unknown')}")
                    print(f"   PID: {data.get('pid')}, TID: {data.get('tid')}")
                    print(f"   Timestamp: {self.format_timestamp(data.get('timestamp_ns'))}")
                    
                    http_info = self.extract_http_request_info(data.get('data', ''))
                    if http_info:
                        if http_info.get('is_request'):
                            print(f"   HTTP: {http_info.get('method')} {http_info.get('url')}")
                        elif http_info.get('is_response'):
                            print(f"   HTTP: {http_info.get('status_code')} {http_info.get('status_text')}")
                            
                    # Show first 200 chars of data
                    raw_data = data.get('data', '')
                    if len(raw_data) > 200:
                        print(f"   Data: {raw_data[:200]}...")
                    else:
                        print(f"   Data: {raw_data}")
            return
            
        # Group events by conversation
        self.group_conversations()
        
        print(f"📊 Summary:")
        print(f"   • Total events: {len(self.events)}")
        print(f"   • Claude SSL events: {len(self.claude_events)}")
        print(f"   • Chunk merger events: {len(self.chunk_merger_events)}")
        print(f"   • Conversations found: {len(self.conversations)}")
        
        print(f"\n💬 CLAUDE CONVERSATIONS:")
        print("-" * 80)
        
        # Sort conversations by timestamp
        sorted_conversations = sorted(
            self.conversations.items(),
            key=lambda x: min(event.get('data', {}).get('timestamp_ns', 0) for event in x[1])
        )
        
        for conv_id, events in sorted_conversations:
            print(f"\n🗨️  Conversation: {conv_id}")
            print(f"   Events: {len(events)}")
            
            # Sort events within conversation by timestamp
            events.sort(key=lambda x: x.get('data', {}).get('timestamp_ns', 0))
            
            for i, event in enumerate(events, 1):
                data = event.get('data', {})
                
                print(f"\n   📝 Message {i}:")
                print(f"      Connection ID: {data.get('connection_id', 'unknown')}")
                print(f"      Message ID: {data.get('message_id', 'None')}")
                print(f"      Content Type: {data.get('content_type', 'unknown')}")
                print(f"      Function: {data.get('function', 'unknown')}")
                print(f"      PID/TID: {data.get('pid')}/{data.get('tid')}")
                print(f"      Timestamp: {self.format_timestamp(data.get('timestamp_ns'))}")
                print(f"      Event Count: {data.get('event_count', 0)}")
                print(f"      Total Size: {data.get('total_size', 0)} bytes")
                print(f"      Has Message Start: {data.get('has_message_start', False)}")
                
                # Show merged content
                merged_content = data.get('merged_content', '')
                if merged_content:
                    print(f"      📄 Content:")
                    if data.get('content_type') == 'json':
                        try:
                            # Try to pretty-print JSON
                            json_obj = json.loads(merged_content)
                            pretty_json = json.dumps(json_obj, indent=8, ensure_ascii=False)
                            print(f"         {pretty_json}")
                        except json.JSONDecodeError:
                            # If not valid JSON, show as text
                            print(f"         {merged_content}")
                    else:
                        print(f"         \"{merged_content}\"")
                else:
                    print(f"      📄 Content: (empty)")
                    
                # Show SSE events summary
                sse_events = data.get('sse_events', [])
                if sse_events:
                    print(f"      🔄 SSE Events ({len(sse_events)}):")
                    event_types = {}
                    for sse_event in sse_events:
                        event_type = sse_event.get('type', 'unknown')
                        event_types[event_type] = event_types.get(event_type, 0) + 1
                    
                    for event_type, count in event_types.items():
                        print(f"         • {event_type}: {count}")
                        
        print("\n" + "=" * 80)
        print("✅ Analysis complete!")
        
    def print_process_statistics(self) -> None:
        """Print process-specific statistics for Claude filtering"""
        print(f"\n🔍 PROCESS FILTERING STATISTICS:")
        print("-" * 40)
        
        # Analyze all processes
        all_processes = defaultdict(int)
        claude_processes = defaultdict(int)
        process_pids = defaultdict(set)
        
        for event in self.events:
            data = event.get('data', {})
            comm = data.get('comm', 'unknown')
            pid = data.get('pid')
            
            all_processes[comm] += 1
            if pid:
                process_pids[comm].add(pid)
                
            if self.is_claude_event(event):
                claude_processes[comm] += 1
                
        print(f"Total processes found: {len(all_processes)}")
        print(f"Claude-related processes: {len(claude_processes)}")
        
        print(f"\n📊 All Processes (top 10):")
        sorted_processes = sorted(all_processes.items(), key=lambda x: x[1], reverse=True)
        for comm, count in sorted_processes[:10]:
            pid_count = len(process_pids[comm])
            marker = "🤖" if comm in claude_processes else "  "
            print(f"  {marker} {comm}: {count} events ({pid_count} PIDs)")
            
        if claude_processes:
            print(f"\n🤖 Claude Processes:")
            for comm, count in claude_processes.items():
                pid_count = len(process_pids[comm])
                pids = sorted(list(process_pids[comm]))
                print(f"  • {comm}: {count} events")
                print(f"    PIDs: {pids}")
        
    def print_statistics(self) -> None:
        """Print detailed statistics about the log"""
        print(f"\n📈 DETAILED STATISTICS:")
        print("-" * 40)
        
        # Message ID analysis
        message_ids = set()
        connection_ids = set()
        content_types = defaultdict(int)
        functions = defaultdict(int)
        
        for event in self.chunk_merger_events:
            data = event.get('data', {})
            if data.get('message_id'):
                message_ids.add(data.get('message_id'))
            if data.get('connection_id'):
                connection_ids.add(data.get('connection_id'))
            content_types[data.get('content_type', 'unknown')] += 1
            functions[data.get('function', 'unknown')] += 1
            
        print(f"Unique Message IDs: {len(message_ids)}")
        print(f"Unique Connection IDs: {len(connection_ids)}")
        
        print(f"\nContent Types:")
        for content_type, count in content_types.items():
            print(f"  • {content_type}: {count}")
            
        print(f"\nFunctions:")
        for function, count in functions.items():
            print(f"  • {function}: {count}")

def main():
    """Main function"""
    # Parse command line arguments
    process_filter = None
    log_file = None
    
    args = sys.argv[1:]
    i = 0
    while i < len(args):
        if args[i] == '--process' or args[i] == '-p':
            if i + 1 < len(args):
                process_filter = args[i + 1]
                i += 2
            else:
                print("❌ Error: --process requires a process name")
                sys.exit(1)
        elif args[i].startswith('--process='):
            process_filter = args[i].split('=', 1)[1]
            i += 1
        elif not log_file and not args[i].startswith('-'):
            log_file = args[i]
            i += 1
        else:
            print(f"❌ Error: Unknown argument: {args[i]}")
            print("Usage: python3 claude_analyzer.py [ssl_log_file] [--process PROCESS_NAME]")
            sys.exit(1)
    
    # Determine log file if not provided
    if not log_file:
        # Look for ssl.log in current directory or collector subdirectory
        candidates = ['ssl.log', 'collector/ssl.log', '../ssl.log']
        for candidate in candidates:
            if os.path.exists(candidate):
                log_file = candidate
                break
                
        if not log_file:
            print("❌ Error: No SSL log file found!")
            print("Usage: python3 claude_analyzer.py [ssl_log_file] [--process PROCESS_NAME]")
            print("Or place ssl.log in the current directory")
            sys.exit(1)
            
    print(f"📂 Analyzing: {log_file}")
    if process_filter:
        print(f"🔍 Filtering by process: {process_filter}")
    
    # Create analyzer and process
    analyzer = ClaudeAnalyzer(log_file, process_filter)
    analyzer.parse_log_file()
    analyzer.print_process_statistics()
    analyzer.print_claude_messages()
    analyzer.print_statistics()

if __name__ == "__main__":
    main() 