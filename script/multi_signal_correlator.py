#!/usr/bin/env python3
"""
Multi-Signal Causal Correlation Engine for AgentSight

Implements the core correlation strategy from the AgentSight paper (Section 3.2) that 
establishes causality between intent and action using three key mechanisms:

1. Process Lineage: Builds complete process tree tracking fork/execve events
2. Temporal Proximity: Associates actions within 100-500ms window after LLM response
3. Argument Matching: Matches content from LLM responses with system call arguments

Based on AgentSight paper Section 3.2 - "Multi-Signal Causal Correlation Engine"
"""

import json
import sys
import argparse
import re
import os
from typing import Dict, List, Any, Optional, Tuple, Set
from dataclasses import dataclass, field
from datetime import datetime
from collections import defaultdict, deque
import subprocess


@dataclass
class ProcessNode:
    """Represents a process in the process tree"""
    pid: int
    ppid: int
    tid: int
    comm: str
    start_time: int
    children: List['ProcessNode'] = field(default_factory=list)
    parent: Optional['ProcessNode'] = None
    actions: List[Dict[str, Any]] = field(default_factory=list)


@dataclass
class LLMInteraction:
    """Represents an LLM request/response pair"""
    request_timestamp: int
    response_timestamp: int
    request_data: Dict[str, Any]
    response_data: Dict[str, Any]
    tid: int
    pid: int
    comm: str
    extracted_references: Set[str] = field(default_factory=set)


@dataclass
class CorrelatedEvent:
    """Represents a correlated intent-action pair"""
    llm_interaction: LLMInteraction
    system_actions: List[Dict[str, Any]]
    correlation_score: float
    correlation_type: str  # "process_lineage", "temporal_proximity", "argument_matching"
    evidence: Dict[str, Any]


class ProcessTreeBuilder:
    """Builds and maintains process tree from process events"""
    
    def __init__(self):
        self.processes: Dict[int, ProcessNode] = {}
        self.root_processes: List[ProcessNode] = []
    
    def add_process_event(self, event: Dict[str, Any]):
        """Add process creation/execution event to tree"""
        data = event.get('data', {})
        pid = data.get('pid', 0)
        ppid = data.get('ppid', 0)
        tid = data.get('tid', pid)
        comm = data.get('comm', 'unknown')
        timestamp = event.get('timestamp', 0)
        
        # Create or update process node
        if pid not in self.processes:
            self.processes[pid] = ProcessNode(
                pid=pid, ppid=ppid, tid=tid, comm=comm, start_time=timestamp
            )
        
        node = self.processes[pid]
        
        # Establish parent-child relationships
        if ppid > 0 and ppid in self.processes:
            parent = self.processes[ppid]
            if node not in parent.children:
                parent.children.append(node)
            node.parent = parent
        elif ppid == 0 or ppid not in self.processes:
            # Root process or parent not yet seen
            if node not in self.root_processes:
                self.root_processes.append(node)
    
    def find_process_lineage(self, pid: int) -> List[ProcessNode]:
        """Find complete lineage from root to given process"""
        if pid not in self.processes:
            return []
        
        lineage = []
        current = self.processes[pid]
        
        while current:
            lineage.insert(0, current)  # Insert at beginning for root-to-leaf order
            current = current.parent
        
        return lineage
    
    def find_related_processes(self, pid: int) -> Set[int]:
        """Find all processes related to given PID (parent, children, siblings)"""
        related = set()
        
        if pid not in self.processes:
            return related
        
        node = self.processes[pid]
        related.add(pid)
        
        # Add parent
        if node.parent:
            related.add(node.parent.pid)
        
        # Add children (recursive)
        def add_children(n: ProcessNode):
            for child in n.children:
                related.add(child.pid)
                add_children(child)
        
        add_children(node)
        
        # Add siblings (same parent)
        if node.parent:
            for sibling in node.parent.children:
                related.add(sibling.pid)
        
        return related


class ArgumentMatcher:
    """Matches content from LLM responses with system call arguments"""
    
    def __init__(self):
        self.file_patterns = [
            r'([/~][a-zA-Z0-9_\-./]+)',  # File paths
            r'([a-zA-Z0-9_\-]+\.[a-zA-Z]{2,4})',  # Filenames with extensions
        ]
        self.command_patterns = [
            r'\b(cat|ls|grep|find|vim|nano|code|python|node|npm|git|make|gcc|clang)\b',
            r'\b([a-zA-Z0-9_\-]+\.py|[a-zA-Z0-9_\-]+\.js|[a-zA-Z0-9_\-]+\.c)\b',
        ]
        self.url_patterns = [
            r'(https?://[a-zA-Z0-9\-._~:/?#[\]@!$&\'()*+,;=]+)',
            r'([a-zA-Z0-9\-]+\.[a-zA-Z]{2,6}(?:/[^\s]*)?)',
        ]
    
    def extract_references(self, text: str) -> Set[str]:
        """Extract file paths, commands, URLs from LLM response text"""
        references = set()
        
        if not text:
            return references
        
        # Extract file paths
        for pattern in self.file_patterns:
            matches = re.findall(pattern, text)
            references.update(matches)
        
        # Extract commands
        for pattern in self.command_patterns:
            matches = re.findall(pattern, text, re.IGNORECASE)
            references.update(matches)
        
        # Extract URLs
        for pattern in self.url_patterns:
            matches = re.findall(pattern, text)
            references.update(matches)
        
        # Clean up and normalize references
        cleaned_refs = set()
        for ref in references:
            ref = ref.strip('\'".,;:!?')
            if len(ref) > 2:  # Filter out very short matches
                cleaned_refs.add(ref)
        
        return cleaned_refs
    
    def calculate_argument_match_score(self, llm_refs: Set[str], system_args: List[str]) -> float:
        """Calculate match score between LLM response references and system call arguments"""
        if not llm_refs or not system_args:
            return 0.0
        
        # Join all system arguments into a single searchable string
        args_text = ' '.join(str(arg) for arg in system_args)
        
        matches = 0
        total_refs = len(llm_refs)
        
        for ref in llm_refs:
            # Check for exact matches
            if ref in args_text:
                matches += 1
            # Check for partial matches (for file paths)
            elif any(ref in str(arg) or str(arg) in ref for arg in system_args):
                matches += 0.5
        
        return matches / total_refs if total_refs > 0 else 0.0


class MultiSignalCorrelator:
    """Main correlation engine implementing the AgentSight approach"""
    
    def __init__(self, temporal_window_ms: int = 500):
        self.temporal_window_ms = temporal_window_ms
        self.temporal_window_ns = temporal_window_ms * 1_000_000  # Convert to nanoseconds
        
        self.process_tree = ProcessTreeBuilder()
        self.argument_matcher = ArgumentMatcher()
        
        # Event queues
        self.llm_interactions: List[LLMInteraction] = []
        self.system_events: List[Dict[str, Any]] = []
        self.correlations: List[CorrelatedEvent] = []
    
    def load_trace_data(self, trace_file: str):
        """Load trace data from AgentSight log file"""
        print(f"Loading trace data from: {trace_file}")
        
        with open(trace_file, 'r') as f:
            for line_num, line in enumerate(f, 1):
                try:
                    event = json.loads(line.strip())
                    self._process_event(event)
                except json.JSONDecodeError:
                    continue
        
        print(f"Loaded {len(self.llm_interactions)} LLM interactions")
        print(f"Loaded {len(self.system_events)} system events")
        print(f"Built process tree with {len(self.process_tree.processes)} processes")
    
    def _process_event(self, event: Dict[str, Any]):
        """Process individual event and categorize"""
        source = event.get('source', '')
        
        if source == 'http_parser':
            self._process_http_event(event)
        elif source == 'ssl':
            self._process_ssl_event(event)
        elif source == 'process':
            self._process_system_event(event)
        elif source == 'sse_processor':
            self._process_sse_event(event)
    
    def _process_http_event(self, event: Dict[str, Any]):
        """Process HTTP parser events (LLM interactions)"""
        data = event.get('data', {})
        message_type = data.get('message_type', '')
        
        if message_type == 'request':
            # Store request, will match with response later
            self._pending_requests = getattr(self, '_pending_requests', {})
            tid = data.get('tid', 0)
            self._pending_requests[tid] = {
                'timestamp': event.get('timestamp'),
                'data': data,
                'event': event
            }
        
        elif message_type == 'response':
            # Match with pending request
            self._pending_requests = getattr(self, '_pending_requests', {})
            tid = data.get('tid', 0)
            
            if tid in self._pending_requests:
                request_info = self._pending_requests[tid]
                
                # Extract response text for argument matching
                response_text = self._extract_response_text(data)
                references = self.argument_matcher.extract_references(response_text)
                
                interaction = LLMInteraction(
                    request_timestamp=request_info['timestamp'],
                    response_timestamp=event.get('timestamp'),
                    request_data=request_info['data'],
                    response_data=data,
                    tid=tid,
                    pid=data.get('pid', 0),
                    comm=data.get('comm', ''),
                    extracted_references=references
                )
                
                self.llm_interactions.append(interaction)
                del self._pending_requests[tid]
    
    def _process_ssl_event(self, event: Dict[str, Any]):
        """Process SSL events"""
        # SSL events are lower-level, mainly for debugging
        # The HTTP parser events are more useful for correlation
        pass
    
    def _process_system_event(self, event: Dict[str, Any]):
        """Process system events (process creation, file operations, etc.)"""
        data = event.get('data', {})
        
        # Add to process tree if it's a process event
        if 'pid' in data and 'comm' in data:
            self.process_tree.add_process_event(event)
        
        # Add to system events for correlation
        self.system_events.append(event)
    
    def _process_sse_event(self, event: Dict[str, Any]):
        """Process Server-Sent Events (streaming LLM responses)"""
        # SSE events contain streaming response data
        # These are handled by the HTTP parser in most cases
        pass
    
    def _extract_response_text(self, response_data: Dict[str, Any]) -> str:
        """Extract text content from LLM response"""
        body = response_data.get('body', '')
        
        if not body:
            return ''
        
        # Try to parse as JSON first
        try:
            if isinstance(body, str):
                json_body = json.loads(body)
            else:
                json_body = body
            
            # Extract text from Claude-style response
            if 'content' in json_body:
                content = json_body['content']
                if isinstance(content, list):
                    texts = []
                    for item in content:
                        if isinstance(item, dict) and item.get('type') == 'text':
                            texts.append(item.get('text', ''))
                    return ' '.join(texts)
                elif isinstance(content, str):
                    return content
            
            # Extract from other response formats
            if 'choices' in json_body:  # OpenAI format
                choices = json_body['choices']
                if choices and 'message' in choices[0]:
                    return choices[0]['message'].get('content', '')
            
            # Fallback to string representation
            return str(json_body)
            
        except json.JSONDecodeError:
            return body
    
    def correlate_events(self) -> List[CorrelatedEvent]:
        """Perform multi-signal correlation between LLM interactions and system events"""
        print(f"Starting correlation with {len(self.llm_interactions)} LLM interactions")
        
        correlations = []
        
        for interaction in self.llm_interactions:
            # Find potentially related system events using all three mechanisms
            related_events = self._find_related_system_events(interaction)
            
            if related_events:
                correlation = CorrelatedEvent(
                    llm_interaction=interaction,
                    system_actions=related_events,
                    correlation_score=self._calculate_correlation_score(interaction, related_events),
                    correlation_type=self._determine_correlation_type(interaction, related_events),
                    evidence=self._gather_correlation_evidence(interaction, related_events)
                )
                correlations.append(correlation)
        
        self.correlations = correlations
        print(f"Found {len(correlations)} correlations")
        return correlations
    
    def _find_related_system_events(self, interaction: LLMInteraction) -> List[Dict[str, Any]]:
        """Find system events related to LLM interaction using three mechanisms"""
        related_events = []
        
        # 1. Process Lineage: Find events from related processes
        related_pids = self.process_tree.find_related_processes(interaction.pid)
        
        # 2. Temporal Proximity: Find events within time window after response
        time_window_start = interaction.response_timestamp
        time_window_end = time_window_start + self.temporal_window_ns
        
        for event in self.system_events:
            event_timestamp = event.get('timestamp', 0)
            event_pid = event.get('data', {}).get('pid', 0)
            
            # Check temporal proximity
            in_time_window = time_window_start <= event_timestamp <= time_window_end
            
            # Check process lineage
            in_process_tree = event_pid in related_pids
            
            # 3. Argument Matching: Check if event arguments match LLM response content
            has_argument_match = self._check_argument_match(interaction, event)
            
            # Include event if it satisfies any of the three criteria
            if in_time_window or in_process_tree or has_argument_match:
                related_events.append(event)
        
        # Sort by timestamp
        related_events.sort(key=lambda x: x.get('timestamp', 0))
        
        return related_events
    
    def _check_argument_match(self, interaction: LLMInteraction, event: Dict[str, Any]) -> bool:
        """Check if system event arguments match LLM response content"""
        if not interaction.extracted_references:
            return False
        
        # Extract arguments from system event
        data = event.get('data', {})
        event_args = []
        
        # Common argument fields in system events
        for field in ['args', 'filename', 'path', 'command', 'syscall']:
            if field in data:
                value = data[field]
                if isinstance(value, list):
                    event_args.extend(str(v) for v in value)
                else:
                    event_args.append(str(value))
        
        # Calculate match score
        match_score = self.argument_matcher.calculate_argument_match_score(
            interaction.extracted_references, event_args
        )
        
        return match_score > 0.3  # Threshold for considering it a match
    
    def _calculate_correlation_score(self, interaction: LLMInteraction, events: List[Dict[str, Any]]) -> float:
        """Calculate overall correlation score"""
        if not events:
            return 0.0
        
        scores = []
        
        for event in events:
            event_timestamp = event.get('timestamp', 0)
            event_pid = event.get('data', {}).get('pid', 0)
            
            # Temporal proximity score (closer = higher score)
            time_diff = abs(event_timestamp - interaction.response_timestamp)
            max_time_diff = self.temporal_window_ns
            temporal_score = max(0, 1 - (time_diff / max_time_diff))
            
            # Process lineage score
            related_pids = self.process_tree.find_related_processes(interaction.pid)
            process_score = 1.0 if event_pid in related_pids else 0.0
            
            # Argument matching score
            event_args = self._extract_event_arguments(event)
            argument_score = self.argument_matcher.calculate_argument_match_score(
                interaction.extracted_references, event_args
            )
            
            # Combined score (weighted average)
            combined_score = (temporal_score * 0.3 + process_score * 0.3 + argument_score * 0.4)
            scores.append(combined_score)
        
        return sum(scores) / len(scores)
    
    def _extract_event_arguments(self, event: Dict[str, Any]) -> List[str]:
        """Extract arguments from system event"""
        data = event.get('data', {})
        args = []
        
        for field in ['args', 'filename', 'path', 'command', 'syscall', 'comm']:
            if field in data:
                value = data[field]
                if isinstance(value, list):
                    args.extend(str(v) for v in value)
                else:
                    args.append(str(value))
        
        return args
    
    def _determine_correlation_type(self, interaction: LLMInteraction, events: List[Dict[str, Any]]) -> str:
        """Determine primary correlation mechanism"""
        process_matches = 0
        temporal_matches = 0
        argument_matches = 0
        
        related_pids = self.process_tree.find_related_processes(interaction.pid)
        time_window_start = interaction.response_timestamp
        time_window_end = time_window_start + self.temporal_window_ns
        
        for event in events:
            event_timestamp = event.get('timestamp', 0)
            event_pid = event.get('data', {}).get('pid', 0)
            
            if event_pid in related_pids:
                process_matches += 1
            
            if time_window_start <= event_timestamp <= time_window_end:
                temporal_matches += 1
            
            if self._check_argument_match(interaction, event):
                argument_matches += 1
        
        # Return the primary correlation type
        if argument_matches > 0:
            return "argument_matching"
        elif temporal_matches > process_matches:
            return "temporal_proximity"
        else:
            return "process_lineage"
    
    def _gather_correlation_evidence(self, interaction: LLMInteraction, events: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Gather evidence for the correlation"""
        evidence = {
            'llm_references': list(interaction.extracted_references),
            'time_window_ms': self.temporal_window_ms,
            'process_lineage': list(self.process_tree.find_related_processes(interaction.pid)),
            'event_details': []
        }
        
        for event in events:
            event_evidence = {
                'timestamp': event.get('timestamp'),
                'source': event.get('source'),
                'pid': event.get('data', {}).get('pid', 0),
                'arguments': self._extract_event_arguments(event),
                'time_offset_ms': (event.get('timestamp', 0) - interaction.response_timestamp) / 1_000_000
            }
            evidence['event_details'].append(event_evidence)
        
        return evidence
    
    def export_correlations(self, output_file: str):
        """Export correlations to JSON file"""
        export_data = {
            'analysis_metadata': {
                'timestamp': datetime.now().isoformat(),
                'correlation_engine_version': '1.0.0',
                'temporal_window_ms': self.temporal_window_ms,
                'total_llm_interactions': len(self.llm_interactions),
                'total_system_events': len(self.system_events),
                'total_correlations': len(self.correlations)
            },
            'correlations': []
        }
        
        for correlation in self.correlations:
            correlation_data = {
                'llm_interaction': {
                    'request_timestamp': correlation.llm_interaction.request_timestamp,
                    'response_timestamp': correlation.llm_interaction.response_timestamp,
                    'tid': correlation.llm_interaction.tid,
                    'pid': correlation.llm_interaction.pid,
                    'comm': correlation.llm_interaction.comm,
                    'extracted_references': list(correlation.llm_interaction.extracted_references),
                    'request_summary': self._summarize_request(correlation.llm_interaction.request_data),
                    'response_summary': self._summarize_response(correlation.llm_interaction.response_data)
                },
                'system_actions': [
                    {
                        'timestamp': event.get('timestamp'),
                        'source': event.get('source'),
                        'pid': event.get('data', {}).get('pid', 0),
                        'comm': event.get('data', {}).get('comm', ''),
                        'action_type': event.get('data', {}).get('syscall', event.get('source', 'unknown')),
                        'arguments': self._extract_event_arguments(event)
                    }
                    for event in correlation.system_actions
                ],
                'correlation_score': correlation.correlation_score,
                'correlation_type': correlation.correlation_type,
                'evidence': correlation.evidence
            }
            export_data['correlations'].append(correlation_data)
        
        with open(output_file, 'w') as f:
            json.dump(export_data, f, indent=2, ensure_ascii=False)
        
        print(f"Correlations exported to: {output_file}")
    
    def _summarize_request(self, request_data: Dict[str, Any]) -> str:
        """Create summary of LLM request"""
        method = request_data.get('method', 'UNKNOWN')
        path = request_data.get('path', '/')
        
        # Try to extract model or key info from body
        body = request_data.get('body', '')
        summary_parts = [f"{method} {path}"]
        
        try:
            if body:
                json_body = json.loads(body) if isinstance(body, str) else body
                if 'model' in json_body:
                    summary_parts.append(f"model: {json_body['model']}")
                if 'messages' in json_body and json_body['messages']:
                    first_message = json_body['messages'][0].get('content', '')
                    if first_message:
                        preview = first_message[:100] + "..." if len(first_message) > 100 else first_message
                        summary_parts.append(f"message: {preview}")
        except:
            pass
        
        return " | ".join(summary_parts)
    
    def _summarize_response(self, response_data: Dict[str, Any]) -> str:
        """Create summary of LLM response"""
        status_code = response_data.get('status_code', 0)
        body = response_data.get('body', '')
        
        summary_parts = [f"status: {status_code}"]
        
        if body:
            # Extract first 200 chars of response
            text_content = self._extract_response_text(response_data)
            if text_content:
                preview = text_content[:200] + "..." if len(text_content) > 200 else text_content
                summary_parts.append(f"response: {preview}")
        
        return " | ".join(summary_parts)
    
    def print_correlation_summary(self):
        """Print human-readable correlation summary"""
        print(f"\n{'='*80}")
        print("MULTI-SIGNAL CAUSAL CORRELATION RESULTS")
        print(f"{'='*80}")
        
        print(f"Total LLM Interactions: {len(self.llm_interactions)}")
        print(f"Total System Events: {len(self.system_events)}")
        print(f"Total Correlations Found: {len(self.correlations)}")
        print(f"Temporal Window: {self.temporal_window_ms}ms")
        
        # Correlation type breakdown
        type_counts = defaultdict(int)
        for corr in self.correlations:
            type_counts[corr.correlation_type] += 1
        
        print(f"\nCorrelation Types:")
        for corr_type, count in type_counts.items():
            print(f"  {corr_type}: {count}")
        
        # Top correlations by score
        sorted_correlations = sorted(self.correlations, key=lambda x: x.correlation_score, reverse=True)
        
        print(f"\nTop 5 Correlations by Score:")
        print(f"{'Score':<8} {'Type':<20} {'LLM Summary':<50} {'Actions':<10}")
        print("-" * 90)
        
        for i, corr in enumerate(sorted_correlations[:5]):
            score = f"{corr.correlation_score:.3f}"
            corr_type = corr.correlation_type[:18]
            llm_summary = self._summarize_request(corr.llm_interaction.request_data)[:48]
            action_count = len(corr.system_actions)
            
            print(f"{score:<8} {corr_type:<20} {llm_summary:<50} {action_count:<10}")


def main():
    parser = argparse.ArgumentParser(
        description="Multi-Signal Causal Correlation Engine for AgentSight",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Multi-Signal Correlation Mechanisms:
  1. Process Lineage - Tracks process tree relationships (fork/execve)
  2. Temporal Proximity - Associates actions within time window (100-500ms)
  3. Argument Matching - Matches LLM response content with syscall arguments

Examples:
  python multi_signal_correlator.py trace.log
  python multi_signal_correlator.py trace.log -o correlations.json
  python multi_signal_correlator.py trace.log --temporal-window 300
  python multi_signal_correlator.py trace.log --verbose

This implements the core correlation strategy from AgentSight paper Section 3.2.
        """
    )
    
    parser.add_argument('trace_file', help='AgentSight trace file to analyze')
    parser.add_argument('-o', '--output', help='Output file for correlations (JSON)')
    parser.add_argument('--temporal-window', type=int, default=500,
                       help='Temporal proximity window in milliseconds (default: 500)')
    parser.add_argument('--verbose', '-v', action='store_true',
                       help='Enable verbose output')
    
    args = parser.parse_args()
    
    try:
        correlator = MultiSignalCorrelator(temporal_window_ms=args.temporal_window)
        
        # Load trace data
        correlator.load_trace_data(args.trace_file)
        
        # Perform correlation
        correlations = correlator.correlate_events()
        
        # Print summary
        correlator.print_correlation_summary()
        
        # Export results if requested
        if args.output:
            correlator.export_correlations(args.output)
        else:
            # Generate default output filename
            base_name = os.path.splitext(args.trace_file)[0]
            output_file = f"{base_name}_correlations.json"
            correlator.export_correlations(output_file)
        
        print(f"\nCorrelation analysis complete!")
        
    except FileNotFoundError:
        print(f"Error: Trace file '{args.trace_file}' not found", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        if args.verbose:
            import traceback
            traceback.print_exc()
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()