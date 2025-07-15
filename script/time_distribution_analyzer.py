#!/usr/bin/env python3
"""
Time Distribution Analyzer for SSL Logs

Analyzes the temporal distribution of SSL log events to understand:
- Why the time span appears to be 3 hours
- Whether timestamps are system uptime vs wall clock
- Event distribution patterns over time
"""

import json
import sys
from datetime import datetime, timedelta
from collections import defaultdict, Counter
import argparse

try:
    import matplotlib.pyplot as plt
    import numpy as np
    HAS_MATPLOTLIB = True
except ImportError:
    HAS_MATPLOTLIB = False


def analyze_timestamps(log_file):
    """Analyze timestamp patterns in SSL log."""
    timestamps = []
    sources = []
    
    print(f"Analyzing timestamps in: {log_file}")
    
    with open(log_file, 'r') as f:
        for line_num, line in enumerate(f, 1):
            try:
                entry = json.loads(line.strip())
                ts = entry.get('timestamp', 0)
                source = entry.get('source', 'unknown')
                
                if ts > 0:
                    timestamps.append(ts)
                    sources.append(source)
                    
            except json.JSONDecodeError:
                continue
    
    if not timestamps:
        print("No valid timestamps found")
        return
    
    # Convert to seconds for easier analysis
    timestamps_s = [ts / 1_000_000_000 for ts in timestamps]
    min_ts = min(timestamps_s)
    max_ts = max(timestamps_s)
    duration = max_ts - min_ts
    
    print(f"\n{'='*60}")
    print("TIMESTAMP ANALYSIS")
    print(f"{'='*60}")
    
    print(f"Raw timestamp range:")
    print(f"  Min: {min(timestamps):,} ns")
    print(f"  Max: {max(timestamps):,} ns") 
    print(f"  Duration: {duration:,.2f} seconds ({duration/3600:.2f} hours)")
    
    print(f"\nAs epoch time (likely incorrect):")
    print(f"  Start: {datetime.fromtimestamp(min_ts)}")
    print(f"  End:   {datetime.fromtimestamp(max_ts)}")
    
    print(f"\nTimestamp interpretation:")
    if min_ts < 86400 * 365 * 10:  # Less than 10 years since epoch
        print("  These appear to be SYSTEM UPTIME timestamps")
        print("  (nanoseconds since system boot, not wall clock time)")
        print(f"  System had been running for {min_ts/3600:.1f} hours when logging started")
        print(f"  Logging session lasted {duration/60:.1f} minutes")
    else:
        print("  These appear to be wall clock timestamps")
    
    return timestamps, sources, timestamps_s


def analyze_distribution(timestamps_s, sources):
    """Analyze temporal distribution of events."""
    print(f"\n{'='*60}")
    print("TIME DISTRIBUTION ANALYSIS")
    print(f"{'='*60}")
    
    # Normalize timestamps to start at 0
    min_ts = min(timestamps_s)
    relative_times = [ts - min_ts for ts in timestamps_s]
    duration = max(relative_times)
    
    print(f"Session duration: {duration:.1f} seconds ({duration/60:.1f} minutes)")
    
    # Create time buckets (1-minute intervals)
    bucket_size = 60  # seconds
    num_buckets = int(duration / bucket_size) + 1
    bucket_counts = defaultdict(int)
    bucket_sources = defaultdict(lambda: defaultdict(int))
    
    for i, rel_time in enumerate(relative_times):
        bucket = int(rel_time / bucket_size)
        bucket_counts[bucket] += 1
        bucket_sources[bucket][sources[i]] += 1
    
    # Find periods of activity
    active_buckets = [(b, count) for b, count in bucket_counts.items() if count > 0]
    active_buckets.sort()
    
    print(f"\nActivity periods (1-minute buckets):")
    print(f"  Total buckets with activity: {len(active_buckets)}")
    print(f"  Total possible buckets: {num_buckets}")
    print(f"  Activity coverage: {len(active_buckets)/num_buckets*100:.1f}%")
    
    # Show top active periods
    top_periods = sorted(active_buckets, key=lambda x: x[1], reverse=True)[:10]
    print(f"\nTop 10 most active minutes:")
    for bucket, count in top_periods:
        minute = bucket * bucket_size / 60
        sources_in_bucket = bucket_sources[bucket]
        source_items = sorted(bucket_sources[bucket].items(), key=lambda x: x[1], reverse=True)[:3]
        source_summary = ", ".join([f"{src}:{cnt}" for src, cnt in source_items])
        print(f"  Minute {minute:6.1f}: {count:3d} events ({source_summary})")
    
    # Analyze gaps
    if len(active_buckets) > 1:
        gaps = []
        for i in range(1, len(active_buckets)):
            prev_bucket = active_buckets[i-1][0]
            curr_bucket = active_buckets[i][0]
            gap = curr_bucket - prev_bucket - 1
            if gap > 0:
                gaps.append(gap * bucket_size)
        
        if gaps:
            print(f"\nInactivity gaps:")
            print(f"  Number of gaps: {len(gaps)}")
            avg_gap = sum(gaps) / len(gaps)
            print(f"  Average gap: {avg_gap:.1f} seconds")
            print(f"  Longest gap: {max(gaps):.1f} seconds ({max(gaps)/60:.1f} minutes)")
            print(f"  Total inactive time: {sum(gaps):.1f} seconds ({sum(gaps)/60:.1f} minutes)")
    
    return relative_times, bucket_counts, bucket_sources


def create_timeline_plot(relative_times, sources, output_file=None):
    """Create a timeline visualization."""
    if not HAS_MATPLOTLIB:
        print("\nMatplotlib not available - skipping visualization")
        print("Install with: pip install matplotlib")
        return
        
    try:
        # Create figure with subplots
        fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 8))
        
        # Plot 1: Event density over time
        ax1.hist(relative_times, bins=50, alpha=0.7, edgecolor='black')
        ax1.set_xlabel('Time (seconds from start)')
        ax1.set_ylabel('Event Count')
        ax1.set_title('SSL Log Event Distribution Over Time')
        ax1.grid(True, alpha=0.3)
        
        # Plot 2: Events by source over time
        source_types = list(set(sources))
        colors = plt.cm.Set3([i/len(source_types) for i in range(len(source_types))])
        
        for i, source in enumerate(source_types):
            source_times = [relative_times[j] for j, s in enumerate(sources) if s == source]
            if source_times:
                ax2.scatter(source_times, [i] * len(source_times), 
                           alpha=0.6, s=20, label=source, color=colors[i])
        
        ax2.set_xlabel('Time (seconds from start)')
        ax2.set_ylabel('Event Source')
        ax2.set_title('SSL Log Events by Source Over Time')
        ax2.set_yticks(range(len(source_types)))
        ax2.set_yticklabels(source_types)
        ax2.legend()
        ax2.grid(True, alpha=0.3)
        
        plt.tight_layout()
        
        if output_file:
            plt.savefig(output_file, dpi=150, bbox_inches='tight')
            print(f"\nTimeline plot saved to: {output_file}")
        else:
            plt.show()
            
    except ImportError:
        print("\nMatplotlib not available - skipping visualization")
        print("Install with: pip install matplotlib")


def main():
    parser = argparse.ArgumentParser(description='Analyze time distribution in SSL logs')
    parser.add_argument('log_file', help='SSL log file to analyze')
    parser.add_argument('--plot', help='Save timeline plot to file (e.g., timeline.png)')
    parser.add_argument('--no-plot', action='store_true', help='Skip plot generation')
    
    args = parser.parse_args()
    
    try:
        timestamps, sources, timestamps_s = analyze_timestamps(args.log_file)
        
        if timestamps:
            relative_times, bucket_counts, bucket_sources = analyze_distribution(timestamps_s, sources)
            
            if not args.no_plot:
                create_timeline_plot(relative_times, sources, args.plot)
                
    except FileNotFoundError:
        print(f"Error: File '{args.log_file}' not found")
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)


if __name__ == '__main__':
    main()