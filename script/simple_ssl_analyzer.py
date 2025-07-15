#!/usr/bin/env python3
"""
Simple SSL Log Analyzer

Extracts key information from SSL logs: time, comm, pid/tid, source, etc.
Provides a quick overview of SSL traffic data.
"""

import json
import sys
import argparse
from datetime import datetime
from collections import Counter
import csv


def parse_timestamp(timestamp_ns):
    """Convert nanosecond timestamp to readable format."""
    try:
        timestamp_s = timestamp_ns / 1_000_000_000
        return datetime.fromtimestamp(timestamp_s).strftime('%Y-%m-%d %H:%M:%S.%f')[:-3]
    except (ValueError, OSError):
        return "Invalid timestamp"


def extract_key_info(line_num, entry):
    """Extract key information from a log entry."""
    try:
        data = entry.get('data', {})
        source = entry.get('source', 'unknown')
        timestamp = entry.get('timestamp', 0)
        
        info = {
            'line': line_num,
            'timestamp_ns': timestamp,
            'timestamp': parse_timestamp(timestamp),
            'source': source,
            'comm': data.get('comm', ''),
            'pid': data.get('pid', 0),
            'tid': data.get('tid', 0),
        }
        
        # Add source-specific details
        if source == 'http_parser':
            info.update({
                'method': data.get('method', ''),
                'path': data.get('path', ''),
                'host': data.get('headers', {}).get('host', ''),
                'status_code': data.get('status_code', ''),
                'content_length': data.get('content_length', 0),
                'message_type': data.get('message_type', ''),
            })
        elif source == 'ssl':
            info.update({
                'function': data.get('function', ''),
                'data_len': data.get('len', 0),
                'latency_ms': data.get('latency_ms', 0),
                'is_handshake': data.get('is_handshake', False),
            })
        elif source == 'sse_processor':
            info.update({
                'connection_id': data.get('connection_id', ''),
                'duration_ms': data.get('duration_ms', 0),
                'event_count': data.get('event_count', 0),
                'function': data.get('function', ''),
            })
            
        return info
        
    except Exception as e:
        return {
            'line': line_num,
            'error': f"Failed to parse: {e}",
            'timestamp': '',
            'source': 'error',
            'comm': '',
            'pid': 0,
            'tid': 0,
        }


def analyze_ssl_log(log_file):
    """Analyze SSL log file and extract key information."""
    entries = []
    
    print(f"Analyzing: {log_file}")
    
    with open(log_file, 'r') as f:
        for line_num, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
                
            try:
                entry = json.loads(line)
                info = extract_key_info(line_num, entry)
                entries.append(info)
            except json.JSONDecodeError:
                entries.append({
                    'line': line_num,
                    'error': 'Invalid JSON',
                    'timestamp': '',
                    'source': 'error',
                    'comm': '',
                    'pid': 0,
                    'tid': 0,
                })
    
    return entries


def print_summary(entries):
    """Print summary statistics."""
    print(f"\n{'='*60}")
    print("SUMMARY")
    print(f"{'='*60}")
    
    total = len(entries)
    errors = len([e for e in entries if 'error' in e])
    valid = total - errors
    
    print(f"Total entries: {total}")
    print(f"Valid entries: {valid}")
    print(f"Parse errors: {errors}")
    
    if valid == 0:
        return
    
    valid_entries = [e for e in entries if 'error' not in e]
    
    # Sources
    sources = Counter(e['source'] for e in valid_entries)
    print(f"\nSources:")
    for source, count in sources.most_common():
        print(f"  {source}: {count}")
    
    # Commands
    comms = Counter(e['comm'] for e in valid_entries if e['comm'])
    print(f"\nTop commands:")
    for comm, count in comms.most_common(5):
        print(f"  {comm}: {count}")
    
    # PIDs
    pids = Counter(e['pid'] for e in valid_entries if e['pid'] > 0)
    print(f"\nTop PIDs:")
    for pid, count in pids.most_common(5):
        print(f"  {pid}: {count}")
    
    # Time range
    timestamps = [e['timestamp_ns'] for e in valid_entries if e['timestamp_ns'] > 0]
    if timestamps:
        min_ts = min(timestamps)
        max_ts = max(timestamps)
        duration = (max_ts - min_ts) / 1_000_000_000
        print(f"\nTime range:")
        print(f"  Start: {parse_timestamp(min_ts)}")
        print(f"  End:   {parse_timestamp(max_ts)}")
        print(f"  Duration: {duration:.2f} seconds")


def print_details(entries, limit=20):
    """Print detailed information for first N entries."""
    print(f"\n{'='*60}")
    print(f"DETAILED VIEW (first {limit} entries)")
    print(f"{'='*60}")
    
    for i, entry in enumerate(entries[:limit], 1):
        print(f"\n[{i}] Line {entry['line']}")
        
        if 'error' in entry:
            print(f"  ERROR: {entry['error']}")
            continue
            
        print(f"  Time: {entry['timestamp']}")
        print(f"  Source: {entry['source']}")
        print(f"  Command: {entry['comm']}")
        print(f"  PID/TID: {entry['pid']}/{entry['tid']}")
        
        # Source-specific details
        if entry['source'] == 'http_parser':
            if entry.get('method'):
                print(f"  HTTP: {entry['method']} {entry.get('path', '')}")
            if entry.get('host'):
                print(f"  Host: {entry['host']}")
            if entry.get('status_code'):
                print(f"  Status: {entry['status_code']}")
            if entry.get('content_length'):
                print(f"  Size: {entry['content_length']} bytes")
                
        elif entry['source'] == 'ssl':
            if entry.get('function'):
                print(f"  Function: {entry['function']}")
            if entry.get('data_len'):
                print(f"  Data: {entry['data_len']} bytes")
            if entry.get('latency_ms'):
                print(f"  Latency: {entry['latency_ms']:.3f} ms")
                
        elif entry['source'] == 'sse_processor':
            if entry.get('duration_ms'):
                print(f"  Duration: {entry['duration_ms']:.3f} ms")
            if entry.get('event_count'):
                print(f"  Events: {entry['event_count']}")
        
        print("  " + "-" * 40)


def export_csv(entries, output_file):
    """Export entries to CSV."""
    if not entries:
        print("No entries to export")
        return
    
    # Define CSV columns
    columns = [
        'line', 'timestamp', 'source', 'comm', 'pid', 'tid',
        'method', 'path', 'host', 'status_code', 'content_length',
        'function', 'data_len', 'latency_ms', 'duration_ms', 'event_count'
    ]
    
    with open(output_file, 'w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=columns, extrasaction='ignore')
        writer.writeheader()
        
        for entry in entries:
            if 'error' not in entry:
                writer.writerow(entry)
    
    valid_count = len([e for e in entries if 'error' not in e])
    print(f"\nExported {valid_count} valid entries to {output_file}")


def main():
    parser = argparse.ArgumentParser(description='Simple SSL log analyzer')
    parser.add_argument('log_file', help='SSL log file to analyze')
    parser.add_argument('--csv', help='Export to CSV file')
    parser.add_argument('--limit', type=int, default=20, help='Limit detailed view (default: 20)')
    parser.add_argument('--summary-only', action='store_true', help='Show summary only')
    
    args = parser.parse_args()
    
    try:
        entries = analyze_ssl_log(args.log_file)
        print_summary(entries)
        
        if not args.summary_only:
            print_details(entries, args.limit)
        
        if args.csv:
            export_csv(entries, args.csv)
            
    except FileNotFoundError:
        print(f"Error: File '{args.log_file}' not found")
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)


if __name__ == '__main__':
    main()