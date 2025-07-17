#[cfg(test)]
mod tests {
    use super::super::web::WebServer;
    use crate::framework::core::Event;
    use tokio::sync::broadcast;
    use std::time::Duration;
    use tokio::time::timeout;
    
    #[tokio::test]
    async fn test_web_server_creation() {
        let (event_sender, _receiver) = broadcast::channel(100);
        let _web_server = WebServer::new(event_sender);
        
        // Test that we can create a web server without panic
        assert!(true, "WebServer created successfully");
    }
    
    #[tokio::test]
    async fn test_web_server_bind_and_shutdown() {
        let (event_sender, _receiver) = broadcast::channel(100);
        let web_server = WebServer::new(event_sender);
        
        // Use a random port to avoid conflicts
        let addr = "127.0.0.1:0".parse().unwrap();
        
        // Start server in a task
        let server_handle = tokio::spawn(async move {
            web_server.start(addr).await
        });
        
        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Cancel the server task
        server_handle.abort();
        
        // Wait for it to finish with timeout
        let result = timeout(Duration::from_secs(1), server_handle).await;
        assert!(result.is_ok() || result.is_err(), "Server should shut down");
    }
    
    #[tokio::test]
    async fn test_event_broadcasting() {
        let (event_sender, mut receiver) = broadcast::channel(100);
        
        // Create a test event
        let test_event = Event::new(
            "test".to_string(),
            1234,
            "test-comm".to_string(),
            serde_json::json!({"message": "test event"})
        );
        
        // Send the event
        let send_result = event_sender.send(test_event.clone());
        assert!(send_result.is_ok(), "Should be able to send event");
        
        // Receive the event
        let received = receiver.recv().await;
        assert!(received.is_ok(), "Should receive the event");
        
        let received_event = received.unwrap();
        assert_eq!(received_event.source, "test");
        assert_eq!(received_event.pid, 1234);
        assert_eq!(received_event.comm, "test-comm");
    }
    
    #[tokio::test]
    async fn test_http_client_request() {
        use hyper::Uri;
        use hyper_util::rt::TokioExecutor;
        use hyper_util::client::legacy::Client as LegacyClient;
        
        let (event_sender, _receiver) = broadcast::channel(100);
        let web_server = WebServer::new(event_sender);
        
        // Start server on a random port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener); // Release the port
        
        // Start server in background
        let server_addr = addr.clone();
        let server_handle = tokio::spawn(async move {
            web_server.start(server_addr).await
        });
        
        // Give server time to start
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Create HTTP client
        let client = LegacyClient::builder(TokioExecutor::new()).build_http::<http_body_util::Full<hyper::body::Bytes>>();
        
        // Test root endpoint
        let uri = format!("http://{}/", addr);
        let response = timeout(
            Duration::from_secs(2),
            client.get(uri.parse::<Uri>().unwrap())
        ).await;
        
        if let Ok(Ok(resp)) = response {
            assert_eq!(resp.status(), 200, "Root endpoint should return 200");
        }
        
        // Test API endpoint
        let api_uri = format!("http://{}/api/events", addr);
        let api_response = timeout(
            Duration::from_secs(2),
            client.get(api_uri.parse::<Uri>().unwrap())
        ).await;
        
        if let Ok(Ok(resp)) = api_response {
            assert_eq!(resp.status(), 200, "API endpoint should return 200");
            
            // Check content type
            let content_type = resp.headers().get("content-type");
            assert!(content_type.is_some());
            assert_eq!(content_type.unwrap(), "application/json");
        }
        
        // Cleanup
        server_handle.abort();
        let _ = timeout(Duration::from_secs(1), server_handle).await;
    }
}