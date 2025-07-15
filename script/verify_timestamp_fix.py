#!/usr/bin/env python3
"""
Verify Timestamp Fix

This script analyzes SSL logs to verify that timestamps are preserved correctly
after the SSE processor and HTTP parser fixes.
"""

import json
import sys
from collections import defaultdict


def analyze_timestamp_consistency(log_file):
    """Analyze timestamp consistency across different event sources."""
    events = []
    
    with open(log_file, 'r') as f:
        for line_num, line in enumerate(f, 1):
            try:
                entry = json.loads(line.strip())
                timestamp = entry.get('timestamp', 0)
                source = entry.get('source', 'unknown')
                data = entry.get('data', {})
                
                events.append({
                    'line': line_num,
                    'timestamp': timestamp,
                    'source': source,
                    'pid': data.get('pid', 0),
                    'tid': data.get('tid', 0),
                    'original_timestamp_ns': data.get('timestamp_ns', 0),
                })
            except json.JSONDecodeError:
                continue
    
    print(f"Analyzing {len(events)} events for timestamp consistency...")
    
    # Group events by PID/TID
    grouped_events = defaultdict(list)
    for event in events:
        key = f"{event['pid']}-{event['tid']}"
        grouped_events[key].append(event)
    
    # Analyze timestamp patterns
    issues = []
    for group_key, group_events in grouped_events.items():
        if len(group_events) < 2:
            continue
            
        # Sort by line number (original order)
        group_events.sort(key=lambda x: x['line'])
        
        # Check for timestamp inconsistencies
        for i in range(1, len(group_events)):
            prev_event = group_events[i-1]
            curr_event = group_events[i]
            
            # If events are very close in line order but far apart in timestamp,
            # this suggests timestamp manipulation
            line_diff = curr_event['line'] - prev_event['line']
            timestamp_diff = abs(curr_event['timestamp'] - prev_event['timestamp'])
            
            # Flag suspicious gaps (>1 hour between closely ordered events)
            if line_diff <= 10 and timestamp_diff > 3600000:  # 1 hour in ms
                issues.append({
                    'type': 'suspicious_gap',
                    'prev_event': prev_event,
                    'curr_event': curr_event,
                    'line_diff': line_diff,
                    'timestamp_diff_ms': timestamp_diff,
                    'timestamp_diff_hours': timestamp_diff / 3600000
                })
    
    # Analyze by source
    source_stats = defaultdict(list)
    for event in events:
        source_stats[event['source']].append(event['timestamp'])
    
    print(f"\nTimestamp Statistics by Source:")
    print(f"{'Source':<15} {'Count':<8} {'Min Timestamp':<15} {'Max Timestamp':<15} {'Range (hours)':<12}")
    print("-" * 80)
    
    for source, timestamps in source_stats.items():
        if timestamps:
            min_ts = min(timestamps)
            max_ts = max(timestamps)
            range_hours = (max_ts - min_ts) / 3600000
            
            print(f"{source:<15} {len(timestamps):<8} {min_ts:<15} {max_ts:<15} {range_hours:<12.2f}")
    
    # Report issues
    if issues:
        print(f"\n⚠️  Found {len(issues)} timestamp inconsistencies:")
        for i, issue in enumerate(issues, 1):
            prev = issue['prev_event']
            curr = issue['curr_event']
            print(f"\n{i}. Suspicious gap between lines {prev['line']} and {curr['line']}:")
            print(f"   Previous: {prev['source']} at timestamp {prev['timestamp']}")
            print(f"   Current:  {curr['source']} at timestamp {curr['timestamp']}")
            print(f"   Line difference: {issue['line_diff']}")
            print(f"   Time gap: {issue['timestamp_diff_hours']:.2f} hours")
    else:
        print(f"\n✅ No major timestamp inconsistencies detected!")
    
    # Check for events that should be simultaneous
    print(f"\nChecking for simultaneous events (same PID/TID, close timestamps):")
    
    simultaneous_groups = defaultdict(list)
    for event in events:
        # Group by PID/TID and timestamp bucket (within 1 second)
        bucket = event['timestamp'] // 1000  # 1-second buckets
        key = f"{event['pid']}-{event['tid']}-{bucket}"
        simultaneous_groups[key].append(event)
    
    simultaneous_count = 0
    for group_key, group_events in simultaneous_groups.items():
        if len(group_events) > 1:
            simultaneous_count += 1
            if simultaneous_count <= 5:  # Show first 5 examples
                print(f"\nSimultaneous events group {simultaneous_count}:")
                for event in group_events:
                    print(f"  Line {event['line']}: {event['source']} at {event['timestamp']}")
    
    if simultaneous_count > 5:
        print(f"  ... and {simultaneous_count - 5} more groups")
    
    return len(issues) == 0


def main():
    if len(sys.argv) != 2:
        print("Usage: python3 verify_timestamp_fix.py <ssl_log_file>")
        sys.exit(1)
    
    log_file = sys.argv[1]
    
    try:
        is_consistent = analyze_timestamp_consistency(log_file)
        
        if is_consistent:
            print(f"\n✅ Timestamp consistency check PASSED!")
            sys.exit(0)
        else:
            print(f"\n❌ Timestamp consistency check FAILED!")
            sys.exit(1)
            
    except FileNotFoundError:
        print(f"Error: File '{log_file}' not found")
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)


if __name__ == '__main__':
    main()