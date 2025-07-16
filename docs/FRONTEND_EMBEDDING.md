# Frontend Embedding Guide

This guide explains how to embed the Next.js frontend build artifacts into the Rust collector binary to create a self-contained web server for AgentSight.

## Overview

The solution embeds the frontend build artifacts directly into the Rust binary using `include_bytes!` and serves them through a simple HTTP server. This creates a single binary that can serve both the web interface and collect/stream observability data.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                Rust Collector                       │
│  ┌─────────────────┐  ┌─────────────────────────┐   │
│  │   HTTP Server   │  │    Data Collection      │   │
│  │  (Static Files) │  │   (SSL/Process/Agent)   │   │
│  └─────────────────┘  └─────────────────────────┘   │
│           │                        │                │
│  ┌─────────────────┐  ┌─────────────────────────┐   │
│  │ Embedded Assets │  │    Event Stream API     │   │
│  │ (HTML/JS/CSS)   │  │   (SSE/WebSocket)       │   │
│  └─────────────────┘  └─────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

## Implementation Steps

### 1. Update Cargo.toml

Add the necessary dependencies for HTTP server functionality and directory embedding:

```toml
[dependencies]
# ... existing dependencies ...
tokio = { version = "1.0", features = ["full"] }
hyper = { version = "1.0", features = ["full"] }
hyper-util = { version = "0.1", features = ["full"] }
http-body-util = "0.1"
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "cors"] }
mime_guess = "2.0"
rust-embed = { version = "8.7", features = ["debug-embed", "compression"] }
```

### 2. Build Frontend Assets

```bash
# Build the frontend
cd frontend
npm run build

# The build output will be in .next/static/ and .next/server/
```

### 3. Create Asset Embedding Module

Create `collector/src/frontend_assets.rs`:

```rust
use rust_embed::RustEmbed;
use std::borrow::Cow;
use mime_guess::from_path;

#[derive(RustEmbed)]
#[folder = "../frontend/.next/static/"]
#[prefix = "_next/static/"]
pub struct StaticAssets;

#[derive(RustEmbed)]
#[folder = "../frontend/.next/server/app/"]
#[prefix = "pages/"]
pub struct PageAssets;

pub struct FrontendAssets;

impl FrontendAssets {
    pub fn new() -> Self {
        Self
    }
    
    /// Get static asset (CSS, JS, images, etc.)
    pub fn get_static(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        StaticAssets::get(path)
    }
    
    /// Get page asset (HTML files)
    pub fn get_page(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        PageAssets::get(path)
    }
    
    /// Get any asset by path
    pub fn get(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        // Try static assets first
        if let Some(content) = self.get_static(path) {
            return Some(content);
        }
        
        // Handle root path
        if path == "/" || path == "/index.html" {
            return self.get_page("pages/index.html");
        }
        
        // Try page assets
        if let Some(content) = self.get_page(path) {
            return Some(content);
        }
        
        None
    }
    
    /// Get MIME type for a file path
    pub fn get_content_type(&self, path: &str) -> &'static str {
        from_path(path).first_or_octet_stream().as_ref()
    }
    
    /// List all available static assets
    pub fn list_static_assets(&self) -> Vec<String> {
        StaticAssets::iter().map(|s| s.to_string()).collect()
    }
    
    /// List all available page assets
    pub fn list_page_assets(&self) -> Vec<String> {
        PageAssets::iter().map(|s| s.to_string()).collect()
    }
}
```

### 4. Create HTTP Server Module

Create `collector/src/web_server.rs`:

```rust
use crate::frontend_assets::FrontendAssets;
use crate::framework::core::Event;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Bytes, Request, Response, Result, Method, StatusCode};
use hyper_util::rt::TokioIo;
use http_body_util::Full;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

pub struct WebServer {
    assets: Arc<FrontendAssets>,
    event_sender: broadcast::Sender<Event>,
}

impl WebServer {
    pub fn new(event_sender: broadcast::Sender<Event>) -> Self {
        Self {
            assets: Arc::new(FrontendAssets::new()),
            event_sender,
        }
    }
    
    pub async fn start(&self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(addr).await?;
        println!("Frontend server running on http://{}", addr);
        
        loop {
            let (stream, _) = listener.accept().await?;
            let assets = Arc::clone(&self.assets);
            let event_sender = self.event_sender.clone();
            
            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                let service = service_fn(move |req| {
                    handle_request(req, assets.clone(), event_sender.clone())
                });
                
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service)
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    assets: Arc<FrontendAssets>,
    event_sender: broadcast::Sender<Event>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let path = req.uri().path();
    
    match (req.method(), path) {
        // Serve static assets
        (&Method::GET, "/") | (&Method::GET, "/index.html") => {
            serve_asset(assets, "/").await
        }
        (&Method::GET, path) if path.starts_with("/_next/") => {
            serve_asset(assets, path).await
        }
        
        // API endpoints
        (&Method::GET, "/api/events") => {
            serve_events_api(event_sender).await
        }
        
        // 404 for everything else
        _ => {
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("Not Found")))
                .unwrap())
        }
    }
}

async fn serve_asset(
    assets: Arc<FrontendAssets>,
    path: &str,
) -> Result<Response<Full<Bytes>>, Infallible> {
    if let Some(content) = assets.get(path) {
        let content_type = assets.get_content_type(path);
        Ok(Response::builder()
            .header("Content-Type", content_type)
            .body(Full::new(Bytes::from(content.as_ref())))
            .unwrap())
    } else {
        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("Asset not found")))
            .unwrap())
    }
}

async fn serve_events_api(
    _event_sender: broadcast::Sender<Event>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    // Return sample events as JSON for now
    let events = serde_json::json!([
        {
            "timestamp": 1234567890,
            "source": "ssl",
            "pid": 1234,
            "comm": "python",
            "data": {"message": "SSL handshake completed"}
        },
        {
            "timestamp": 1234567891,
            "source": "process",
            "pid": 1235,
            "comm": "node",
            "data": {"message": "Process started"}
        }
    ]);
    
    Ok(Response::builder()
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(events.to_string())))
        .unwrap())
}
```

### 5. Add Web Server Command

Update `collector/src/main.rs` to add a new `serve` command:

```rust
// Add to imports
use crate::web_server::WebServer;
use tokio::sync::broadcast;

// Add new module
mod web_server;
mod frontend_assets;

// Add to Commands enum
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    
    /// Start web server with embedded frontend
    Serve {
        /// Port to bind to
        #[arg(short, long, default_value = "8080")]
        port: u16,
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Also start data collection
        #[arg(long)]
        collect: bool,
    },
}

// Add to main function
Commands::Serve { port, host, collect } => {
    let addr = format!("{}:{}", host, port).parse().unwrap();
    let (event_sender, _) = broadcast::channel(1000);
    
    let web_server = WebServer::new(event_sender.clone());
    
    if collect {
        // Start data collection in background
        let binary_extractor = BinaryExtractor::new().await.map_err(convert_runner_error)?;
        let event_sender_clone = event_sender.clone();
        
        tokio::spawn(async move {
            // Start SSL monitoring and forward events
            let mut ssl_runner = SslRunner::from_binary_extractor(binary_extractor.get_sslsniff_path())
                .add_analyzer(Box::new(OutputAnalyzer::new()));
            
            if let Ok(mut stream) = ssl_runner.run().await {
                while let Some(event) = stream.next().await {
                    let _ = event_sender_clone.send(event);
                }
            }
        });
    }
    
    web_server.start(addr).await.map_err(convert_runner_error)?;
}
```

### 6. Build Script for Asset Management

Create `collector/build.rs`:

```rust
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let frontend_dir = Path::new("../frontend");
    let next_dir = frontend_dir.join(".next");
    
    // Auto-build frontend if needed
    if !next_dir.exists() {
        println!("cargo:warning=Frontend not built. Building now...");
        build_frontend();
    }
    
    // Tell cargo to rebuild if frontend files change
    println!("cargo:rerun-if-changed=../frontend/.next");
    println!("cargo:rerun-if-changed=../frontend/src");
    println!("cargo:rerun-if-changed=../frontend/package.json");
    
    // Verify required directories exist
    verify_build_structure();
}

fn build_frontend() {
    let output = Command::new("npm")
        .args(&["run", "build"])
        .current_dir("../frontend")
        .output()
        .expect("Failed to execute npm build");
    
    if !output.status.success() {
        panic!("Frontend build failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    println!("Frontend build completed successfully");
}

fn verify_build_structure() {
    let required_dirs = [
        "../frontend/.next/static",
        "../frontend/.next/server/app",
    ];
    
    for dir in &required_dirs {
        if !Path::new(dir).exists() {
            panic!("Required build directory not found: {}", dir);
        }
    }
}
```

## Usage

### 1. Build the Collector with Frontend

The build script will automatically build the frontend if needed:

```bash
cd collector
cargo build --release
```

Or manually build the frontend first:

```bash
cd frontend
npm run build
cd ../collector
cargo build --release
```

### 2. Start the Web Server

```bash
# Serve frontend only
sudo ./target/release/collector serve --port 8080

# Serve frontend with data collection
sudo ./target/release/collector serve --port 8080 --collect
```

### 3. Access the Frontend

Open your browser to `http://localhost:8080`

## API Endpoints

The embedded server provides these endpoints:

- `GET /` - Serves the main frontend application
- `GET /_next/static/*` - Serves static assets (JS, CSS, etc.)
- `GET /api/events` - Returns events as JSON

## Advanced Features

### Multiple Asset Directories

You can embed multiple directories using additional `RustEmbed` structs:

```rust
#[derive(RustEmbed)]
#[folder = "../frontend/.next/static/"]
#[prefix = "_next/static/"]
pub struct StaticAssets;

#[derive(RustEmbed)]
#[folder = "../frontend/.next/server/app/"]
#[prefix = "pages/"]
pub struct PageAssets;

#[derive(RustEmbed)]
#[folder = "../frontend/public/"]
#[prefix = "public/"]
pub struct PublicAssets;
```

### Asset Filtering and Compression

Use rust-embed features to filter and compress assets:

```rust
#[derive(RustEmbed)]
#[folder = "../frontend/.next/static/"]
#[prefix = "_next/static/"]
#[include = "*.js"]
#[include = "*.css"]
#[exclude = "*.map"]
// Automatically compresses assets
pub struct StaticAssets;
```

### Development Mode

rust-embed automatically loads files from disk in debug mode unless `debug-embed` feature is enabled:

```toml
[dependencies]
# Loads from filesystem in debug mode
rust-embed = "8.7"

# Always embeds in binary (even in debug)
# rust-embed = { version = "8.7", features = ["debug-embed"] }
```

### Asset Listing and Debugging

List all embedded assets for debugging:

```rust
pub fn list_all_assets() {
    println!("Static Assets:");
    for path in StaticAssets::iter() {
        println!("  {}", path);
    }
    
    println!("Page Assets:");
    for path in PageAssets::iter() {
        println!("  {}", path);
    }
}
```

## Benefits

1. **Single Binary**: No need to deploy frontend separately
2. **Simplified Deployment**: Just copy one binary
3. **Integrated Data Flow**: Direct access to collector data streams
4. **Minimal Dependencies**: No external web server required

## Limitations

1. **Binary Size**: Increases collector binary size
2. **Static Assets**: No dynamic frontend updates without rebuild
3. **Limited HTTP Features**: Basic HTTP server functionality

## Next Steps

1. Implement Server-Sent Events for real-time data streaming
2. Add WebSocket support for bidirectional communication
3. Add authentication and security features
4. Implement proper error handling and logging
5. Add compression for static assets

This approach provides a simple, self-contained solution for serving the frontend directly from the Rust collector binary.