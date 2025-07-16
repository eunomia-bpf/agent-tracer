#[cfg(test)]
mod tests {
    use super::super::assets::FrontendAssets;
    
    #[test]
    fn test_frontend_assets_creation() {
        let assets = FrontendAssets::new();
        
        // Test that we can create assets without panic
        assert!(true, "FrontendAssets created successfully");
        
        // List all assets
        let all_assets = assets.list_all_assets();
        println!("Total embedded assets: {}", all_assets.len());
        
        // List static assets
        let static_assets = assets.list_static_assets();
        println!("Static assets: {}", static_assets.len());
        for asset in static_assets.iter().take(5) {
            println!("  - {}", asset);
        }
        
        // List page assets
        let page_assets = assets.list_page_assets();
        println!("Page assets: {}", page_assets.len());
        for asset in page_assets.iter().take(5) {
            println!("  - {}", asset);
        }
    }
    
    #[test]
    fn test_get_root_page() {
        let assets = FrontendAssets::new();
        
        // Test getting root path
        let root_content = assets.get("/");
        assert!(root_content.is_some(), "Root page should be available");
        
        // Test getting index.html
        let index_content = assets.get("/index.html");
        assert!(index_content.is_some(), "index.html should be available");
        
        // Both should return the same content
        if let (Some(root), Some(index)) = (root_content, index_content) {
            assert_eq!(root.len(), index.len(), "Root and index.html should be the same");
        }
    }
    
    #[test]
    fn test_content_types() {
        let assets = FrontendAssets::new();
        
        // Test various file types
        assert_eq!(assets.get_content_type("/index.html"), "text/html");
        assert_eq!(assets.get_content_type("/script.js"), "application/javascript");
        assert_eq!(assets.get_content_type("/style.css"), "text/css");
        assert_eq!(assets.get_content_type("/data.json"), "application/json");
        assert_eq!(assets.get_content_type("/image.png"), "image/png");
        assert_eq!(assets.get_content_type("/unknown.xyz"), "application/octet-stream");
    }
    
    #[test]
    fn test_static_asset_retrieval() {
        let assets = FrontendAssets::new();
        
        // Try to get some static assets if they exist
        let static_list = assets.list_static_assets();
        if !static_list.is_empty() {
            let first_static = &static_list[0];
            let content = assets.get_static(first_static);
            assert!(content.is_some(), "Should be able to retrieve first static asset");
            
            // Verify the content is not empty
            if let Some(data) = content {
                assert!(!data.is_empty(), "Static asset should have content");
                println!("First static asset '{}' has {} bytes", first_static, data.len());
            }
        }
    }
    
    #[test]
    fn test_page_asset_retrieval() {
        let assets = FrontendAssets::new();
        
        // Try to get page assets if they exist
        let page_list = assets.list_page_assets();
        if !page_list.is_empty() {
            let first_page = &page_list[0];
            let content = assets.get_page(first_page);
            assert!(content.is_some(), "Should be able to retrieve first page asset");
            
            // Verify the content is not empty
            if let Some(data) = content {
                assert!(!data.is_empty(), "Page asset should have content");
                println!("First page asset '{}' has {} bytes", first_page, data.len());
            }
        }
    }
    
    #[test]
    fn test_nonexistent_asset() {
        let assets = FrontendAssets::new();
        
        // Test getting a non-existent asset
        let missing = assets.get("/this/does/not/exist.txt");
        assert!(missing.is_none(), "Non-existent asset should return None");
        
        let missing_static = assets.get_static("nonexistent.js");
        assert!(missing_static.is_none(), "Non-existent static asset should return None");
        
        let missing_page = assets.get_page("missing.html");
        assert!(missing_page.is_none(), "Non-existent page asset should return None");
    }
}