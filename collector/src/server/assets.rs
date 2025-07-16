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
        StaticAssets::get(path)
    }
    
    /// Get page asset (HTML files)
    pub fn get_page(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        PageAssets::get(path)
    }
    
    /// Get any asset by path
    pub fn get(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        // Handle root path
        if path == "/" || path == "/index.html" {
            return self.get_page("index.html");
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
    
    /// List all available assets for debugging
    pub fn list_all_assets(&self) -> Vec<String> {
        let mut assets = Vec::new();
        assets.extend(self.list_static_assets());
        assets.extend(self.list_page_assets());
        assets
    }
}