# Case Study 2: Reasoning Loop Detection Experiment

## Overview
This experiment demonstrates AgentSight's capability to detect and interrupt costly reasoning loops where an AI agent repeatedly fails to correct errors, consuming resources without making progress.

## Experiment Design

### Loop Scenario
An agent attempts to run a data processing tool but encounters an error due to incorrect arguments. Instead of correcting the mistake, it enters a "try-fail-re-reason" loop, repeatedly attempting the same failing command while consuming API tokens and compute resources.

### Prerequisites
- AgentSight collector with eBPF programs
- An AI agent with tool execution capabilities
- Mock failing tool/command for controlled testing
- Python 3.8+ for simulation scripts

## Implementation Steps

### Step 1: Create Failing Tool Simulation

```bash
# Create a tool that always fails with specific error patterns
cat > failing_tool.py << 'EOF'
#!/usr/bin/env python3
import sys
import time

def main():
    # Simulate processing time
    time.sleep(0.5)
    
    # Always fail with the same error
    if "--format" not in sys.argv:
        print("Error: Missing required argument --format")
        print("Usage: failing_tool.py --input <file> --format <json|csv>")
        sys.exit(1)
    
    # Even with --format, fail differently
    print("Error: Invalid format specified. Supported formats: json, csv")
    sys.exit(1)

if __name__ == "__main__":
    main()
EOF

chmod +x failing_tool.py
```

### Step 2: Start AgentSight Monitoring with Loop Detection

```bash
# Terminal 1: Start AgentSight with enhanced monitoring
cd /root/yunwei37/agentsight/collector
cargo run trace --ssl --process \
    --comm python --comm bash --comm failing_tool.py \
    --server --log-file reasoning_loop_test.log \
    --process-filter "exec.contains('failing_tool')" \
    --ssl-filter "data.type=api_call"
```

### Step 3: Create Agent Simulation with Loop Behavior

```bash
# Create agent that simulates reasoning loops
cat > agent_loop_simulator.py << 'EOF'
import subprocess
import time
import json
import requests
from datetime import datetime

class AgentSimulator:
    def __init__(self):
        self.api_calls = 0
        self.token_usage = 0
        self.attempts = []
        
    def call_llm(self, prompt):
        """Simulate LLM API call"""
        self.api_calls += 1
        self.token_usage += len(prompt.split()) * 2  # Rough token estimate
        
        # Log the API call
        api_log = {
            "timestamp": datetime.now().isoformat(),
            "type": "api_call",
            "prompt_preview": prompt[:100] + "...",
            "tokens": len(prompt.split()) * 2
        }
        print(f"LLM API Call #{self.api_calls}: {json.dumps(api_log)}")
        
        # Simulate LLM response that keeps making the same mistake
        if "error" in prompt.lower():
            return "I need to run the tool with the correct format. Let me try: failing_tool.py --input data.json"
        return "I'll process the data using the failing_tool"
    
    def execute_tool(self, command):
        """Execute tool and capture output"""
        attempt = {
            "timestamp": datetime.now().isoformat(),
            "command": command,
            "attempt_number": len(self.attempts) + 1
        }
        
        try:
            result = subprocess.run(
                command, 
                shell=True, 
                capture_output=True, 
                text=True,
                timeout=5
            )
            attempt["exit_code"] = result.returncode
            attempt["stdout"] = result.stdout
            attempt["stderr"] = result.stderr
        except Exception as e:
            attempt["error"] = str(e)
        
        self.attempts.append(attempt)
        return attempt
    
    def simulate_reasoning_loop(self, max_attempts=5):
        """Simulate an agent stuck in a reasoning loop"""
        print("=== Agent Starting Task: Process data.json ===\n")
        
        for i in range(max_attempts):
            print(f"\n--- Attempt {i+1} ---")
            
            # Step 1: Agent reasons about the task
            if i == 0:
                reasoning = self.call_llm("Process the file data.json using failing_tool")
            else:
                # Agent sees the error but fails to understand it
                last_error = self.attempts[-1].get("stdout", "")
                reasoning = self.call_llm(f"The tool failed with error: {last_error}. How should I fix this?")
            
            print(f"Agent reasoning: {reasoning}")
            
            # Step 2: Agent executes the command (incorrectly)
            # Note: Agent keeps forgetting to add --format argument
            command = "./failing_tool.py --input data.json"
            print(f"Executing: {command}")
            
            result = self.execute_tool(command)
            
            # Step 3: Agent sees the error
            if result["exit_code"] != 0:
                print(f"Error: {result['stdout']}")
                time.sleep(1)  # Simulate thinking time
            
            # Check if we're in a loop
            if i >= 2:
                # Check if last 3 attempts are identical
                recent_commands = [a["command"] for a in self.attempts[-3:]]
                if len(set(recent_commands)) == 1:
                    print("\n⚠️  LOOP DETECTED: Same command attempted 3 times!")
                    self.generate_loop_report()
                    return
        
        print("\n❌ Max attempts reached without success")
        self.generate_loop_report()
    
    def generate_loop_report(self):
        """Generate a report of the loop behavior"""
        print("\n=== REASONING LOOP ANALYSIS ===")
        print(f"Total API calls: {self.api_calls}")
        print(f"Total tokens used: {self.token_usage}")
        print(f"Estimated cost: ${self.token_usage * 0.00002:.2f}")  # GPT-4 pricing estimate
        print(f"Total attempts: {len(self.attempts)}")
        print(f"Time wasted: {len(self.attempts) * 2} seconds")
        
        # Pattern analysis
        commands = [a["command"] for a in self.attempts]
        unique_commands = set(commands)
        print(f"\nCommand patterns:")
        for cmd in unique_commands:
            count = commands.count(cmd)
            print(f"  '{cmd}' - attempted {count} times")

if __name__ == "__main__":
    simulator = AgentSimulator()
    simulator.simulate_reasoning_loop(max_attempts=10)
EOF
```

### Step 4: Create Loop Detection Analyzer

```bash
# Create analyzer for detecting loops in AgentSight logs
cat > loop_detector.py << 'EOF'
import json
import sys
from collections import defaultdict, deque
from datetime import datetime, timedelta

class LoopDetector:
    def __init__(self, window_size=3, time_window_seconds=60):
        self.window_size = window_size
        self.time_window = timedelta(seconds=time_window_seconds)
        self.command_history = deque(maxlen=window_size)
        self.api_calls = []
        self.loop_patterns = []
        
    def analyze_log(self, log_file):
        """Analyze AgentSight log for reasoning loops"""
        events_by_time = []
        
        with open(log_file, 'r') as f:
            for line in f:
                try:
                    event = json.loads(line)
                    events_by_time.append(event)
                except:
                    continue
        
        # Sort by timestamp
        events_by_time.sort(key=lambda x: x.get('timestamp', 0))
        
        # Analyze patterns
        process_events = defaultdict(list)
        ssl_events = []
        
        for event in events_by_time:
            if event['event_type'] == 'process':
                data = event.get('data', {})
                if 'exec' in data and 'failing_tool' in data.get('exec', ''):
                    process_events[data['exec']].append(event)
                    
            elif event['event_type'] == 'ssl':
                if 'api_call' in str(event.get('data', {})):
                    ssl_events.append(event)
        
        # Detect loops in process executions
        for cmd, events in process_events.items():
            if len(events) >= self.window_size:
                # Check for repeated executions within time window
                for i in range(len(events) - self.window_size + 1):
                    window = events[i:i + self.window_size]
                    first_time = window[0]['timestamp']
                    last_time = window[-1]['timestamp']
                    
                    # If same command executed multiple times quickly
                    if (last_time - first_time) < self.time_window.total_seconds() * 1000:
                        self.loop_patterns.append({
                            'type': 'process_loop',
                            'command': cmd,
                            'count': len(window),
                            'duration_ms': last_time - first_time,
                            'events': window
                        })
        
        # Analyze API call patterns
        self._analyze_api_patterns(ssl_events)
        
        # Generate report
        self._generate_report()
    
    def _analyze_api_patterns(self, ssl_events):
        """Analyze API calling patterns for waste"""
        if len(ssl_events) < 2:
            return
            
        total_tokens = 0
        call_intervals = []
        
        for i in range(1, len(ssl_events)):
            interval = ssl_events[i]['timestamp'] - ssl_events[i-1]['timestamp']
            call_intervals.append(interval)
            
            # Estimate tokens from event data
            data_str = str(ssl_events[i].get('data', {}))
            total_tokens += len(data_str.split()) // 10  # Rough estimate
        
        self.api_calls = {
            'count': len(ssl_events),
            'total_tokens': total_tokens,
            'avg_interval_ms': sum(call_intervals) / len(call_intervals) if call_intervals else 0,
            'estimated_cost': total_tokens * 0.00002  # GPT-4 estimate
        }
    
    def _generate_report(self):
        """Generate comprehensive loop detection report"""
        print("=== AGENTSIGHT LOOP DETECTION REPORT ===\n")
        
        if self.loop_patterns:
            print(f"⚠️  LOOPS DETECTED: {len(self.loop_patterns)}\n")
            
            for i, pattern in enumerate(self.loop_patterns, 1):
                print(f"Loop #{i}:")
                print(f"  Type: {pattern['type']}")
                print(f"  Command: {pattern['command']}")
                print(f"  Repetitions: {pattern['count']}")
                print(f"  Duration: {pattern['duration_ms']}ms")
                print(f"  Detection confidence: HIGH")
                print()
        else:
            print("✓ No obvious loops detected\n")
        
        if hasattr(self, 'api_calls') and self.api_calls['count'] > 0:
            print("API Usage Analysis:")
            print(f"  Total calls: {self.api_calls['count']}")
            print(f"  Estimated tokens: {self.api_calls['total_tokens']}")
            print(f"  Estimated cost: ${self.api_calls['estimated_cost']:.4f}")
            print(f"  Avg interval: {self.api_calls['avg_interval_ms']:.0f}ms")
            
            # Detect rapid API calling
            if self.api_calls['avg_interval_ms'] < 5000:  # Less than 5 seconds
                print("  ⚠️  WARNING: Rapid API calling detected!")
        
        print("\nRecommendations:")
        if self.loop_patterns:
            print("  1. Implement retry limits with exponential backoff")
            print("  2. Add error pattern recognition to break loops")
            print("  3. Set resource consumption thresholds")
            print("  4. Enable human intervention after N failures")

if __name__ == "__main__":
    detector = LoopDetector()
    log_file = sys.argv[1] if len(sys.argv) > 1 else "reasoning_loop_test.log"
    detector.analyze_log(log_file)
EOF
```

### Step 5: Run the Experiment

```bash
# Terminal 2: Execute the loop simulation
python agent_loop_simulator.py

# Terminal 3: Real-time monitoring
tail -f reasoning_loop_test.log | grep -E "(failing_tool|api_call)"
```

### Step 6: Analyze Results

```bash
# Run loop detection analysis
python loop_detector.py reasoning_loop_test.log

# Generate cost analysis
cat > cost_analyzer.py << 'EOF'
import json
import sys

def analyze_costs(log_file):
    total_cost = 0
    api_calls = 0
    wasted_calls = 0
    
    with open(log_file, 'r') as f:
        for line in f:
            try:
                event = json.loads(line)
                if 'api_call' in str(event):
                    api_calls += 1
                    # Rough token estimate
                    tokens = len(str(event).split()) * 2
                    cost = tokens * 0.00002
                    total_cost += cost
                    
                    # Check if this was a wasted call (same as previous)
                    # Implementation depends on your specific logging
                    
            except:
                continue
    
    print(f"Total API calls: {api_calls}")
    print(f"Estimated total cost: ${total_cost:.4f}")
    print(f"Average cost per call: ${total_cost/api_calls:.4f}" if api_calls > 0 else "N/A")

if __name__ == "__main__":
    analyze_costs(sys.argv[1] if len(sys.argv) > 1 else "reasoning_loop_test.log")
EOF

python cost_analyzer.py reasoning_loop_test.log
```

## Loop Pattern Variations

### 1. Tool Argument Loop
```python
# Agent keeps trying different incorrect arguments
commands = [
    "./tool --input data.json",
    "./tool -i data.json", 
    "./tool data.json --input",
    "./tool --input data.json"  # Back to first attempt
]
```

### 2. Cascading Failure Loop
```python
# One failure leads to another in a cycle
# Step 1: Try to read non-existent file → Error
# Step 2: Try to create file → Permission denied
# Step 3: Try to change permissions → Not owner
# Step 4: Back to Step 1
```

### 3. Reasoning Exhaustion Loop
```python
# Agent's context window fills with error messages
# Eventually forgets original task and early attempts
# Starts repeating very early failed approaches
```

## Real-time Intervention

### Automatic Loop Breaking
```bash
# Monitor script that kills agent after detecting loop
cat > loop_breaker.py << 'EOF'
import subprocess
import time
import json

def monitor_and_break():
    loop_count = 0
    last_commands = []
    
    # Monitor AgentSight output
    process = subprocess.Popen(
        ['tail', '-f', 'reasoning_loop_test.log'],
        stdout=subprocess.PIPE,
        text=True
    )
    
    for line in process.stdout:
        try:
            event = json.loads(line)
            if event['event_type'] == 'process':
                cmd = event.get('data', {}).get('exec', '')
                last_commands.append(cmd)
                
                # Keep only last 3 commands
                if len(last_commands) > 3:
                    last_commands.pop(0)
                
                # Check for loop
                if len(last_commands) == 3 and len(set(last_commands)) == 1:
                    loop_count += 1
                    print(f"⚠️  Loop detected! Count: {loop_count}")
                    
                    if loop_count >= 2:
                        print("🛑 BREAKING LOOP - Sending interrupt signal")
                        # In real scenario, would signal the agent
                        subprocess.run(['pkill', '-f', 'agent_loop_simulator'])
                        break
                        
        except:
            continue

monitor_and_break()
EOF
```

## Success Metrics

1. **Detection Latency**: Loop identified within 3 iterations
2. **Cost Savings**: Prevent >$2 in wasted API calls per incident  
3. **Resource Protection**: CPU/memory usage capped
4. **Intervention Speed**: Automatic breaking within 30 seconds

## Integration Points

- **Alerting**: Send notifications when loops detected
- **Rate Limiting**: Implement token bucket for API calls
- **Circuit Breaker**: Disable tool after N failures
- **Learning**: Build error pattern database

## Dashboard Visualization

Create real-time metrics showing:
- Loop detection events timeline
- API token burn rate graph  
- Cost accumulation meter
- Command repetition heatmap
- Intervention trigger status