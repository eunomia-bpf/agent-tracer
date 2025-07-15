#!/usr/bin/env python3
"""
Test JSON Structure

Verifies that the JSON output from events exactly matches the struct fields.
"""

import json
import sys


def test_json_structure(log_file):
    """Test that event JSON matches expected struct fields."""
    
    expected_sse_fields = {
        'connection_id', 'message_id', 'start_time', 'end_time', 'duration_ns',
        'original_source', 'function', 'tid', 'json_content', 'text_content',
        'total_size', 'event_count', 'has_message_start', 'sse_events'
    }
    
    expected_http_fields = {
        'tid', 'message_type', 'first_line', 'method', 'path', 'protocol',
        'status_code', 'status_text', 'headers', 'body', 'total_size',
        'has_body', 'is_chunked', 'content_length', 'original_source', 'raw_data'
    }
    
    sse_events = []
    http_events = []
    
    with open(log_file, 'r') as f:
        for line_num, line in enumerate(f, 1):
            try:
                entry = json.loads(line.strip())
                source = entry.get('source', '')
                data = entry.get('data', {})
                
                if source == 'sse_processor':
                    sse_events.append((line_num, data))
                elif source == 'http_parser':
                    http_events.append((line_num, data))
                    
            except json.JSONDecodeError:
                continue
    
    print(f"Found {len(sse_events)} SSE events and {len(http_events)} HTTP events")
    
    # Test SSE events
    if sse_events:
        line_num, data = sse_events[0]
        actual_fields = set(data.keys())
        
        print(f"\nSSE Event Fields (line {line_num}):")
        print(f"Expected: {sorted(expected_sse_fields)}")
        print(f"Actual:   {sorted(actual_fields)}")
        
        missing = expected_sse_fields - actual_fields
        extra = actual_fields - expected_sse_fields
        
        if missing:
            print(f"❌ Missing fields: {missing}")
        if extra:
            print(f"⚠️  Extra fields: {extra}")
        if not missing and not extra:
            print(f"✅ SSE fields match exactly!")
    
    # Test HTTP events
    if http_events:
        line_num, data = http_events[0]
        actual_fields = set(data.keys())
        
        print(f"\nHTTP Event Fields (line {line_num}):")
        print(f"Expected: {sorted(expected_http_fields)}")
        print(f"Actual:   {sorted(actual_fields)}")
        
        missing = expected_http_fields - actual_fields
        extra = actual_fields - expected_http_fields
        
        if missing:
            print(f"❌ Missing fields: {missing}")
        if extra:
            print(f"⚠️  Extra fields: {extra}")
        if not missing and not extra:
            print(f"✅ HTTP fields match exactly!")
    
    return len(sse_events) > 0 or len(http_events) > 0


def main():
    if len(sys.argv) != 2:
        print("Usage: python3 test_json_structure.py <ssl_log_file>")
        sys.exit(1)
    
    log_file = sys.argv[1]
    
    try:
        found_events = test_json_structure(log_file)
        if found_events:
            print(f"\n✅ JSON structure test completed!")
        else:
            print(f"\n⚠️  No SSE or HTTP events found in log")
            
    except FileNotFoundError:
        print(f"Error: File '{log_file}' not found")
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)


if __name__ == '__main__':
    main()