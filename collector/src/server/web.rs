use crate::framework::core::Event;
use crate::server::assets::FrontendAssets;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Bytes, Request, Response, Method, StatusCode};
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
    log_file: Option<String>,
}

impl WebServer {
    pub fn new(event_sender: broadcast::Sender<Event>, log_file: Option<&str>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let assets = FrontendAssets::new()?;
        Ok(Self {
            assets: Arc::new(assets),
            event_sender,
            log_file: log_file.map(|s| s.to_string()),
        })
    }
    
    pub async fn start(&self, addr: SocketAddr) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(addr).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        log::info!("🚀 Frontend server running on http://{}", addr);
        
        // List embedded assets for debugging
        let all_assets = self.assets.list_all_assets();
        log::info!("📦 Embedded {} assets from frontend/dist:", all_assets.len());
        for asset in all_assets.iter().take(10) {
            log::info!("   - {}", asset);
        }
        if all_assets.len() > 10 {
            log::info!("   ... and {} more", all_assets.len() - 10);
        }
        
        loop {
            let (stream, _) = listener.accept().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            let assets = Arc::clone(&self.assets);
            let event_sender = self.event_sender.clone();
            let log_file = self.log_file.clone();
            
            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                let service = service_fn(move |req| {
                    handle_request(req, assets.clone(), event_sender.clone(), log_file.clone())
                });
                
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service)
                    .await
                {
                    log::error!("❌ Error serving connection: {:?}", err);
                }
            });
        }
    }
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    assets: Arc<FrontendAssets>,
    event_sender: broadcast::Sender<Event>,
    log_file: Option<String>,
) -> std::result::Result<Response<Full<Bytes>>, Infallible> {
    let path = req.uri().path();
    
    log::info!("📨 {} {}", req.method(), path);
    
    match (req.method(), path) {
        // API endpoints first
        (&Method::GET, "/api/events") => {
            serve_events_api(event_sender, log_file).await
        }
        (&Method::GET, "/api/assets") => {
            serve_assets_list(assets).await
        }
        
        // Serve static assets (catch-all for GET requests)
        (&Method::GET, _) => {
            serve_asset(assets, path).await
        }
        
        // 404 for non-GET methods
        _ => {
            log::info!("❌ 404 Not Found: {} {}", req.method(), path);
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
                .body(Full::new(Bytes::from("Not Found")))
                .unwrap())
        }
    }
}

async fn serve_asset(
    assets: Arc<FrontendAssets>,
    path: &str,
) -> std::result::Result<Response<Full<Bytes>>, Infallible> {
    if let Some(content) = assets.get(path) {
        let content_type = assets.get_content_type(path);
        log::info!("✅ Serving asset: {} ({})", path, content_type);
        Ok(Response::builder()
            .header("Content-Type", content_type)
            .header("Cache-Control", "public, max-age=31536000")
            .body(Full::new(Bytes::from(content.to_vec())))
            .unwrap())
    } else {
        log::info!("❌ Asset not found: {}", path);
        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "text/plain")
            .body(Full::new(Bytes::from("Asset not found")))
            .unwrap())
    }
}

async fn serve_events_api(
    _event_sender: broadcast::Sender<Event>,
    log_file: Option<String>,
) -> std::result::Result<Response<Full<Bytes>>, Infallible> {
    // If log file is specified, read and return its contents
    if let Some(log_path) = log_file {
        match tokio::fs::read_to_string(&log_path).await {
            Ok(content) => {
                log::info!("📊 Serving log file: {} ({} bytes)", log_path, content.len());
                Ok(Response::builder()
                    .header("Content-Type", "text/plain")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Full::new(Bytes::from(content)))
                    .unwrap())
            }
            Err(e) => {
                log::error!("❌ Failed to read log file {}: {}", log_path, e);
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "text/plain")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Full::new(Bytes::from(format!("Failed to read log file: {}", e))))
                    .unwrap())
            }
        }
    } else {
        // Return sample events as JSON for now
        let events = serde_json::json!([
            {
                "timestamp": 1234567890,
                "source": "ssl",
                "pid": 1234,
                "comm": "python",
                "data": {"message": "SSL handshake completed", "url": "https://api.example.com"}
            },
            {
                "timestamp": 1234567891,
                "source": "process",
                "pid": 1235,
                "comm": "node",
                "data": {"message": "Process started", "args": ["node", "server.js"]}
            },
            {
                "timestamp": 1234567892,
                "source": "ssl",
                "pid": 1234,
                "comm": "python",
                "data": {"message": "HTTP request", "method": "GET", "url": "https://api.example.com/users"}
            }
        ]);
        
        log::info!("📊 Serving sample events API");
        Ok(Response::builder()
            .header("Content-Type", "application/json")
            .header("Access-Control-Allow-Origin", "*")
            .body(Full::new(Bytes::from(events.to_string())))
            .unwrap())
    }
}

async fn serve_assets_list(
    assets: Arc<FrontendAssets>,
) -> std::result::Result<Response<Full<Bytes>>, Infallible> {
    let all_assets = assets.list_all_assets();
    let response = serde_json::json!({
        "assets": all_assets,
        "total_count": all_assets.len()
    });
    
    log::info!("📋 Serving assets list ({} assets)", all_assets.len());
    Ok(Response::builder()
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(response.to_string())))
        .unwrap())
}