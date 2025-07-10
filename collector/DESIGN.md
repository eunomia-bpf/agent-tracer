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
}

impl SslRunner {
    pub fn new() -> Self;
    pub fn with_id(id: String) -> Self;
    pub fn port(mut self, port: u16) -> Self;
    pub fn interface(mut self, interface: String) -> Self;
}

impl Runner for SslRunner {
    fn add_analyzer(mut self, analyzer: Box<dyn Analyzer>) -> Self {
        self.analyzers.push(analyzer);
        self
    }
    
    async fn run(&mut self) -> Result<EventStream, Box<dyn std::error::Error>> {
        let raw_stream = self.collect_ssl_events().await?;
        self.process_through_analyzers(raw_stream).await
    }
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

### 1. Simple Single Runner
```rust
// agent-tracer sslsniff --raw
SslRunner::new()
    .add_analyzer(Box::new(RawAnalyzer::new()))
    .run()
    .await?
```

### 2. Multiple Runners with Orchestrator
```rust
// agent-tracer sslsniff process --merge --store --serve 0.0.0.0:8080
let storage = InMemoryStorage::shared();

let orchestrator = RunnerOrchestrator::new(storage.clone())
    .add_runner(Box::new(
        SslRunner::new()
            .with_id("ssl-443".to_string())
            .port(443)
            .add_analyzer(Box::new(ExtractAnalyzer::new(vec!["host", "port"])))
            .add_analyzer(Box::new(StorageAnalyzer::new(storage.clone())))
    ))
    .add_runner(Box::new(
        ProcessRunner::new()
            .with_id("proc-python".to_string())
            .filter("name:python")
            .add_analyzer(Box::new(ExtractAnalyzer::new(vec!["pid", "cpu"])))
            .add_analyzer(Box::new(StorageAnalyzer::new(storage.clone())))
    ));

// Start all runners
orchestrator.start_all().await?;

// Start server for management
let server = ObservabilityServer::new(
    Arc::new(Mutex::new(orchestrator)),
    storage,
    "0.0.0.0:8080".to_string()
);
server.start().await?;
```

### 3. Dynamic Runner Management via API
```rust
// Server can manage runners dynamically
let orchestrator = RunnerOrchestrator::new(storage.clone());

// Add runners without starting them
orchestrator.add_runner(Box::new(SslRunner::new().with_id("ssl-main")));
orchestrator.add_runner(Box::new(ProcessRunner::new().with_id("proc-main")));

// Start server - runners can be controlled via API
let server = ObservabilityServer::new(
    Arc::new(Mutex::new(orchestrator)),
    storage,
    "0.0.0.0:8080".to_string()
);
server.start().await?;

// Frontend can now:
// POST /runners/ssl-main/start
// POST /runners/proc-main/start  
// GET /stream/merged (combined stream)
```

### 4. Advanced Stream Merging
```rust
let storage = InMemoryStorage::shared();

let mut orchestrator = RunnerOrchestrator::new(storage.clone())
    .add_runner(Box::new(
        SslRunner::new()
            .with_id("ssl-high-priority")
            .add_analyzer(Box::new(FilterAnalyzer::new("severity:critical")))
            .add_analyzer(Box::new(StorageAnalyzer::new(storage.clone())))
    ))
    .add_runner(Box::new(
        ProcessRunner::new()
            .with_id("process-low-priority")
            .add_analyzer(Box::new(FilterAnalyzer::new("cpu<10")))
            .add_analyzer(Box::new(StorageAnalyzer::new(storage.clone())))
    ));

// Configure priority-based merging
orchestrator.stream_merger = StreamMerger::new(
    MergeStrategy::Priority(hashmap! {
        "ssl-high-priority".to_string() => 10,
        "process-low-priority".to_string() => 1,
    })
);

orchestrator.start_all().await?;

// Get merged stream with priority ordering
let merged_stream = orchestrator.get_merged_stream().await?;
```

## CLI Design

### Focused Subcommands
```bash
# observe: Capture and combine events from both SSL and process monitoring
agent-tracer observe [OPTIONS]

# api: Capture only SSL/TLS API events  
agent-tracer api [OPTIONS]

# process: Capture only system/process events
agent-tracer process [OPTIONS]

# server: Start server mode for frontend integration
agent-tracer server [OPTIONS]

# dyn: Dynamic configuration and runner management
agent-tracer dyn [COMMAND]
```

### CLI Architecture
```rust
#[derive(Parser)]
#[command(name = "agent-tracer")]
#[command(about = "AI Agent Observability Framework")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Capture and combine events from both SSL and process monitoring
    Observe {
        // Analyzer flags
        #[arg(long)]
        raw: bool,
        
        #[arg(long)]
        extract: Option<String>,
        
        #[arg(long)]
        filter: Option<String>,
        
        #[arg(long)]
        merge: bool,
        
        #[arg(long)]
        count: bool,
        
        #[arg(long)]
        store: bool,
        
        // Merge strategy for combining streams
        #[arg(long, default_value = "time-ordered")]
        merge_strategy: MergeStrategy,
        
        // Output options
        #[arg(long)]
        output: Option<OutputMode>,
        
        // Runner-specific config
        #[arg(long)]
        ssl_port: Option<u16>,
        
        #[arg(long)]
        ssl_interface: Option<String>,
        
        #[arg(long)]
        process_filter: Option<String>,
    },
    
    /// Capture only SSL/TLS API events
    Api {
        // SSL-specific config
        #[arg(long)]
        port: Option<u16>,
        
        #[arg(long)]
        interface: Option<String>,
        
        #[arg(long)]
        tls_version: Option<String>,
        
        // Analyzer flags
        #[arg(long)]
        raw: bool,
        
        #[arg(long)]
        extract: Option<String>,
        
        #[arg(long)]
        filter: Option<String>,
        
        #[arg(long)]
        merge: bool,
        
        #[arg(long)]
        count: bool,
        
        #[arg(long)]
        store: bool,
        
        // Output options
        #[arg(long)]
        output: Option<OutputMode>,
    },
    
    /// Capture only system/process events
    Process {
        // Process-specific config
        #[arg(long)]
        pid: Option<u32>,
        
        #[arg(long)]
        name: Option<String>,
        
        #[arg(long)]
        cpu_threshold: Option<f32>,
        
        #[arg(long)]
        memory_threshold: Option<u64>,
        
        // Analyzer flags
        #[arg(long)]
        raw: bool,
        
        #[arg(long)]
        extract: Option<String>,
        
        #[arg(long)]
        filter: Option<String>,
        
        #[arg(long)]
        merge: bool,
        
        #[arg(long)]
        count: bool,
        
        #[arg(long)]
        store: bool,
        
        // Output options
        #[arg(long)]
        output: Option<OutputMode>,
    },
    
    /// Start server mode for frontend integration
    Server {
        // Server config
        #[arg(long, default_value = "0.0.0.0:8080")]
        bind: String,
        
        #[arg(long)]
        cors: bool,
        
        #[arg(long)]
        static_dir: Option<String>,
        
        // Default runners to start
        #[arg(long)]
        enable_ssl: bool,
        
        #[arg(long)]
        enable_process: bool,
        
        #[arg(long)]
        ssl_port: Option<u16>,
        
        #[arg(long)]
        process_filter: Option<String>,
        
        // Storage config
        #[arg(long, default_value = "50000")]
        max_events: usize,
    },
    
    /// Dynamic configuration and runner management  
    Dyn {
        #[command(subcommand)]
        action: DynCommands,
    },
}

#[derive(Subcommand)]
pub enum DynCommands {
    /// List active runners
    List,
    
    /// Start a runner
    Start {
        #[arg(value_enum)]
        runner_type: RunnerType,
        
        #[arg(long)]
        id: String,
        
        #[arg(long)]
        config: Option<String>, // JSON config
    },
    
    /// Stop a runner
    Stop {
        #[arg(long)]
        id: String,
    },
    
    /// Get runner status
    Status {
        #[arg(long)]
        id: Option<String>, // If None, show all
    },
    
    /// Configure storage
    Storage {
        #[arg(long)]
        max_events: Option<usize>,
        
        #[arg(long)]
        clear: bool,
    },
}

#[derive(ValueEnum, Clone)]
pub enum RunnerType {
    Ssl,
    Process,
}

#[derive(ValueEnum, Clone)]
pub enum MergeStrategy {
    TimeOrdered,
    RoundRobin,
    Priority,
    Immediate,
}

#[derive(ValueEnum, Clone)]
pub enum OutputMode {
    Stdout,
    Json,
    Pretty,
    File,
}
```

## Implementation Examples

### 1. Observe Mode (Combined Monitoring)
```rust
// agent-tracer observe --merge --store --extract "host,pid,cpu"
pub async fn run_observe_command(args: ObserveArgs) -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::shared();
    
    let orchestrator = RunnerOrchestrator::new(storage.clone())
        .add_runner(Box::new(
            SslRunner::new()
                .with_id("observe-ssl".to_string())
                .port(args.ssl_port.unwrap_or(443))
                .interface(args.ssl_interface.unwrap_or_else(|| "any".to_string()))
                .add_analyzer(Box::new(ExtractAnalyzer::new_if_specified(args.extract.clone())))
                .add_analyzer(Box::new(FilterAnalyzer::new_if_specified(args.filter.clone())))
                .add_analyzer(Box::new(MergeAnalyzer::new_if(args.merge)))
                .add_analyzer(Box::new(CountAnalyzer::new_if(args.count)))
                .add_analyzer(Box::new(StorageAnalyzer::new_if(args.store, storage.clone())))
        ))
        .add_runner(Box::new(
            ProcessRunner::new()
                .with_id("observe-process".to_string())
                .filter(args.process_filter.unwrap_or_else(|| "*".to_string()))
                .add_analyzer(Box::new(ExtractAnalyzer::new_if_specified(args.extract)))
                .add_analyzer(Box::new(FilterAnalyzer::new_if_specified(args.filter)))
                .add_analyzer(Box::new(MergeAnalyzer::new_if(args.merge)))
                .add_analyzer(Box::new(CountAnalyzer::new_if(args.count)))
                .add_analyzer(Box::new(StorageAnalyzer::new_if(args.store, storage.clone())))
        ));
    
    // Configure merge strategy
    orchestrator.set_merge_strategy(args.merge_strategy);
    
    // Start all runners
    orchestrator.start_all().await?;
    
    // Handle output
    match args.output {
        Some(OutputMode::File) => {
            let merged_stream = orchestrator.get_merged_stream().await?;
            OutputHandler::new(OutputMode::File("observe.json".to_string()))
                .handle(merged_stream).await?;
        },
        _ => {
            let merged_stream = orchestrator.get_merged_stream().await?;
            OutputHandler::new(args.output.unwrap_or(OutputMode::Pretty))
                .handle(merged_stream).await?;
        }
    }
    
    Ok(())
}
```

### 2. API Mode (SSL Only)
```rust
// agent-tracer api --port 443 --filter "tls_version:1.3" --merge --store
pub async fn run_api_command(args: ApiArgs) -> Result<(), Box<dyn std::error::Error>> {
    let storage = if args.store { 
        Some(InMemoryStorage::shared()) 
    } else { 
        None 
    };
    
    let mut runner = SslRunner::new()
        .with_id("api-ssl".to_string())
        .port(args.port.unwrap_or(443))
        .interface(args.interface.unwrap_or_else(|| "any".to_string()));
    
    // Add analyzers based on flags
    if args.raw {
        runner = runner.add_analyzer(Box::new(RawAnalyzer::new()));
    }
    
    if let Some(extract_fields) = args.extract {
        runner = runner.add_analyzer(Box::new(ExtractAnalyzer::new(
            extract_fields.split(',').map(|s| s.to_string()).collect()
        )));
    }
    
    if let Some(filter_expr) = args.filter {
        runner = runner.add_analyzer(Box::new(FilterAnalyzer::new(filter_expr)));
    }
    
    if args.merge {
        runner = runner.add_analyzer(Box::new(MergeAnalyzer::new()));
    }
    
    if args.count {
        runner = runner.add_analyzer(Box::new(CountAnalyzer::new()));
    }
    
    if let Some(storage) = storage {
        runner = runner.add_analyzer(Box::new(StorageAnalyzer::new(storage)));
    }
    
    // Run and handle output
    let stream = runner.run().await?;
    OutputHandler::new(args.output.unwrap_or(OutputMode::Pretty))
        .handle(stream).await?;
    
    Ok(())
}
```

### 3. Process Mode (System Only)
```rust
// agent-tracer process --name "python" --cpu-threshold 80.0 --extract "pid,cpu,memory"
pub async fn run_process_command(args: ProcessArgs) -> Result<(), Box<dyn std::error::Error>> {
    let storage = if args.store { 
        Some(InMemoryStorage::shared()) 
    } else { 
        None 
    };
    
    let mut runner = ProcessRunner::new()
        .with_id("process-monitor".to_string());
    
    // Configure process filtering
    if let Some(pid) = args.pid {
        runner = runner.pid(pid);
    }
    
    if let Some(name) = args.name {
        runner = runner.name_filter(name);
    }
    
    if let Some(cpu_threshold) = args.cpu_threshold {
        runner = runner.cpu_threshold(cpu_threshold);
    }
    
    if let Some(memory_threshold) = args.memory_threshold {
        runner = runner.memory_threshold(memory_threshold);
    }
    
    // Add analyzers based on flags
    if args.raw {
        runner = runner.add_analyzer(Box::new(RawAnalyzer::new()));
    }
    
    if let Some(extract_fields) = args.extract {
        runner = runner.add_analyzer(Box::new(ExtractAnalyzer::new(
            extract_fields.split(',').map(|s| s.to_string()).collect()
        )));
    }
    
    if let Some(filter_expr) = args.filter {
        runner = runner.add_analyzer(Box::new(FilterAnalyzer::new(filter_expr)));
    }
    
    if args.merge {
        runner = runner.add_analyzer(Box::new(MergeAnalyzer::new()));
    }
    
    if args.count {
        runner = runner.add_analyzer(Box::new(CountAnalyzer::new()));
    }
    
    if let Some(storage) = storage {
        runner = runner.add_analyzer(Box::new(StorageAnalyzer::new(storage)));
    }
    
    // Run and handle output
    let stream = runner.run().await?;
    OutputHandler::new(args.output.unwrap_or(OutputMode::Pretty))
        .handle(stream).await?;
    
    Ok(())
}
```

### 4. Server Mode
```rust
// agent-tracer server --bind 0.0.0.0:8080 --enable-ssl --enable-process --cors
pub async fn run_server_command(args: ServerArgs) -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new(args.max_events);
    let mut orchestrator = RunnerOrchestrator::new(Arc::new(storage.clone()));
    
    // Add default runners if enabled
    if args.enable_ssl {
        orchestrator = orchestrator.add_runner(Box::new(
            SslRunner::new()
                .with_id("server-ssl".to_string())
                .port(args.ssl_port.unwrap_or(443))
                .add_analyzer(Box::new(StorageAnalyzer::new(Arc::new(storage.clone()))))
        ));
    }
    
    if args.enable_process {
        orchestrator = orchestrator.add_runner(Box::new(
            ProcessRunner::new()
                .with_id("server-process".to_string())
                .filter(args.process_filter.unwrap_or_else(|| "*".to_string()))
                .add_analyzer(Box::new(StorageAnalyzer::new(Arc::new(storage.clone()))))
        ));
    }
    
    // Start enabled runners
    if args.enable_ssl || args.enable_process {
        orchestrator.start_all().await?;
    }
    
    // Configure and start server
    let server = ObservabilityServer::new(
        Arc::new(Mutex::new(orchestrator)),
        Arc::new(storage),
        args.bind
    )
    .enable_cors(args.cors)
    .static_directory(args.static_dir);
    
    println!("Starting server on {}", args.bind);
    server.start().await?;
    
    Ok(())
}
```

### 5. Dynamic Commands
```rust
// agent-tracer dyn start ssl --id custom-ssl --config '{"port": 8443}'
pub async fn run_dyn_command(args: DynCommands) -> Result<(), Box<dyn std::error::Error>> {
    let client = ApiClient::new("http://localhost:8080")?; // Connect to running server
    
    match args {
        DynCommands::List => {
            let runners = client.list_runners().await?;
            println!("Active runners:");
            for runner in runners {
                println!("  {} ({}): {:?}", runner.id, runner.name, runner.status);
            }
        },
        
        DynCommands::Start { runner_type, id, config } => {
            let runner_config = config.unwrap_or_else(|| "{}".to_string());
            match runner_type {
                RunnerType::Ssl => {
                    client.start_ssl_runner(&id, &runner_config).await?;
                    println!("Started SSL runner: {}", id);
                },
                RunnerType::Process => {
                    client.start_process_runner(&id, &runner_config).await?;
                    println!("Started process runner: {}", id);
                },
            }
        },
        
        DynCommands::Stop { id } => {
            client.stop_runner(&id).await?;
            println!("Stopped runner: {}", id);
        },
        
        DynCommands::Status { id } => {
            if let Some(runner_id) = id {
                let status = client.get_runner_status(&runner_id).await?;
                println!("Runner {}: {:?}", runner_id, status);
            } else {
                let all_status = client.get_all_runner_status().await?;
                for (id, status) in all_status {
                    println!("Runner {}: {:?}", id, status);
                }
            }
        },
        
        DynCommands::Storage { max_events, clear } => {
            if let Some(max) = max_events {
                client.configure_storage_max_events(max).await?;
                println!("Set max events to: {}", max);
            }
            if clear {
                client.clear_storage().await?;
                println!("Storage cleared");
            }
        },
    }
    
    Ok(())
}
```

## Usage Examples

```bash
# Combined monitoring with time-ordered merging
agent-tracer observe --merge --store --extract "host,pid,cpu" --merge-strategy time-ordered

# SSL-only monitoring with filtering  
agent-tracer api --port 443 --filter "tls_version:1.3" --merge --store

# Process monitoring with thresholds
agent-tracer process --name "python" --cpu-threshold 80.0 --extract "pid,cpu,memory"

# Start server with both runners enabled
agent-tracer server --bind 0.0.0.0:8080 --enable-ssl --enable-process --cors

# Dynamically manage runners
agent-tracer dyn list
agent-tracer dyn start ssl --id custom-ssl --config '{"port": 8443, "interface": "eth0"}'
agent-tracer dyn start process --id high-cpu --config '{"cpu_threshold": 90.0}'
agent-tracer dyn stop custom-ssl
agent-tracer dyn status
agent-tracer dyn storage --max-events 100000 --clear
```

This redesign provides clean, focused subcommands where each has a clear purpose and intuitive configuration options!
