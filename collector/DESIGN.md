# AI Agent Observability Framework - MVP Design

## Overview

A minimal CLI-driven observability framework using a fluent builder pattern where runners can have analyzers attached directly. Includes a RunnerOrchestrator for managing multiple runners simultaneously and server components for frontend integration.

## Core Architecture

### 1. Core Event System

```rust
pub struct Event {
    pub id: String,
    pub timestamp: u64,
    pub source: String,
    pub event_type: String,
    pub data: serde_json::Value,
}
```

### 2. Runners (Fluent Builder Pattern)

#### Base Runner Trait

```rust
#[async_trait]
pub trait Runner: Send + Sync {
    async fn run(&mut self) -> Result<EventStream, Box<dyn std::error::Error>>;
    fn add_analyzer(self, analyzer: Box<dyn Analyzer>) -> Self;
    fn name(&self) -> &str;
    fn id(&self) -> String; // Unique identifier for this runner instance
}

type EventStream = Pin<Box<dyn Stream<Item = ObservabilityEvent> + Send>>;
```

#### Runner Implementation

```rust
pub struct SslRunner {
    id: String,
    analyzers: Vec<Box<dyn Analyzer>>,
    config: SslConfig,
    executor: BinaryExecutor,
    additional_args: Vec<String>,
}

impl SslRunner {
    pub fn from_binary_extractor(binary_path: impl AsRef<Path>) -> Self;
    pub fn with_id(mut self, id: String) -> Self;
    pub fn with_args<I, S>(mut self, args: I) -> Self;
    pub fn tls_version(mut self, version: String) -> Self;
}

impl Runner for SslRunner {
    fn add_analyzer(mut self, analyzer: Box<dyn Analyzer>) -> Self {
        self.analyzers.push(analyzer);
        self
    }
    
    async fn run(&mut self) -> Result<EventStream, RunnerError> {
        let json_stream = self.executor.get_json_stream().await?;
        let event_stream = json_stream.map(|json_value| {
            Event::new_with_id_and_timestamp(
                Uuid::new_v4().to_string(),
                extract_timestamp(&json_value),
                "ssl".to_string(),
                json_value,
            )
        });
        AnalyzerProcessor::process_through_analyzers(Box::pin(event_stream), &mut self.analyzers).await
    }
}

pub struct ProcessRunner {
    id: String,
    analyzers: Vec<Box<dyn Analyzer>>,
    config: ProcessConfig,
    executor: BinaryExecutor,
    additional_args: Vec<String>,
}

impl ProcessRunner {
    pub fn from_binary_extractor(binary_path: impl AsRef<Path>) -> Self;
    pub fn with_id(mut self, id: String) -> Self;
    pub fn with_args<I, S>(mut self, args: I) -> Self;
    pub fn pid(mut self, pid: u32) -> Self;
    pub fn memory_threshold(mut self, threshold: u64) -> Self;
}

impl Runner for ProcessRunner {
    fn add_analyzer(mut self, analyzer: Box<dyn Analyzer>) -> Self {
        self.analyzers.push(analyzer);
        self
    }
    
    async fn run(&mut self) -> Result<EventStream, RunnerError> {
        let json_stream = self.executor.get_json_stream().await?;
        let event_stream = json_stream.map(|json_value| {
            Event::new_with_id_and_timestamp(
                Uuid::new_v4().to_string(),
                extract_timestamp(&json_value),
                "process".to_string(),
                json_value,
            )
        });
        AnalyzerProcessor::process_through_analyzers(Box::pin(event_stream), &mut self.analyzers).await
    }
}

/// Agent runner that runs both SSL and Process runners concurrently
///
/// This runner provides a simple interface for running both SSL and Process 
/// monitoring simultaneously. It returns a merged stream of events from both
/// sources. It's particularly useful for:
/// - Monitoring both network traffic and system processes
/// - Correlating SSL events with process events
/// - Simplified deployment with concurrent execution
/// - Stream-based processing with analyzer support
///
/// Note: This differs from the current main.rs implementation which spawns
/// separate tasks and waits for completion statistics. This design follows
/// the streaming architecture used by other runners and analyzers.
pub struct AgentRunner {
    id: String,
    analyzers: Vec<Box<dyn Analyzer>>,
    ssl_runner: SslRunner,
    process_runner: ProcessRunner,
}

impl AgentRunner {
    pub fn new(ssl_binary_path: impl AsRef<Path>, process_binary_path: impl AsRef<Path>) -> Self {
        let ssl_runner = SslRunner::from_binary_extractor(ssl_binary_path)
            .with_id("ssl".to_string());
        let process_runner = ProcessRunner::from_binary_extractor(process_binary_path)
            .with_id("process".to_string());
        
        Self {
            id: Uuid::new_v4().to_string(),
            analyzers: Vec::new(),
            ssl_runner,
            process_runner,
        }
    }
    
    pub fn with_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }
    
    pub fn with_ssl_args<I, S>(mut self, args: I) -> Self 
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.ssl_runner = self.ssl_runner.with_args(args);
        self
    }
    
    pub fn with_process_args<I, S>(mut self, args: I) -> Self 
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.process_runner = self.process_runner.with_args(args);
        self
    }
    
    pub fn with_comm_filter(self, comm: String) -> Self {
        self.with_ssl_args(vec!["-c", &comm])
            .with_process_args(vec!["-c", &comm])
    }
    
    pub fn with_pid_filter(self, pid: u32) -> Self {
        let pid_str = pid.to_string();
        self.with_ssl_args(vec!["-p", &pid_str])
            .with_process_args(vec!["-p", &pid_str])
    }
}

impl Runner for AgentRunner {
    fn add_analyzer(mut self, analyzer: Box<dyn Analyzer>) -> Self {
        self.analyzers.push(analyzer);
        self
    }
    
    async fn run(&mut self) -> Result<EventStream, RunnerError> {
        // Start both runners and get their streams
        let ssl_stream = self.ssl_runner.run().await?;
        let process_stream = self.process_runner.run().await?;
        
        // Merge the streams using select to interleave events as they arrive
        use futures::stream::{select, StreamExt};
        
        let merged_stream = select(ssl_stream, process_stream).map(|mut event| {
            // Tag events with their source runner for debugging
            if let Some(data) = event.data.as_object_mut() {
                data.insert("runner_source".to_string(), serde_json::json!(event.source.clone()));
            }
            event
        });
        
        // Process through analyzer chain
        AnalyzerProcessor::process_through_analyzers(Box::pin(merged_stream), &mut self.analyzers).await
    }
    
    fn name(&self) -> &str {
        "agent"
    }
    
    fn id(&self) -> String {
        self.id.clone()
    }
}

/// Generic combined runner that can dynamically combine any collection of runners
///
/// This runner provides a flexible abstraction for combining multiple runners
/// of any type into a single stream. It supports:
/// - Combining multiple instances of the same runner type (e.g., 2 SSL runners)
/// - Combining different runner types (e.g., SSL + Process)
/// - Recursive composition (combining already combined runners)
/// - Dynamic runtime configuration
/// - Stream merging with configurable strategies
/// - Analyzer chain processing on the merged stream
///
/// Examples:
/// - Combine 2 SSL runners monitoring different ports
/// - Combine 2 process runners monitoring different PIDs
/// - Combine SSL + Process runners for correlation
/// - Combine multiple combined runners for complex topologies
pub struct CombinedRunner {
    id: String,
    runners: Vec<Box<dyn Runner>>,
    analyzers: Vec<Box<dyn Analyzer>>,
    merge_strategy: MergeStrategy,
}

impl CombinedRunner {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            runners: Vec::new(),
            analyzers: Vec::new(),
            merge_strategy: MergeStrategy::TimeOrdered,
        }
    }
    
    pub fn with_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }
    
    pub fn add_runner(mut self, runner: Box<dyn Runner>) -> Self {
        self.runners.push(runner);
        self
    }
    
    pub fn with_merge_strategy(mut self, strategy: MergeStrategy) -> Self {
        self.merge_strategy = strategy;
        self
    }
    
    pub fn runner_count(&self) -> usize {
        self.runners.len()
    }
    
    pub fn get_runner_ids(&self) -> Vec<String> {
        self.runners.iter().map(|r| r.id()).collect()
    }
    
    // Convenience methods for common combinations
    pub fn ssl_and_process(ssl_binary_path: impl AsRef<Path>, process_binary_path: impl AsRef<Path>) -> Self {
        let ssl_runner = SslRunner::from_binary_extractor(ssl_binary_path)
            .with_id("ssl".to_string());
        let process_runner = ProcessRunner::from_binary_extractor(process_binary_path)
            .with_id("process".to_string());
        
        Self::new()
            .with_id("ssl-process-combined".to_string())
            .add_runner(Box::new(ssl_runner))
            .add_runner(Box::new(process_runner))
    }
    
    pub fn multiple_ssl<P: AsRef<Path>>(binary_path: P, configs: Vec<SslRunnerConfig>) -> Self {
        let mut combined = Self::new().with_id("multi-ssl-combined".to_string());
        
        for (i, config) in configs.into_iter().enumerate() {
            let mut ssl_runner = SslRunner::from_binary_extractor(&binary_path)
                .with_id(format!("ssl-{}", i));
            
            if !config.args.is_empty() {
                ssl_runner = ssl_runner.with_args(config.args);
            }
            
            if let Some(version) = config.tls_version {
                ssl_runner = ssl_runner.tls_version(version);
            }
            
            combined = combined.add_runner(Box::new(ssl_runner));
        }
        
        combined
    }
    
    pub fn multiple_process<P: AsRef<Path>>(binary_path: P, configs: Vec<ProcessRunnerConfig>) -> Self {
        let mut combined = Self::new().with_id("multi-process-combined".to_string());
        
        for (i, config) in configs.into_iter().enumerate() {
            let mut process_runner = ProcessRunner::from_binary_extractor(&binary_path)
                .with_id(format!("process-{}", i));
            
            if !config.args.is_empty() {
                process_runner = process_runner.with_args(config.args);
            }
            
            if let Some(pid) = config.pid {
                process_runner = process_runner.pid(pid);
            }
            
            if let Some(threshold) = config.memory_threshold {
                process_runner = process_runner.memory_threshold(threshold);
            }
            
            combined = combined.add_runner(Box::new(process_runner));
        }
        
        combined
    }
}

impl Runner for CombinedRunner {
    fn add_analyzer(mut self, analyzer: Box<dyn Analyzer>) -> Self {
        self.analyzers.push(analyzer);
        self
    }
    
    async fn run(&mut self) -> Result<EventStream, RunnerError> {
        if self.runners.is_empty() {
            return Err(RunnerError::Configuration("No runners configured".to_string()));
        }
        
        // Start all child runners and collect their streams
        let mut streams = Vec::new();
        for runner in &mut self.runners {
            let stream = runner.run().await?;
            streams.push(stream);
        }
        
        // Merge all streams based on the configured strategy
        let merged_stream = self.merge_streams(streams).await?;
        
        // Process through analyzer chain
        AnalyzerProcessor::process_through_analyzers(merged_stream, &mut self.analyzers).await
    }
    
    fn name(&self) -> &str {
        "combined"
    }
    
    fn id(&self) -> String {
        self.id.clone()
    }
}

impl CombinedRunner {
    async fn merge_streams(&self, streams: Vec<EventStream>) -> Result<EventStream, RunnerError> {
        use futures::stream::{select_all, StreamExt};
        
        if streams.is_empty() {
            return Err(RunnerError::Configuration("No streams to merge".to_string()));
        }
        
        if streams.len() == 1 {
            return Ok(streams.into_iter().next().unwrap());
        }
        
        match self.merge_strategy {
            MergeStrategy::TimeOrdered => {
                // For time-ordered merging, we'd need a more sophisticated approach
                // For now, use immediate merging as a fallback
                let merged = select_all(streams).map(|mut event| {
                    // Tag events with combined runner info
                    if let Some(data) = event.data.as_object_mut() {
                        data.insert("combined_runner_id".to_string(), serde_json::json!(self.id.clone()));
                    }
                    event
                });
                Ok(Box::pin(merged))
            },
            MergeStrategy::Immediate => {
                let merged = select_all(streams).map(|mut event| {
                    // Tag events with combined runner info
                    if let Some(data) = event.data.as_object_mut() {
                        data.insert("combined_runner_id".to_string(), serde_json::json!(self.id.clone()));
                    }
                    event
                });
                Ok(Box::pin(merged))
            },
            MergeStrategy::RoundRobin => {
                // TODO: Implement round-robin merging
                let merged = select_all(streams).map(|mut event| {
                    if let Some(data) = event.data.as_object_mut() {
                        data.insert("combined_runner_id".to_string(), serde_json::json!(self.id.clone()));
                    }
                    event
                });
                Ok(Box::pin(merged))
            },
            MergeStrategy::Priority => {
                // TODO: Implement priority-based merging
                let merged = select_all(streams).map(|mut event| {
                    if let Some(data) = event.data.as_object_mut() {
                        data.insert("combined_runner_id".to_string(), serde_json::json!(self.id.clone()));
                    }
                    event
                });
                Ok(Box::pin(merged))
            },
        }
    }
}

// Configuration structs for convenience methods
pub struct SslRunnerConfig {
    pub args: Vec<String>,
    pub tls_version: Option<String>,
}

pub struct ProcessRunnerConfig {
    pub args: Vec<String>,
    pub pid: Option<u32>,
    pub memory_threshold: Option<u64>,
}

pub enum MergeStrategy {
    TimeOrdered,    // Merge by timestamp (default)
    Immediate,      // First-come-first-served
    RoundRobin,     // Alternate between streams
    Priority,       // Priority-based merging using runner priorities
}
```

### 3. Runner Orchestrator

#### Orchestrator for Multiple Runners

```rust
pub struct RunnerOrchestrator {
    runners: HashMap<String, Box<dyn Runner>>,
    active_tasks: HashMap<String, JoinHandle<Result<(), Box<dyn std::error::Error>>>>,
    stream_merger: StreamMerger,
    storage: Arc<dyn Storage>,
}

impl RunnerOrchestrator {
    pub fn new(storage: Arc<dyn Storage>) -> Self;
    
    // Builder-style runner registration
    pub fn add_runner(mut self, runner: Box<dyn Runner>) -> Self;
    
    // Individual runner control
    pub async fn start_runner(&mut self, runner_id: &str) -> Result<(), Box<dyn std::error::Error>>;
    pub async fn stop_runner(&mut self, runner_id: &str) -> Result<(), Box<dyn std::error::Error>>;
    
    // Bulk operations
    pub async fn start_all(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    pub async fn stop_all(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    
    // Stream management
    pub async fn get_merged_stream(&self) -> Result<EventStream, Box<dyn std::error::Error>>;
    
    // Status and monitoring
    pub fn list_runners(&self) -> Vec<RunnerInfo>;
    pub fn get_runner_status(&self, runner_id: &str) -> Option<RunnerStatus>;
}

pub struct RunnerInfo {
    pub id: String,
    pub name: String,
    pub status: RunnerStatus,
    pub events_processed: u64,
    pub last_event_time: Option<u64>,
}

pub enum RunnerStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
}
```

#### Stream Merger

```rust
pub struct StreamMerger {
    merge_strategy: MergeStrategy,
    buffer_size: usize,
}

pub enum MergeStrategy {
    TimeOrdered,        // Merge by timestamp
    Immediate,          // First-come-first-served
}

impl StreamMerger {
    pub fn new(strategy: MergeStrategy) -> Self;
    
    pub async fn merge_streams(
        &self, 
        streams: Vec<(String, EventStream)>
    ) -> Result<EventStream, Box<dyn std::error::Error>>;
}
```

### 4. Analyzers (Stream Processors)

#### Base Analyzer Trait

```rust
#[async_trait]
pub trait Analyzer: Send + Sync {
    async fn process(&mut self, stream: EventStream) -> Result<EventStream, Box<dyn std::error::Error>>;
    fn name(&self) -> &str;
}
```

#### Analyzer Types

- **RawAnalyzer**: Pass-through for raw JSON output
- **ExtractAnalyzer**: Extract specific fields/patterns
- **MergeAnalyzer**: Combine related events
- **FilterAnalyzer**: Filter events by criteria
- **CountAnalyzer**: Count and aggregate events
- **StorageAnalyzer**: Store events in memory/backend
- **CorrelationAnalyzer**: Cross-runner event correlation

### 5. Storage System

#### Storage Trait

```rust
#[async_trait]
pub trait Storage: Send + Sync {
    async fn store(&self, event: ObservabilityEvent) -> Result<(), Box<dyn std::error::Error>>;
    async fn query(&self, query: StorageQuery) -> Result<Vec<ObservabilityEvent>, Box<dyn std::error::Error>>;
    async fn get_stats(&self) -> Result<StorageStats, Box<dyn std::error::Error>>;
    async fn get_runner_stats(&self, runner_id: &str) -> Result<RunnerStats, Box<dyn std::error::Error>>;
}

pub struct StorageQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub filters: HashMap<String, String>,
    pub time_range: Option<(u64, u64)>,
    pub runner_ids: Option<Vec<String>>, // Filter by specific runners
}

pub struct StorageStats {
    pub total_events: usize,
    pub events_by_type: HashMap<String, usize>,
    pub events_by_runner: HashMap<String, usize>,
    pub last_event_time: Option<u64>,
}

pub struct RunnerStats {
    pub runner_id: String,
    pub event_count: usize,
    pub first_event_time: Option<u64>,
    pub last_event_time: Option<u64>,
    pub events_by_type: HashMap<String, usize>,
}
```

#### In-Memory Storage

```rust
pub struct InMemoryStorage {
    events: Arc<RwLock<Vec<ObservabilityEvent>>>,
    max_events: usize,
    indices: Arc<RwLock<HashMap<String, Vec<usize>>>>,
    runner_indices: Arc<RwLock<HashMap<String, Vec<usize>>>>, // Index by runner
}

impl InMemoryStorage {
    pub fn new(max_events: usize) -> Self;
    pub fn shared() -> Arc<dyn Storage>; // Singleton for sharing across components
}
```

### 6. Server Component

#### Enhanced REST API Server

```rust
pub struct ObservabilityServer {
    orchestrator: Arc<Mutex<RunnerOrchestrator>>,
    storage: Arc<dyn Storage>,
    bind_address: String,
}

impl ObservabilityServer {
    pub fn new(
        orchestrator: Arc<Mutex<RunnerOrchestrator>>, 
        storage: Arc<dyn Storage>, 
        bind_address: String
    ) -> Self;
    
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>>;
}
```

#### Enhanced API Endpoints

```
GET    /health                    - Health check
GET    /runners                   - List all runners and their status
POST   /runners/{id}/start        - Start specific runner
POST   /runners/{id}/stop         - Stop specific runner
POST   /runners/start-all         - Start all runners
POST   /runners/stop-all          - Stop all runners
GET    /events                    - Query stored events
GET    /events/stats              - Get storage statistics
GET    /events/runners/{id}/stats - Get runner-specific statistics
GET    /events/stream             - SSE stream of live merged events
POST   /events/query              - Advanced query with filters
GET    /stream/merged             - Live merged stream from all active runners
GET    /stream/runner/{id}        - Live stream from specific runner
```

### 7. Output Handlers

```rust
pub enum OutputMode {
    Stdout,
    File(String),
    Json,
    Pretty,
    Server(String), // Start server on address
}

pub struct OutputHandler {
    mode: OutputMode,
    storage: Option<Arc<dyn Storage>>,
}
```

## Implementation Examples

### 1. SSL Runner

```rust
// agent-tracer ssl --sse-merge -- --port 443
let mut ssl_runner = SslRunner::from_binary_extractor(binary_extractor.get_sslsniff_path())
    .with_id("ssl-raw".to_string())
    .with_args(vec!["--port", "443"])
    .add_analyzer(Box::new(ChunkMerger::new_with_timeout(30000)))
    .add_analyzer(Box::new(FileLogger::new("ssl.log").unwrap()))
    .add_analyzer(Box::new(OutputAnalyzer::new()));

let mut stream = ssl_runner.run().await?;
while let Some(_event) = stream.next().await {
    // Events are processed by the analyzers in the chain
}
```

### 2. Process Runner

```rust
// agent-tracer process -- --pid 1234
let mut process_runner = ProcessRunner::from_binary_extractor(binary_extractor.get_process_path())
    .with_id("process-raw".to_string())
    .with_args(vec!["--pid", "1234"])
    .add_analyzer(Box::new(OutputAnalyzer::new()));

let mut stream = process_runner.run().await?;
while let Some(_event) = stream.next().await {
    // Events are processed by the analyzers in the chain
}
```

### 3. Agent Runner (Both SSL and Process)

```rust
// agent-tracer agent --comm python --pid 1234
let binary_extractor = BinaryExtractor::new().await?;

let mut agent_runner = AgentRunner::new(
    binary_extractor.get_sslsniff_path(), 
    binary_extractor.get_process_path()
)
.with_id("agent-both".to_string())
.with_comm_filter("python".to_string())
.with_pid_filter(1234)
.add_analyzer(Box::new(OutputAnalyzer::new()));

// Get the merged stream from both SSL and process runners
let mut stream = agent_runner.run().await?;

// Process events as they arrive from either runner
while let Some(event) = stream.next().await {
    // Events are processed by the analyzers in the chain
    // Each event is tagged with its source ("ssl" or "process")
}
```

### 4. Combined Runner (Multiple SSL Runners)

```rust
// agent-tracer combined --ssl-binary /path/to/sslsniff --ssl-args "--port 443" --ssl-args "--port 8443"
let binary_extractor = BinaryExtractor::new().await?;

let mut combined_runner = CombinedRunner::new()
    .with_id("multi-ssl-combined".to_string())
    .add_runner(Box::new(SslRunner::from_binary_extractor(binary_extractor.get_sslsniff_path())
        .with_id("ssl-port-443".to_string())
        .with_args(vec!["--port", "443"])))
    .add_runner(Box::new(SslRunner::from_binary_extractor(binary_extractor.get_sslsniff_path())
        .with_id("ssl-port-8443".to_string())
        .with_args(vec!["--port", "8443"])));

let mut stream = combined_runner.run().await?;
while let Some(event) = stream.next().await {
    // Events are processed by the analyzers in the chain
    // Each event is tagged with the combined runner ID
}
```

### 5. Combined Runner (Multiple Process Runners)

```rust
// Monitor multiple processes with different PIDs
let binary_extractor = BinaryExtractor::new().await?;

let mut combined_runner = CombinedRunner::new()
    .with_id("multi-process-combined".to_string())
    .add_runner(Box::new(ProcessRunner::from_binary_extractor(binary_extractor.get_process_path())
        .with_id("process-python".to_string())
        .with_args(vec!["-c", "python"])))
    .add_runner(Box::new(ProcessRunner::from_binary_extractor(binary_extractor.get_process_path())
        .with_id("process-node".to_string())
        .with_args(vec!["-c", "node"])))
    .add_runner(Box::new(ProcessRunner::from_binary_extractor(binary_extractor.get_process_path())
        .with_id("process-1234".to_string())
        .with_args(vec!["-p", "1234"])))
    .add_analyzer(Box::new(OutputAnalyzer::new()));

let mut stream = combined_runner.run().await?;
while let Some(event) = stream.next().await {
    // Events from all three process runners are merged
}
```

### 6. Combined Runner (SSL + Process Mix)

```rust
// Combine SSL and Process runners for comprehensive monitoring
let binary_extractor = BinaryExtractor::new().await?;

let mut combined_runner = CombinedRunner::new()
    .with_id("ssl-process-mix".to_string())
    .add_runner(Box::new(SslRunner::from_binary_extractor(binary_extractor.get_sslsniff_path())
        .with_id("ssl-https".to_string())
        .with_args(vec!["--port", "443"])))
    .add_runner(Box::new(SslRunner::from_binary_extractor(binary_extractor.get_sslsniff_path())
        .with_id("ssl-dev".to_string())
        .with_args(vec!["--port", "8443"])))
    .add_runner(Box::new(ProcessRunner::from_binary_extractor(binary_extractor.get_process_path())
        .with_id("process-web".to_string())
        .with_args(vec!["-c", "nginx"])))
    .add_runner(Box::new(ProcessRunner::from_binary_extractor(binary_extractor.get_process_path())
        .with_id("process-app".to_string())
        .with_args(vec!["-c", "python"])))
    .with_merge_strategy(MergeStrategy::TimeOrdered)
    .add_analyzer(Box::new(CorrelationAnalyzer::new()))
    .add_analyzer(Box::new(OutputAnalyzer::new()));

let mut stream = combined_runner.run().await?;
while let Some(event) = stream.next().await {
    // Events from 2 SSL runners and 2 Process runners are merged and correlated
}
```

### 7. Recursive Combined Runner (Combining Combined Runners)

```rust
// Create nested combinations for complex monitoring topologies
let binary_extractor = BinaryExtractor::new().await?;

// Create a combined runner for web services (SSL + Process)
let web_services_runner = CombinedRunner::new()
    .with_id("web-services".to_string())
    .add_runner(Box::new(SslRunner::from_binary_extractor(binary_extractor.get_sslsniff_path())
        .with_id("ssl-web".to_string())
        .with_args(vec!["--port", "443"])))
    .add_runner(Box::new(ProcessRunner::from_binary_extractor(binary_extractor.get_process_path())
        .with_id("process-web".to_string())
        .with_args(vec!["-c", "nginx"])))
    .add_analyzer(Box::new(FilterAnalyzer::new("web_filter")));

// Create a combined runner for database services
let db_services_runner = CombinedRunner::new()
    .with_id("db-services".to_string())
    .add_runner(Box::new(ProcessRunner::from_binary_extractor(binary_extractor.get_process_path())
        .with_id("process-postgres".to_string())
        .with_args(vec!["-c", "postgres"])))
    .add_runner(Box::new(ProcessRunner::from_binary_extractor(binary_extractor.get_process_path())
        .with_id("process-redis".to_string())
        .with_args(vec!["-c", "redis"])))
    .add_analyzer(Box::new(FilterAnalyzer::new("db_filter")));

// Combine the combined runners into a master runner
let mut master_runner = CombinedRunner::new()
    .with_id("master-services".to_string())
    .add_runner(Box::new(web_services_runner))
    .add_runner(Box::new(db_services_runner))
    .with_merge_strategy(MergeStrategy::Priority)
    .add_analyzer(Box::new(CorrelationAnalyzer::new()))
    .add_analyzer(Box::new(StorageAnalyzer::new(InMemoryStorage::shared())))
    .add_analyzer(Box::new(OutputAnalyzer::new()));

let mut stream = master_runner.run().await?;
while let Some(event) = stream.next().await {
    // Events from nested combined runners are merged with priority strategy
}
```

### 8. Convenience Methods for Common Patterns

```rust
// Using convenience methods for common combinations
let binary_extractor = BinaryExtractor::new().await?;

// SSL + Process combination (equivalent to AgentRunner)
let agent_runner = CombinedRunner::ssl_and_process(
    binary_extractor.get_sslsniff_path(),
    binary_extractor.get_process_path()
)
.add_analyzer(Box::new(OutputAnalyzer::new()));

// Multiple SSL runners with different configurations
let multi_ssl_runner = CombinedRunner::multiple_ssl(
    binary_extractor.get_sslsniff_path(),
    vec![
        SslRunnerConfig {
            args: vec!["--port".to_string(), "443".to_string()],
            tls_version: Some("1.3".to_string()),
        },
        SslRunnerConfig {
            args: vec!["--port".to_string(), "8443".to_string()],
            tls_version: Some("1.2".to_string()),
        },
    ]
)
.add_analyzer(Box::new(OutputAnalyzer::new()));

// Multiple Process runners with different configurations
let multi_process_runner = CombinedRunner::multiple_process(
    binary_extractor.get_process_path(),
    vec![
        ProcessRunnerConfig {
            args: vec!["-c".to_string(), "python".to_string()],
            pid: None,
            memory_threshold: Some(1024 * 1024 * 100), // 100MB
        },
        ProcessRunnerConfig {
            args: vec!["-c".to_string(), "node".to_string()],
            pid: None,
            memory_threshold: Some(1024 * 1024 * 200), // 200MB
        },
        ProcessRunnerConfig {
            args: vec![],
            pid: Some(1234),
            memory_threshold: None,
        },
    ]
)
.add_analyzer(Box::new(OutputAnalyzer::new()));
```

### 9. Dynamic Runtime Configuration

```rust
// Build combined runners dynamically at runtime
let binary_extractor = BinaryExtractor::new().await?;
let mut combined_runner = CombinedRunner::new().with_id("dynamic-combined".to_string());

// Add runners based on runtime conditions
if monitor_ssl {
    combined_runner = combined_runner.add_runner(Box::new(SslRunner::from_binary_extractor(binary_extractor.get_sslsniff_path())
        .with_id("ssl-dynamic".to_string())
        .with_args(ssl_args)));
}

if monitor_processes {
    for pid in process_pids {
        combined_runner = combined_runner.add_runner(Box::new(ProcessRunner::from_binary_extractor(binary_extractor.get_process_path())
            .with_id(format!("process-{}", pid))
            .pid(pid)));
    }
}

// Add analyzers based on requirements
if enable_correlation {
    combined_runner = combined_runner.add_analyzer(Box::new(CorrelationAnalyzer::new()));
}

if enable_storage {
    combined_runner = combined_runner.add_analyzer(Box::new(StorageAnalyzer::new(InMemoryStorage::shared())));
}

combined_runner = combined_runner.add_analyzer(Box::new(OutputAnalyzer::new()));

let mut stream = combined_runner.run().await?;
while let Some(event) = stream.next().await {
    // Handle dynamically configured events
}
```

## CLI Design

### Focused Subcommands

```bash
# ssl: Analyze SSL traffic with raw JSON output
agent-tracer ssl [OPTIONS]

# process: Test process runner with embedded binary
agent-tracer process [OPTIONS]

# agent: Test both runners with embedded binaries
agent-tracer agent [OPTIONS]

# combined: Test multiple runners with embedded binaries
agent-tracer combined [OPTIONS]
```

### CLI Architecture

```rust
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze SSL traffic with raw JSON output
    Ssl {
        /// Enable HTTP chunk merging for SSL traffic
        #[arg(long)]
        sse_merge: bool,
        /// Additional arguments to pass to the SSL binary
        #[arg(last = true)]
        args: Vec<String>,
    },
    
    /// Test process runner with embedded binary
    Process {
        /// Additional arguments to pass to the process binary
        #[arg(last = true)]
        args: Vec<String>,
    },
    
    /// Test both runners with embedded binaries
    Agent {
        /// Filter by process command name (comma-separated list)
        #[arg(short = 'c', long)]
        comm: Option<String>,
        /// Filter by process PID
        #[arg(short = 'p', long)]
        pid: Option<u32>,
    },

    /// Test multiple runners with embedded binaries
    Combined {
        /// Path to the binary for SSL runners
        #[arg(long)]
        ssl_binary: Option<String>,
        /// Path to the binary for Process runners
        #[arg(long)]
        process_binary: Option<String>,
        /// Arguments for SSL runners (repeatable)
        #[arg(last = true)]
        ssl_args: Vec<String>,
        /// Arguments for Process runners (repeatable)
        #[arg(last = true)]
        process_args: Vec<String>,
        /// Filter by process command name (comma-separated list) for combined SSL runners
        #[arg(short = 'c', long)]
        comm: Option<String>,
        /// Filter by process PID for combined Process runners
        #[arg(short = 'p', long)]
        pid: Option<u32>,
    },
}
```

## Command Implementations

### SSL Command

```rust
// agent-tracer ssl --sse-merge -- --port 443
async fn run_raw_ssl(binary_extractor: &BinaryExtractor, enable_chunk_merger: bool, args: &Vec<String>) -> Result<(), RunnerError> {
    let mut ssl_runner = SslRunner::from_binary_extractor(binary_extractor.get_sslsniff_path())
        .with_id("ssl-raw".to_string());
    
    if !args.is_empty() {
        ssl_runner = ssl_runner.with_args(args);
    }
    
    if enable_chunk_merger {
        ssl_runner = ssl_runner.add_analyzer(Box::new(ChunkMerger::new_with_timeout(30000)));
    }
    
    ssl_runner = ssl_runner
        .add_analyzer(Box::new(FileLogger::new("ssl.log").unwrap()))
        .add_analyzer(Box::new(OutputAnalyzer::new()));
    
    let mut stream = ssl_runner.run().await?;
    while let Some(_event) = stream.next().await {
        // Events are processed by the analyzers in the chain
    }
    
    Ok(())
}
```

### Process Command

```rust
// agent-tracer process -- --pid 1234
async fn run_raw_process(binary_extractor: &BinaryExtractor, args: &Vec<String>) -> Result<(), RunnerError> {
    let mut process_runner = ProcessRunner::from_binary_extractor(binary_extractor.get_process_path())
        .with_id("process-raw".to_string());
    
    if !args.is_empty() {
        process_runner = process_runner.with_args(args);
    }
    
    process_runner = process_runner.add_analyzer(Box::new(OutputAnalyzer::new()));
    
    let mut stream = process_runner.run().await?;
    while let Some(_event) = stream.next().await {
        // Events are processed by the analyzers in the chain
    }
    
    Ok(())
}
```

### Agent Command (Concurrent SSL + Process)

```rust
// agent-tracer agent --comm python --pid 1234
async fn run_both_real(binary_extractor: &BinaryExtractor, comm: Option<&str>, pid: Option<u32>) -> Result<(), Box<dyn std::error::Error>> {
    // Build arguments for filtering
    let mut args = Vec::new();
    if let Some(comm_filter) = comm {
        args.push("-c".to_string());
        args.push(comm_filter.to_string());
    }
    if let Some(pid_filter) = pid {
        args.push("-p".to_string());
        args.push(pid_filter.to_string());
    }
    
    // Spawn both runners concurrently
    let ssl_handle = tokio::spawn(async move {
        let mut ssl_runner = SslRunner::from_binary_extractor(ssl_path)
            .with_id("ssl-both".to_string())
            .with_args(&args)
            .add_analyzer(Box::new(OutputAnalyzer::new()));
        
        match ssl_runner.run().await {
            Ok(mut stream) => {
                let mut count = 0;
                while let Some(_event) = stream.next().await {
                    count += 1;
                }
                count
            }
            Err(e) => {
                println!("SSL Runner error: {}", e);
                0
            }
        }
    });
    
    let process_handle = tokio::spawn(async move {
        let mut process_runner = ProcessRunner::from_binary_extractor(process_path)
            .with_id("process".to_string())
            .with_args(&args)
            .add_analyzer(Box::new(OutputAnalyzer::new()));
        
        match process_runner.run().await {
            Ok(mut stream) => {
                let mut count = 0;
                while let Some(_event) = stream.next().await {
                    count += 1;
                }
                count
            }
            Err(e) => {
                println!("Process Runner error: {}", e);
                0
            }
        }
    });
    
    let (ssl_count, process_count) = tokio::try_join!(ssl_handle, process_handle)?;
    
    println!("Both runners completed!");
    println!("SSL events: {}", ssl_count);
    println!("Process events: {}", process_count);
    
    Ok(())
}
```

### Combined Command

```rust
// agent-tracer combined --ssl-binary /path/to/sslsniff --ssl-args "--port 443" --ssl-args "--port 8443"
async fn run_combined(binary_extractor: &BinaryExtractor, ssl_binary_path: Option<&str>, process_binary_path: Option<&str>, ssl_args: Vec<String>, process_args: Vec<String>, comm: Option<&str>, pid: Option<u32>) -> Result<(), Box<dyn std::error::Error>> {
    let mut combined_runner = CombinedRunner::new();

    if let Some(ssl_binary) = ssl_binary_path {
        combined_runner = combined_runner.add_runner(Box::new(SslRunner::from_binary_extractor(ssl_binary)
            .with_id("ssl-port-443".to_string())
            .with_args(vec!["--port", "443"])));
    }
    if let Some(process_binary) = process_binary_path {
        combined_runner = combined_runner.add_runner(Box::new(ProcessRunner::from_binary_extractor(process_binary)
            .with_id("process-pid-1234".to_string())
            .with_args(vec!["--pid", "1234"])));
    }

    if let Some(comm_filter) = comm {
        combined_runner = combined_runner.with_comm_filter(comm_filter.to_string());
    }
    if let Some(pid_filter) = pid {
        combined_runner = combined_runner.with_pid_filter(pid_filter);
    }

    let mut stream = combined_runner.run().await?;
    while let Some(event) = stream.next().await {
        // Events are processed by the analyzers in the chain
        // Each event is tagged with the combined runner ID
    }

    Ok(())
}
```

## Usage Examples

```bash
# SSL traffic analysis with chunk merging
agent-tracer ssl --sse-merge -- --port 443 --interface eth0

# SSL traffic analysis with raw output
agent-tracer ssl -- --port 8443

# Process monitoring with specific PID
agent-tracer process -- --pid 1234

# Process monitoring with custom arguments
agent-tracer process -- -c python -p 5678

# Agent mode - both SSL and process monitoring concurrently
agent-tracer agent --comm python --pid 1234

# Agent mode with command filtering
agent-tracer agent -c "node,python" -p 5678

# Agent mode with just PID filtering
agent-tracer agent -p 1234

# Agent mode with just command filtering
agent-tracer agent -c python

# Combined mode - multiple SSL runners
agent-tracer combined --ssl-binary /path/to/sslsniff --ssl-args "--port 443" --ssl-args "--port 8443"

# Combined mode - multiple process runners
agent-tracer combined --process-binary /path/to/process --process-args "--pid 1234" --process-args "--pid 5678"

# Combined mode - SSL and Process runners with filtering
agent-tracer combined --ssl-binary /path/to/sslsniff --ssl-args "--port 443" --process-binary /path/to/process --process-args "--pid 1234" --comm "node,python"
```

This redesign provides clean, focused subcommands where each has a clear purpose and intuitive configuration options!
