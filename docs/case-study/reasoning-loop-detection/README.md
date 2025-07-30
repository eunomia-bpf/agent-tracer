# Case Study 2: Reasoning Loop Detection - Real CrewAI Implementation

## Overview
This experiment demonstrates AgentSight's capability to detect and interrupt costly reasoning loops using a real CrewAI multi-agent system. We implement a research agent using CrewAI with GPT-4o-mini that repeatedly calls a web search tool (SerperDevTool) with incorrect arguments, receives an error, but fails to correct its mistake, entering an infinite "try-fail-re-reason" loop.

## Real-World Loop Scenario
The implemented research crew contains:
- **Researcher Agent**: Uses SerperDevTool for web searches to uncover cutting-edge developments
- **Reporting Analyst**: Processes research findings into detailed lists

**The Loop Problem**: The researcher agent attempts multiple web searches but encounters persistent tool usage errors. Instead of learning from the error patterns, it:
1. Executes the same failing search command
2. Receives identical error messages from SerperDevTool
3. Passes the error back to the reasoning LLM (GPT-4o-mini)
4. Fails to learn from the tool's output
5. Repeats the exact same failing command

This creates a resource-consuming loop where API calls accumulate without progress, exactly matching the behavior described in the research paper.

### Prerequisites
- AgentSight collector with eBPF programs
- CrewAI framework with GPT-4o-mini integration
- SerperDev API key for web search functionality
- Python >=3.10 <=3.13 with Poetry for dependency management
- OpenAI API key for LLM interactions

### CrewAI Project Structure
The `latest_ai_development/` directory contains the real CrewAI implementation:
```
latest_ai_development/
├── src/latest_ai_development/
│   ├── config/
│   │   ├── agents.yaml    # Researcher and Reporting Analyst definitions
│   │   └── tasks.yaml     # Research and reporting task configurations
│   ├── tools/
│   │   └── custom_tool.py # Markdown link checker (with potential errors)
│   ├── crew.py           # CrewAI setup with SerperDevTool integration
│   └── main.py           # Entry point for agent execution
├── pyproject.toml        # Poetry dependencies and configuration
└── README.md            # CrewAI project documentation
```

## Implementation Steps

### Step 1: Setup CrewAI Project with Loop-Inducing Configuration

```bash
# Navigate to the CrewAI project directory
cd latest_ai_development

# Install dependencies
pip install poetry
poetry install

# Alternatively, use CrewAI CLI
crewai install

# Setup environment variables in .env file
cat > .env << 'EOF'
OPENAI_API_KEY=your_openai_api_key_here
SERPER_API_KEY=your_serper_dev_api_key_here
EOF
```

**Key Loop-Inducing Elements in the Configuration:**

1. **Researcher Agent** (`config/agents.yaml`):
   - Uses SerperDevTool for web searches
   - Configured to search "one at a time, do not search multiple queries at once"
   - Has goal to find 20 references with exact links

2. **Research Task** (`config/tasks.yaml`):
   - Demands 20 references with exact links
   - Requires multiple search queries but constrains to "search one at a time"
   - If SerperDevTool returns errors, agent must retry to meet requirements

3. **Potential Loop Triggers**:
   - API rate limits on SerperDev causing consistent failures
   - Malformed search queries due to context formatting
   - Missing or invalid SERPER_API_KEY causing authentication errors

### Step 2: Start AgentSight Monitoring for CrewAI Loop Detection

```bash
# Terminal 1: Start AgentSight with enhanced monitoring for CrewAI
cd /root/yunwei37/agentsight/collector
cargo run trace --ssl --process \
    --comm python --comm crewai \
    --server --log-file crewai_loop_detection.log \
    --process-filter "exec.contains('python')" \
    --ssl-filter "data.contains('openai') OR data.contains('serper')" \
    --http-filter "request.path.contains('/chat/completions') OR request.path.contains('/search')"
```

**Monitoring Focus Areas:**
- **SSL Traffic**: OpenAI API calls and SerperDev search requests
- **Process Events**: Python/CrewAI execution and subprocess creation
- **HTTP Patterns**: Chat completions and search API endpoints
- **Loop Indicators**: Repeated identical API calls with same parameters

### Step 3: Trigger CrewAI Reasoning Loop

```bash
# Terminal 2: Navigate to CrewAI project and run with loop-inducing parameters
cd latest_ai_development

# Method 1: Run with invalid/missing API keys to trigger tool failures
SERPER_API_KEY="" crewai run

# Method 2: Run with a complex topic that requires many searches
crewai run --topic "Latest developments in quantum computing applications for AI optimization in 2024"

# Method 3: Run with intentionally malformed context to trigger parsing errors
crewai run --topic "AI research" --context "Find exactly 20 unique references with working links to recent papers published in top-tier venues about this specific topic including arxiv preprints and conference proceedings"
```

### Step 4: Alternative - Create Controlled Loop Simulation

If the natural CrewAI loop doesn't trigger, create a controlled simulation:

```bash
# Create a modified version that forces SerperDevTool errors
cat > loop_inducer.py << 'EOF'
#!/usr/bin/env python3
"""
CrewAI Loop Inducer - Forces SerperDevTool to fail consistently
This simulates the research paper scenario where the agent gets stuck
"""
import os
import sys
from crewai import Agent, Task, Crew
from crewai_tools import SerperDevTool
from datetime import datetime
import time

class FailingSerperTool(SerperDevTool):
    """Modified SerperDevTool that always fails with the same error"""
    
    def _run(self, search_query):
        print(f"[{datetime.now()}] SerperDevTool called with: {search_query}")
        # Simulate consistent API failure
        error_msg = "Error: API quota exceeded. Please check your SERPER_API_KEY and billing status."
        print(f"[ERROR] {error_msg}")
        return error_msg

def create_looping_crew():
    """Create a CrewAI setup that will loop on tool failures"""
    
    # Agent that will get stuck in loop
    researcher = Agent(
        role="Persistent Researcher",
        goal="Find exactly 20 unique references about AI developments, no matter how many attempts it takes",
        backstory="You never give up and will keep trying the same approach until it works",
        tools=[FailingSerperTool()],
        verbose=True,
        max_retry_limit=10  # Allow many retries
    )
    
    # Task that requires successful tool usage
    research_task = Task(
        description="Search for 'latest AI developments 2024' and find 20 unique references. Keep trying until successful.",
        expected_output="List of 20 references with URLs",
        agent=researcher
    )
    
    # Crew that will execute the loop
    crew = Crew(
        agents=[researcher],
        tasks=[research_task],
        verbose=True
    )
    
    return crew

if __name__ == "__main__":
    print("=== CrewAI Reasoning Loop Simulation ===")
    print("This will demonstrate the 'try-fail-re-reason' loop pattern\n")
    
    # Create and run the looping crew
    crew = create_looping_crew()
    
    try:
        result = crew.kickoff()
        print(f"Final result: {result}")
    except KeyboardInterrupt:
        print("\n⚠️ Loop interrupted by user")
    except Exception as e:
        print(f"\n❌ Crew failed with error: {e}")
EOF

chmod +x loop_inducer.py
```

### Step 5: Create CrewAI-Specific Loop Detection Analyzer

```bash
# Create analyzer for detecting CrewAI loops in AgentSight logs
cat > crewai_loop_detector.py << 'EOF'
#!/usr/bin/env python3
"""
CrewAI-specific loop detection analyzer for AgentSight logs
Detects the 'try-fail-re-reason' pattern described in the research paper
"""
import json
import sys
from collections import defaultdict, deque
from datetime import datetime, timedelta
import re

class CrewAILoopDetector:
    def __init__(self, window_size=3, time_window_seconds=120):
        self.window_size = window_size
        self.time_window = timedelta(seconds=time_window_seconds)
        self.api_call_history = deque(maxlen=window_size)
        self.serper_calls = []
        self.openai_calls = []
        self.loop_patterns = []
        self.total_cost = 0.0
        
    def analyze_log(self, log_file):
        """Analyze AgentSight log for CrewAI reasoning loops"""
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
        
        # Categorize events
        for event in events_by_time:
            self._categorize_event(event)
        
        # Detect loop patterns
        self._detect_serper_loops()
        self._detect_openai_loops()
        self._analyze_cost_waste()
        self._generate_report()
    
    def _categorize_event(self, event):
        """Categorize events by type and extract relevant data"""
        if event.get('event_type') == 'ssl':
            data_str = str(event.get('data', {}))
            
            # Detect SerperDev API calls
            if 'serper' in data_str.lower() or 'google.com' in data_str:
                self.serper_calls.append({
                    'timestamp': event['timestamp'],
                    'data': event.get('data', {}),
                    'event': event
                })
            
            # Detect OpenAI API calls
            elif 'openai' in data_str.lower() or 'chat/completions' in data_str:
                # Estimate tokens and cost
                tokens = self._estimate_tokens(data_str)
                cost = tokens * 0.00015  # GPT-4o-mini pricing
                self.total_cost += cost
                
                self.openai_calls.append({
                    'timestamp': event['timestamp'],
                    'tokens': tokens,
                    'cost': cost,
                    'data': event.get('data', {}),
                    'event': event
                })
    
    def _estimate_tokens(self, data_str):
        """Rough token estimation for cost analysis"""
        # Simple word count * 1.3 (rough token-to-word ratio)
        word_count = len(data_str.split())
        return int(word_count * 1.3)
    
    def _detect_serper_loops(self):
        """Detect repeated SerperDev tool calls (the core loop pattern)"""
        if len(self.serper_calls) < self.window_size:
            return
        
        # Look for repeated search queries
        search_patterns = defaultdict(list)
        
        for call in self.serper_calls:
            # Extract search query from the call
            data_str = str(call['data'])
            # Look for similar search patterns
            query_hash = self._extract_search_pattern(data_str)
            search_patterns[query_hash].append(call)
        
        # Identify loops: same search repeated multiple times
        for pattern, calls in search_patterns.items():
            if len(calls) >= self.window_size:
                # Check if calls are within time window
                time_span = calls[-1]['timestamp'] - calls[0]['timestamp']
                if time_span <= self.time_window.total_seconds() * 1000:
                    self.loop_patterns.append({
                        'type': 'serper_search_loop',
                        'pattern': pattern,
                        'count': len(calls),
                        'duration_ms': time_span,
                        'calls': calls,
                        'severity': 'HIGH' if len(calls) >= 5 else 'MEDIUM'
                    })
    
    def _detect_openai_loops(self):
        """Detect repeated OpenAI API calls with similar prompts"""
        if len(self.openai_calls) < self.window_size:
            return
        
        # Look for rapid consecutive calls (indicating retry behavior)
        for i in range(len(self.openai_calls) - self.window_size + 1):
            window = self.openai_calls[i:i + self.window_size]
            
            # Check if calls are rapid (< 30 seconds apart)
            time_span = window[-1]['timestamp'] - window[0]['timestamp']
            if time_span < 30000:  # 30 seconds in milliseconds
                total_cost = sum(call['cost'] for call in window)
                
                self.loop_patterns.append({
                    'type': 'openai_rapid_retry',
                    'count': len(window),
                    'duration_ms': time_span,
                    'total_cost': total_cost,
                    'calls': window,
                    'severity': 'HIGH' if total_cost > 0.10 else 'MEDIUM'
                })
    
    def _extract_search_pattern(self, data_str):
        """Extract search pattern from API call data"""
        # Simple pattern extraction - in real implementation, 
        # this would be more sophisticated
        words = re.findall(r'\b\w+\b', data_str.lower())
        # Use key terms to identify similar searches
        key_terms = [w for w in words if len(w) > 3 and w not in ['http', 'https', 'data', 'json']]
        return ' '.join(sorted(key_terms[:5]))  # Use top 5 terms as pattern
    
    def _analyze_cost_waste(self):
        """Calculate wasted costs from loop behavior"""
        self.wasted_costs = {
            'serper_loops': 0,
            'openai_loops': 0,
            'total_openai_cost': self.total_cost
        }
        
        for pattern in self.loop_patterns:
            if pattern['type'] == 'serper_search_loop':
                # Assume each SerperDev call costs $0.001
                wasted = (pattern['count'] - 1) * 0.001  # First call is legitimate
                self.wasted_costs['serper_loops'] += wasted
            
            elif pattern['type'] == 'openai_rapid_retry':
                # Cost of retry calls
                wasted = pattern['total_cost'] * 0.8  # 80% of rapid retries are waste
                self.wasted_costs['openai_loops'] += wasted
    
    def _generate_report(self):
        """Generate comprehensive CrewAI loop detection report"""
        print("=== CREWAI REASONING LOOP DETECTION REPORT ===\n")
        
        print(f"📊 API Call Summary:")
        print(f"  SerperDev calls: {len(self.serper_calls)}")
        print(f"  OpenAI calls: {len(self.openai_calls)}")
        print(f"  Total OpenAI cost: ${self.total_cost:.4f}")
        print()
        
        if self.loop_patterns:
            print(f"⚠️  REASONING LOOPS DETECTED: {len(self.loop_patterns)}\n")
            
            for i, pattern in enumerate(self.loop_patterns, 1):
                print(f"Loop #{i} - {pattern['severity']} SEVERITY:")
                print(f"  Type: {pattern['type']}")
                print(f"  Repetitions: {pattern['count']}")
                print(f"  Duration: {pattern['duration_ms']/1000:.1f} seconds")
                
                if pattern['type'] == 'serper_search_loop':
                    print(f"  Search Pattern: {pattern['pattern']}")
                    print(f"  🔍 Same search query repeated {pattern['count']} times")
                elif pattern['type'] == 'openai_rapid_retry':
                    print(f"  Cost: ${pattern['total_cost']:.4f}")
                    print(f"  💸 Rapid retry pattern - wasted reasoning cycles")
                print()
        else:
            print("✅ No obvious reasoning loops detected\n")
        
        # Cost analysis
        total_waste = sum(self.wasted_costs.values()) - self.wasted_costs['total_openai_cost']
        if total_waste > 0:
            print(f"💰 Cost Waste Analysis:")
            print(f"  SerperDev loops: ${self.wasted_costs['serper_loops']:.4f}")
            print(f"  OpenAI loops: ${self.wasted_costs['openai_loops']:.4f}")
            print(f"  Total waste: ${total_waste:.4f}")
            print(f"  Waste percentage: {(total_waste/self.total_cost)*100:.1f}%")
            print()
        
        # Recommendations based on patterns found
        print("🔧 Recommendations:")
        if any(p['type'] == 'serper_search_loop' for p in self.loop_patterns):
            print("  1. Implement SerperDev error handling with circuit breaker")
            print("  2. Add exponential backoff for API failures")
            print("  3. Cache search results to avoid duplicate queries")
        
        if any(p['type'] == 'openai_rapid_retry' for p in self.loop_patterns):
            print("  1. Add delay between reasoning attempts")
            print("  2. Implement maximum retry limits")
            print("  3. Use different prompting strategies after failures")
        
        if self.loop_patterns:
            print("  4. Set resource consumption alerts")
            print("  5. Enable human intervention after N consecutive failures")
        else:
            print("  ✅ No immediate action required - normal operation detected")

if __name__ == "__main__":
    detector = CrewAILoopDetector()
    log_file = sys.argv[1] if len(sys.argv) > 1 else "crewai_loop_detection.log"
    
    print(f"Analyzing AgentSight log: {log_file}")
    print("=" * 50)
    
    try:
        detector.analyze_log(log_file)
    except FileNotFoundError:
        print(f"❌ Error: Log file '{log_file}' not found")
        print("Make sure AgentSight is running and generating logs")
    except Exception as e:
        print(f"❌ Error analyzing log: {e}")
EOF

chmod +x crewai_loop_detector.py
```

### Step 6: Execute the CrewAI Loop Experiment

```bash
# Terminal 2: Run CrewAI with loop-inducing conditions
cd latest_ai_development

# Method 1: Natural loop with missing API key
SERPER_API_KEY="" python loop_inducer.py

# Method 2: Run real CrewAI with challenging task
crewai run --inputs '{"topic": "latest AI developments 2024"}'

# Terminal 3: Real-time monitoring
tail -f ../crewai_loop_detection.log | grep -E "(serper|openai|api_call)"
```

### Step 7: Analyze Loop Detection Results

```bash
# Run the CrewAI-specific loop analysis
python crewai_loop_detector.py crewai_loop_detection.log

# Generate cost impact report
python -c "
import json, sys
total_cost = 0
api_calls = 0
with open('crewai_loop_detection.log', 'r') as f:
    for line in f:
        try:
            event = json.loads(line)
            if 'openai' in str(event).lower():
                api_calls += 1 
                total_cost += 0.002  # Rough estimate
        except: pass
print(f'API calls: {api_calls}, Estimated cost: \${total_cost:.4f}')
"
```

## CrewAI-Specific Loop Patterns

### 1. SerperDev Tool Error Loop
The most common pattern observed in the research paper:
```
1. Agent receives task: "Find 20 references about AI developments"
2. Calls SerperDevTool with query
3. Gets error: "API quota exceeded" or "Invalid API key"
4. LLM tries to reason about the error
5. Agent retries the EXACT same SerperDevTool call
6. Gets identical error → Loop continues
```

### 2. Context Window Exhaustion Loop
```
1. Initial SerperDev failures fill context with error messages
2. Agent loses track of original task requirements
3. Starts making increasingly desperate search attempts
4. Context fills with more errors, creating reasoning confusion
5. Agent falls back to repeating earliest failed approaches
```

### 3. Multi-Agent Coordination Loop
```
1. Researcher agent fails to get search results
2. Reporting analyst waits for research input
3. CrewAI framework retries researcher with same failing tool
4. Both agents get stuck in coordination deadlock
```

## Real-World Detection Results

### Expected AgentSight Observations:

**SSL Traffic Patterns:**
- Repeated identical HTTP requests to `serper.dev` API
- Same search parameters in request body
- Consistent 401/403 error responses

**Process Events:**
- Multiple Python subprocess spawns for same CrewAI task
- Identical command-line arguments across failed attempts
- Increasing memory usage without progress

**Cost Impact:**
- OpenAI API calls: ~50-100 calls during a 5-minute loop
- Estimated cost: $0.15-0.30 per loop incident
- SerperDev quota waste: Multiple API calls against quota limits

## Success Metrics for CrewAI Implementation

1. **Loop Detection Speed**: Identify SerperDev error patterns within 3 failed calls
2. **Cost Prevention**: Save >$0.20 per incident by early intervention
3. **Resource Protection**: Prevent CPU/memory runaway from infinite retries
4. **Real-time Alerts**: Notify operators within 60 seconds of loop detection

## Integration with CrewAI Framework

### Recommended Circuit Breaker Implementation:
```python
class LoopAwareSerperTool(SerperDevTool):
    def __init__(self):
        super().__init__()
        self.failure_count = 0
        self.last_error = None
        
    def _run(self, query):
        result = super()._run(query)
        
        if "error" in result.lower():
            self.failure_count += 1
            if self.failure_count >= 3:
                return "CIRCUIT_BREAKER: Tool disabled after repeated failures"
        else:
            self.failure_count = 0  # Reset on success
            
        return result
```

## Dashboard Visualization

AgentSight frontend timeline shows:
- 🔍 SerperDev API call frequency and error rates
- 💸 Real-time cost accumulation meter
- ⚠️ Loop detection alerts with severity levels
- 📊 Agent reasoning cycle duration metrics
- 🛑 Automatic intervention trigger points