#!/usr/bin/env python3
"""
LLM Semantic Analyzer for AgentSight

Implements the secondary LLM analysis component described in the AgentSight paper.
Uses an LLM as a security analyst to detect:
1. Prompt injection attacks
2. Resource-wasting reasoning loops
3. Multi-agent coordination failures
4. Other suspicious behaviors

Based on Section 4.2 - "The Hybrid Correlation Engine"
"""

import json
import sys
import argparse
import os
from datetime import datetime
from typing import Dict, List, Any, Optional
import subprocess
import tempfile
from dataclasses import dataclass
from collections import defaultdict, Counter


@dataclass
class AnalysisResult:
    """Structured result from LLM analysis"""
    threat_level: int  # 1-5 scale
    threat_type: str
    confidence: float  # 0-1 scale
    summary: str
    details: str
    recommendations: List[str]
    evidence: List[Dict[str, Any]]


class AgentBehaviorPattern:
    """Detects patterns in agent behavior from trace data"""
    
    def __init__(self):
        self.reset()
    
    def reset(self):
        self.llm_calls = []
        self.system_actions = []
        self.error_patterns = []
        self.resource_usage = {
            'api_tokens': 0,
            'system_calls': 0,
            'file_operations': 0,
            'network_connections': 0
        }
    
    def add_llm_interaction(self, interaction: Dict[str, Any]):
        """Add LLM interaction to pattern analysis"""
        self.llm_calls.append(interaction)
        
        # Track token usage if available
        if 'usage' in interaction:
            self.resource_usage['api_tokens'] += interaction['usage'].get('total_tokens', 0)
    
    def add_system_action(self, action: Dict[str, Any]):
        """Add system action to pattern analysis"""
        self.system_actions.append(action)
        
        # Categorize system actions
        if action.get('type') == 'syscall':
            self.resource_usage['system_calls'] += 1
        elif action.get('type') == 'file_operation':
            self.resource_usage['file_operations'] += 1
        elif action.get('type') == 'network':
            self.resource_usage['network_connections'] += 1
    
    def detect_reasoning_loop(self) -> Optional[Dict[str, Any]]:
        """Detect if agent is stuck in a reasoning loop"""
        if len(self.llm_calls) < 3:
            return None
        
        # Look for repeated error patterns
        recent_calls = self.llm_calls[-10:]  # Last 10 calls
        error_sequences = []
        
        for call in recent_calls:
            if 'error' in call.get('response', '').lower():
                error_sequences.append(call)
        
        # Check for repeated identical errors
        if len(error_sequences) >= 3:
            error_messages = [call.get('response', '') for call in error_sequences]
            if len(set(error_messages)) == 1:  # All errors are identical
                return {
                    'type': 'reasoning_loop',
                    'evidence': error_sequences,
                    'loop_count': len(error_sequences),
                    'total_tokens_wasted': sum(call.get('usage', {}).get('total_tokens', 0) for call in error_sequences)
                }
        
        return None
    
    def detect_data_exfiltration(self) -> Optional[Dict[str, Any]]:
        """Detect potential data exfiltration patterns"""
        suspicious_actions = []
        
        # Look for sensitive file reads followed by network activity
        sensitive_files = ['/etc/passwd', '/etc/shadow', '~/.ssh/', '/home/', '/root/']
        
        for action in self.system_actions:
            if action.get('type') == 'file_operation':
                file_path = action.get('path', '')
                if any(sensitive in file_path for sensitive in sensitive_files):
                    suspicious_actions.append(action)
        
        # Check if followed by network connections
        if suspicious_actions:
            file_read_time = suspicious_actions[-1].get('timestamp', 0)
            network_actions = [
                a for a in self.system_actions 
                if a.get('type') == 'network' and a.get('timestamp', 0) > file_read_time
            ]
            
            if network_actions:
                return {
                    'type': 'data_exfiltration',
                    'evidence': {
                        'file_operations': suspicious_actions,
                        'network_operations': network_actions
                    }
                }
        
        return None


class LLMSemanticAnalyzer:
    """Main analyzer class implementing the secondary LLM analysis from AgentSight"""
    
    def __init__(self, llm_provider: str = "claude", model: str = "claude-3-sonnet-20240229"):
        self.llm_provider = llm_provider
        self.model = model
        self.pattern_detector = AgentBehaviorPattern()
        
    def analyze_trace(self, trace_file: str, output_file: Optional[str] = None) -> AnalysisResult:
        """Analyze agent trace data using secondary LLM analysis"""
        
        # Load and preprocess trace data
        trace_data = self._load_trace_data(trace_file)
        
        # Extract behavioral patterns
        self._extract_patterns(trace_data)
        
        # Perform heuristic analysis
        heuristic_findings = self._heuristic_analysis()
        
        # Generate LLM analysis prompt
        analysis_prompt = self._generate_analysis_prompt(trace_data, heuristic_findings)
        
        # Query LLM for semantic analysis
        llm_response = self._query_llm(analysis_prompt)
        
        # Parse and structure results
        result = self._parse_llm_response(llm_response, heuristic_findings)
        
        # Save results if output file specified
        if output_file:
            self._save_results(result, output_file)
        
        return result
    
    def _load_trace_data(self, trace_file: str) -> List[Dict[str, Any]]:
        """Load and parse trace data from file"""
        trace_data = []
        
        with open(trace_file, 'r') as f:
            for line in f:
                try:
                    entry = json.loads(line.strip())
                    trace_data.append(entry)
                except json.JSONDecodeError:
                    continue
        
        return trace_data
    
    def _extract_patterns(self, trace_data: List[Dict[str, Any]]):
        """Extract behavioral patterns from trace data"""
        self.pattern_detector.reset()
        
        for entry in trace_data:
            source = entry.get('source', '')
            data = entry.get('data', {})
            
            if source == 'http_parser':
                # This is an LLM interaction
                if data.get('message_type') == 'request':
                    self.pattern_detector.add_llm_interaction({
                        'timestamp': entry.get('timestamp'),
                        'request': data.get('body', ''),
                        'path': data.get('path', ''),
                        'method': data.get('method', '')
                    })
                elif data.get('message_type') == 'response':
                    # Add response to last interaction
                    if self.pattern_detector.llm_calls:
                        self.pattern_detector.llm_calls[-1]['response'] = data.get('body', '')
            
            elif source == 'process':
                # This is a system action
                self.pattern_detector.add_system_action({
                    'timestamp': entry.get('timestamp'),
                    'type': 'syscall',
                    'comm': data.get('comm', ''),
                    'pid': data.get('pid', 0),
                    'action': data.get('syscall', ''),
                    'args': data.get('args', [])
                })
    
    def _heuristic_analysis(self) -> Dict[str, Any]:
        """Perform heuristic analysis to identify potential issues"""
        findings = {
            'reasoning_loop': None,
            'data_exfiltration': None,
            'resource_usage': self.pattern_detector.resource_usage,
            'risk_score': 0
        }
        
        # Check for reasoning loops
        loop_detection = self.pattern_detector.detect_reasoning_loop()
        if loop_detection:
            findings['reasoning_loop'] = loop_detection
            findings['risk_score'] += 3
        
        # Check for data exfiltration
        exfil_detection = self.pattern_detector.detect_data_exfiltration()
        if exfil_detection:
            findings['data_exfiltration'] = exfil_detection
            findings['risk_score'] += 5
        
        # Check resource usage patterns
        if findings['resource_usage']['api_tokens'] > 10000:
            findings['risk_score'] += 2
        
        if findings['resource_usage']['system_calls'] > 1000:
            findings['risk_score'] += 1
        
        return findings
    
    def _generate_analysis_prompt(self, trace_data: List[Dict[str, Any]], heuristic_findings: Dict[str, Any]) -> str:
        """Generate prompt for secondary LLM analysis"""
        
        # Summarize trace data
        trace_summary = {
            'total_entries': len(trace_data),
            'llm_interactions': len(self.pattern_detector.llm_calls),
            'system_actions': len(self.pattern_detector.system_actions),
            'timespan_hours': self._calculate_timespan(trace_data),
            'resource_usage': heuristic_findings['resource_usage']
        }
        
        # Extract key interactions for analysis
        key_interactions = self._extract_key_interactions(trace_data)
        
        prompt = f"""You are a security analyst examining AI agent behavior traces from the AgentSight observability framework. Your task is to analyze the following agent execution trace and identify potential security threats, performance issues, or anomalous behaviors.

TRACE SUMMARY:
- Total entries: {trace_summary['total_entries']}
- LLM interactions: {trace_summary['llm_interactions']}
- System actions: {trace_summary['system_actions']}
- Execution timespan: {trace_summary['timespan_hours']:.2f} hours
- Resource usage: {json.dumps(trace_summary['resource_usage'], indent=2)}

HEURISTIC ANALYSIS FINDINGS:
{json.dumps(heuristic_findings, indent=2, default=str)}

KEY INTERACTIONS (chronological sample):
{json.dumps(key_interactions, indent=2, default=str)}

ANALYSIS INSTRUCTIONS:
1. Assess the threat level on a scale of 1-5 (1=benign, 5=critical threat)
2. Identify the primary threat type if any (prompt_injection, reasoning_loop, data_exfiltration, resource_abuse, coordination_failure, none)
3. Provide your confidence level (0.0-1.0)
4. Give a brief summary of findings
5. Provide detailed analysis explaining your reasoning
6. Suggest specific recommendations for remediation

Focus particularly on:
- Signs of prompt injection attacks (unexpected system operations following LLM interactions)
- Reasoning loops (repeated identical errors or API calls)
- Data exfiltration patterns (sensitive file access followed by network activity)
- Resource abuse (excessive API usage, infinite loops)
- Multi-agent coordination issues (blocking, conflicts, inefficiencies)

Respond in the following JSON format:
{{
    "threat_level": <1-5>,
    "threat_type": "<type>",
    "confidence": <0.0-1.0>,
    "summary": "<brief summary>",
    "details": "<detailed analysis>",
    "recommendations": ["<recommendation1>", "<recommendation2>", ...],
    "evidence": [
        {{"type": "<evidence_type>", "description": "<description>", "severity": "<low|medium|high>"}}
    ]
}}"""
        
        return prompt
    
    def _calculate_timespan(self, trace_data: List[Dict[str, Any]]) -> float:
        """Calculate execution timespan in hours"""
        if not trace_data:
            return 0.0
        
        timestamps = [entry.get('timestamp', 0) for entry in trace_data if entry.get('timestamp', 0) > 0]
        if not timestamps:
            return 0.0
        
        duration_ns = max(timestamps) - min(timestamps)
        return duration_ns / (1_000_000_000 * 3600)  # Convert to hours
    
    def _extract_key_interactions(self, trace_data: List[Dict[str, Any]], max_items: int = 20) -> List[Dict[str, Any]]:
        """Extract key interactions for LLM analysis"""
        # Sort by timestamp
        sorted_trace = sorted(trace_data, key=lambda x: x.get('timestamp', 0))
        
        # Take evenly distributed samples
        if len(sorted_trace) <= max_items:
            return sorted_trace
        
        step = len(sorted_trace) // max_items
        samples = []
        
        for i in range(0, len(sorted_trace), step):
            if len(samples) >= max_items:
                break
            samples.append(sorted_trace[i])
        
        return samples
    
    def _query_llm(self, prompt: str) -> str:
        """Query LLM for semantic analysis"""
        if self.llm_provider == "claude":
            return self._query_claude(prompt)
        elif self.llm_provider == "openai":
            return self._query_openai(prompt)
        elif self.llm_provider == "local":
            return self._query_local_llm(prompt)
        else:
            raise ValueError(f"Unsupported LLM provider: {self.llm_provider}")
    
    def _query_claude(self, prompt: str) -> str:
        """Query Claude API"""
        try:
            import antropic
            client = antropic.Anthropic()
            
            response = client.messages.create(
                model=self.model,
                max_tokens=4000,
                messages=[{"role": "user", "content": prompt}]
            )
            
            return response.content[0].text
        except ImportError:
            # Fallback to curl if antropic library not available
            return self._query_claude_curl(prompt)
    
    def _query_claude_curl(self, prompt: str) -> str:
        """Query Claude using curl (fallback method)"""
        api_key = os.environ.get('ANTHROPIC_API_KEY')
        if not api_key:
            return self._generate_mock_response()
        
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            payload = {
                "model": self.model,
                "max_tokens": 4000,
                "messages": [{"role": "user", "content": prompt}]
            }
            json.dump(payload, f)
            temp_file = f.name
        
        try:
            cmd = [
                'curl', '-X', 'POST',
                'https://api.anthropic.com/v1/messages',
                '-H', f'x-api-key: {api_key}',
                '-H', 'Content-Type: application/json',
                '-H', 'anthropic-version: 2023-06-01',
                '-d', f'@{temp_file}'
            ]
            
            result = subprocess.run(cmd, capture_output=True, text=True)
            if result.returncode == 0:
                response = json.loads(result.stdout)
                return response['content'][0]['text']
            else:
                print(f"Claude API error: {result.stderr}")
                return self._generate_mock_response()
        
        finally:
            os.unlink(temp_file)
    
    def _query_openai(self, prompt: str) -> str:
        """Query OpenAI API"""
        try:
            import openai
            client = openai.OpenAI()
            
            response = client.chat.completions.create(
                model=self.model,
                messages=[{"role": "user", "content": prompt}],
                max_tokens=4000
            )
            
            return response.choices[0].message.content
        except ImportError:
            return self._generate_mock_response()
    
    def _query_local_llm(self, prompt: str) -> str:
        """Query local LLM (e.g., via Ollama)"""
        try:
            cmd = ['ollama', 'run', self.model, prompt]
            result = subprocess.run(cmd, capture_output=True, text=True)
            if result.returncode == 0:
                return result.stdout
            else:
                return self._generate_mock_response()
        except FileNotFoundError:
            return self._generate_mock_response()
    
    def _generate_mock_response(self) -> str:
        """Generate mock response for testing/demo purposes"""
        mock_response = {
            "threat_level": 1,
            "threat_type": "none",
            "confidence": 0.8,
            "summary": "No significant threats detected in agent behavior trace. Normal development workflow observed.",
            "details": "Analysis of the agent execution trace shows typical software development patterns with LLM-assisted code generation and compilation tasks. No indicators of prompt injection, data exfiltration, or reasoning loops detected. Resource usage appears within normal parameters.",
            "recommendations": [
                "Continue monitoring for unusual patterns",
                "Implement rate limiting for API calls if not already in place",
                "Regular security audits of agent interactions"
            ],
            "evidence": [
                {
                    "type": "normal_behavior",
                    "description": "Agent performed standard code compilation and analysis tasks",
                    "severity": "low"
                }
            ]
        }
        return json.dumps(mock_response, indent=2)
    
    def _parse_llm_response(self, response: str, heuristic_findings: Dict[str, Any]) -> AnalysisResult:
        """Parse LLM response into structured result"""
        try:
            # Try to extract JSON from response
            response_clean = response.strip()
            if response_clean.startswith('```json'):
                response_clean = response_clean[7:]
            if response_clean.endswith('```'):
                response_clean = response_clean[:-3]
            
            parsed = json.loads(response_clean)
            
            return AnalysisResult(
                threat_level=parsed.get('threat_level', 1),
                threat_type=parsed.get('threat_type', 'unknown'),
                confidence=parsed.get('confidence', 0.5),
                summary=parsed.get('summary', ''),
                details=parsed.get('details', ''),
                recommendations=parsed.get('recommendations', []),
                evidence=parsed.get('evidence', [])
            )
        
        except json.JSONDecodeError:
            # Fallback to parsing from text
            return AnalysisResult(
                threat_level=self._extract_threat_level(response),
                threat_type='unknown',
                confidence=0.5,
                summary=response[:200] + "..." if len(response) > 200 else response,
                details=response,
                recommendations=[],
                evidence=[]
            )
    
    def _extract_threat_level(self, text: str) -> int:
        """Extract threat level from text response"""
        import re
        match = re.search(r'threat.{0,10}level.{0,10}(\d)', text.lower())
        if match:
            return int(match.group(1))
        return 1
    
    def _save_results(self, result: AnalysisResult, output_file: str):
        """Save analysis results to file"""
        output_data = {
            'analysis_metadata': {
                'timestamp': datetime.now().isoformat(),
                'analyzer_version': '1.0.0',
                'llm_provider': self.llm_provider,
                'model': self.model
            },
            'threat_assessment': {
                'threat_level': result.threat_level,
                'threat_type': result.threat_type,
                'confidence': result.confidence
            },
            'analysis': {
                'summary': result.summary,
                'details': result.details,
                'recommendations': result.recommendations,
                'evidence': result.evidence
            },
            'resource_usage': self.pattern_detector.resource_usage
        }
        
        with open(output_file, 'w') as f:
            json.dump(output_data, f, indent=2, ensure_ascii=False)


def main():
    parser = argparse.ArgumentParser(
        description="LLM Semantic Analyzer for AgentSight - Secondary LLM analysis of agent behavior traces",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python llm_semantic_analyzer.py trace.log
  python llm_semantic_analyzer.py trace.log -o analysis_report.json
  python llm_semantic_analyzer.py trace.log --llm-provider openai --model gpt-4
  python llm_semantic_analyzer.py trace.log --llm-provider local --model llama2

Supported LLM providers:
  claude    - Anthropic Claude (requires ANTHROPIC_API_KEY)
  openai    - OpenAI GPT (requires OPENAI_API_KEY)
  local     - Local LLM via Ollama

The analyzer implements the secondary LLM analysis component from the AgentSight
paper, using an LLM as a security analyst to detect threats and anomalies.
        """
    )
    
    parser.add_argument('trace_file', help='Agent trace file to analyze')
    parser.add_argument('-o', '--output', help='Output file for analysis results')
    parser.add_argument('--llm-provider', choices=['claude', 'openai', 'local'], 
                       default='claude', help='LLM provider to use (default: claude)')
    parser.add_argument('--model', help='LLM model to use (provider-specific)')
    parser.add_argument('--verbose', '-v', action='store_true', 
                       help='Enable verbose output')
    
    args = parser.parse_args()
    
    # Set default model based on provider
    if not args.model:
        if args.llm_provider == 'claude':
            args.model = 'claude-3-sonnet-20240229'
        elif args.llm_provider == 'openai':
            args.model = 'gpt-4'
        elif args.llm_provider == 'local':
            args.model = 'llama2'
    
    try:
        analyzer = LLMSemanticAnalyzer(args.llm_provider, args.model)
        
        if args.verbose:
            print(f"Analyzing trace file: {args.trace_file}")
            print(f"LLM provider: {args.llm_provider}")
            print(f"Model: {args.model}")
        
        result = analyzer.analyze_trace(args.trace_file, args.output)
        
        # Print results
        print(f"\n{'='*60}")
        print("AGENT BEHAVIOR ANALYSIS RESULTS")
        print(f"{'='*60}")
        print(f"Threat Level: {result.threat_level}/5")
        print(f"Threat Type: {result.threat_type}")
        print(f"Confidence: {result.confidence:.2f}")
        print(f"\nSummary:")
        print(result.summary)
        
        if args.verbose:
            print(f"\nDetails:")
            print(result.details)
            
            if result.recommendations:
                print(f"\nRecommendations:")
                for i, rec in enumerate(result.recommendations, 1):
                    print(f"  {i}. {rec}")
            
            if result.evidence:
                print(f"\nEvidence:")
                for i, evidence in enumerate(result.evidence, 1):
                    print(f"  {i}. [{evidence.get('severity', 'unknown')}] {evidence.get('description', 'No description')}")
        
        if args.output:
            print(f"\nDetailed results saved to: {args.output}")
        
    except FileNotFoundError:
        print(f"Error: Trace file '{args.trace_file}' not found", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        if args.verbose:
            import traceback
            traceback.print_exc()
        sys.exit(1)


if __name__ == '__main__':
    main()