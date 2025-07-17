use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::path::PathBuf;
use std::fs;
use mime_guess::from_path;

#[derive(RustEmbed)]
#[folder = "../frontend/dist/"]
pub struct FrontendDist;

pub struct FrontendAssets {
    temp_dir: PathBuf,
}

impl FrontendAssets {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let temp_dir = std::env::temp_dir().join(format!("agentsight-frontend-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir)?;
        
        // Extract all embedded assets to temp directory
        for file_path in FrontendDist::iter() {
            if let Some(content) = FrontendDist::get(&file_path) {
                let full_path = temp_dir.join(&*file_path);
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&full_path, &content.data)?;
            }
        }
        
        log::info!("📁 Extracted frontend assets to: {}", temp_dir.display());
        Ok(Self { temp_dir })
    }
    
    /// Get any asset by path from the extracted temp directory
    pub fn get(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        // Handle root path
        let file_path = if path == "/" || path == "/index.html" {
            self.temp_dir.join("index.html")
        } else {
            // Remove leading slash for file lookup
            let normalized_path = path.strip_prefix('/').unwrap_or(path);
            self.temp_dir.join(normalized_path)
        };
        
        // Try to read from temp directory
        if let Ok(content) = fs::read(&file_path) {
            Some(Cow::Owned(content))
        } else {
            None
        }
    }
    
    
    /// Get MIME type for a file path
    pub fn get_content_type(&self, path: &str) -> String {
        // Handle root path - should serve as HTML
        let file_path = if path == "/" || path == "/index.html" {
            "index.html"
        } else {
            // Remove leading slash for proper MIME detection
            path.strip_prefix('/').unwrap_or(path)
        };
        
        from_path(file_path).first_or_octet_stream().to_string()
    }
    
    /// List all available assets from the embedded dist
    pub fn list_all_assets(&self) -> Vec<String> {
        FrontendDist::iter().map(|s| s.to_string()).collect()
    }
}

impl Drop for FrontendAssets {
    fn drop(&mut self) {
        if self.temp_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&self.temp_dir) {
                log::warn!("Failed to cleanup temp directory {}: {}", self.temp_dir.display(), e);
            } else {
                log::info!("🧹 Cleaned up temp directory: {}", self.temp_dir.display());
            }
        }
    }
}