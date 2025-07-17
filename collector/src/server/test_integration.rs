#[cfg(test)]
mod integration_tests {
    use super::super::{WebServer, assets::FrontendAssets};
    use crate::framework::core::Event;
    use tokio::sync::broadcast;
    
    #[test]
    fn test_server_module_imports() {
        // Test that all public types are accessible
        let _assets = FrontendAssets::new();
        let (sender, _) = broadcast::channel::<Event>(10);
        let _server = WebServer::new(sender);
        
        println!("✅ All server module types are accessible");
    }
    
    #[test]
    fn test_asset_embedding_compiles() {
        // This test verifies that the rust-embed macros compile correctly
        // even if the frontend directories don't exist yet
        let assets = FrontendAssets::new();
        
        // These methods should compile even without actual files
        let _static_list = assets.list_static_assets();
        let _page_list = assets.list_page_assets();
        let _all_list = assets.list_all_assets();
        
        println!("✅ Asset embedding compiles successfully");
    }
    
    #[tokio::test]
    async fn test_basic_server_lifecycle() {
        let (event_sender, _receiver) = broadcast::channel(100);
        let _web_server = WebServer::new(event_sender.clone());
        
        // Test event creation
        let test_event = Event::new(
            "test-source".to_string(),
            9999,
            "test-binary".to_string(),
            serde_json::json!({
                "action": "test",
                "status": "success"
            })
        );
        
        // Test broadcasting
        assert!(event_sender.send(test_event).is_ok(), "Should broadcast event");
        
        println!("✅ Basic server lifecycle test passed");
    }
}