use rust_embed::RustEmbed;
use std::borrow::Cow;
use mime_guess::from_path;

#[derive(RustEmbed)]
#[folder = "../frontend/.next/static/"]
#[prefix = "_next/static/"]
pub struct StaticAssets;

#[derive(RustEmbed)]
#[folder = "../frontend/.next/server/app/"]
pub struct PageAssets;

pub struct FrontendAssets;

impl FrontendAssets {
    pub fn new() -> Self {
        Self
    }
    
    /// Get static asset (CSS, JS, images, etc.)
    pub fn get_static(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        StaticAssets::get(path).map(|file| file.data)
    }
    
    /// Get page asset (HTML files)
    pub fn get_page(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        PageAssets::get(path).map(|file| file.data)
    }
    
    /// Get any asset by path
    pub fn get(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        // Handle root path
        if path == "/" || path == "/index.html" {
            // Try to get from embedded page assets first
            if let Some(content) = self.get_page("index.html") {
                return Some(content);
            }
            // Fallback to a basic HTML page
            return Some(Cow::Owned(self.get_fallback_html().into_bytes()));
        }
        
        // Try static assets first (they have _next/static/ prefix)
        if let Some(content) = self.get_static(path) {
            return Some(content);
        }
        
        // Try page assets
        if let Some(content) = self.get_page(path) {
            return Some(content);
        }
        
        None
    }
    
    /// Get a fallback HTML page when embedded assets are not available
    fn get_fallback_html(&self) -> String {
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>AgentSight - Process Monitor</title>
    <style>
        body { 
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; 
            margin: 0; 
            padding: 20px; 
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            min-height: 100vh;
        }
        .container { 
            max-width: 800px; 
            margin: 0 auto; 
            text-align: center; 
            padding: 40px 20px;
        }
        .logo { 
            font-size: 3em; 
            margin-bottom: 20px; 
            font-weight: bold;
        }
        .subtitle { 
            font-size: 1.2em; 
            margin-bottom: 40px; 
            opacity: 0.9;
        }
        .api-section {
            background: rgba(255,255,255,0.1);
            padding: 20px;
            border-radius: 10px;
            margin: 20px 0;
            text-align: left;
        }
        .endpoint {
            background: rgba(0,0,0,0.2);
            padding: 10px;
            border-radius: 5px;
            font-family: monospace;
            margin: 10px 0;
        }
        a { color: #ffd700; text-decoration: none; }
        a:hover { text-decoration: underline; }
    </style>
</head>
<body>
    <div class="container">
        <div class="logo">🔍 AgentSight</div>
        <div class="subtitle">AI Agent Process Monitor</div>
        
        <div class="api-section">
            <h3>Available Endpoints</h3>
            <div class="endpoint">
                <strong>GET /api/events</strong><br>
                <a href="/api/events">View real-time events data</a>
            </div>
            <div class="endpoint">
                <strong>GET /api/assets</strong><br>
                <a href="/api/assets">List embedded assets</a>
            </div>
        </div>
        
        <div class="api-section">
            <h3>About</h3>
            <p>This embedded web server provides real-time monitoring of AI agent behavior through eBPF-based system observation.</p>
            <p><strong>Status:</strong> Server is running successfully</p>
            <p><strong>Note:</strong> Full React frontend not available - using fallback interface</p>
        </div>
    </div>
    
    <script>
        // Simple auto-refresh for events
        function refreshEvents() {
            fetch('/api/events')
                .then(response => response.json())
                .then(data => {
                    console.log('Latest events:', data);
                })
                .catch(err => console.error('Error fetching events:', err));
        }
        
        // Refresh every 5 seconds
        setInterval(refreshEvents, 5000);
        refreshEvents(); // Initial load
    </script>
</body>
</html>"#.to_string()
    }
    
    /// Get MIME type for a file path
    pub fn get_content_type(&self, path: &str) -> String {
        from_path(path).first_or_octet_stream().to_string()
    }
    
    /// List all available static assets
    pub fn list_static_assets(&self) -> Vec<String> {
        StaticAssets::iter().map(|s| s.to_string()).collect()
    }
    
    /// List all available page assets
    pub fn list_page_assets(&self) -> Vec<String> {
        PageAssets::iter().map(|s| s.to_string()).collect()
    }
    
    /// List all available assets for debugging
    pub fn list_all_assets(&self) -> Vec<String> {
        let mut assets = Vec::new();
        assets.extend(self.list_static_assets());
        assets.extend(self.list_page_assets());
        assets
    }
}