use crate::framework::core::Event;
use crate::server::assets::FrontendAssets;
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
        println!("🚀 Frontend server running on http://{}", addr);
        
        // List embedded assets for debugging
        let all_assets = self.assets.list_all_assets();
        println!("📦 Embedded {} assets:", all_assets.len());
        for asset in all_assets.iter().take(10) {
            println!("   - {}", asset);
        }
        if all_assets.len() > 10 {
            println!("   ... and {} more", all_assets.len() - 10);
        }
        
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
                    eprintln!("❌ Error serving connection: {:?}", err);
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
    
    println!("📨 {} {}", req.method(), path);
    
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
        (&Method::GET, "/api/assets") => {
            serve_assets_list(assets).await
        }
        
        // 404 for everything else
        _ => {
            println!("❌ 404 Not Found: {}", path);
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
) -> Result<Response<Full<Bytes>>, Infallible> {
    if let Some(content) = assets.get(path) {
        let content_type = assets.get_content_type(path);
        println!("✅ Serving asset: {} ({})", path, content_type);
        Ok(Response::builder()
            .header("Content-Type", content_type)
            .header("Cache-Control", "public, max-age=31536000")
            .body(Full::new(Bytes::from(content.as_ref())))
            .unwrap())
    } else {
        println!("❌ Asset not found: {}", path);
        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "text/plain")
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
    
    println!("📊 Serving events API");
    Ok(Response::builder()
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(events.to_string())))
        .unwrap())
}

async fn serve_assets_list(
    assets: Arc<FrontendAssets>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let all_assets = assets.list_all_assets();
    let response = serde_json::json!({
        "static_assets": assets.list_static_assets(),
        "page_assets": assets.list_page_assets(),
        "total_count": all_assets.len()
    });
    
    println!("📋 Serving assets list ({} assets)", all_assets.len());
    Ok(Response::builder()
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(response.to_string())))
        .unwrap())
}