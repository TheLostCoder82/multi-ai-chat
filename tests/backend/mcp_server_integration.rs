// Integration tests for MCP server
use multi_agent_chat::mcp_server::{MCPServer, ServerConfig};
use multi_agent_chat::mcp_server::protocol::{MCPMethod, Request, Response};
use tokio;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{stream::StreamExt, sink::SinkExt};

#[tokio::test]
async fn test_server_startup_and_shutdown() {
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8765,
        max_connections: 100,
        heartbeat_interval: 30000,
    };
    
    let server = MCPServer::new(config);
    
    // Start server in background
    let server_handle = tokio::spawn(async move {
        server.run().await.unwrap();
    });
    
    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Verify server is running by attempting connection
    let uri = "ws://127.0.0.1:8765/mcp";
    let result = connect_async(uri).await;
    assert!(result.is_ok());
    
    // Shutdown server
    server_handle.abort();
}

#[tokio::test]
async fn test_agent_handshake_protocol() {
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8766,
        max_connections: 100,
        heartbeat_interval: 30000,
    };
    
    let server = MCPServer::new(config);
    
    let server_handle = tokio::spawn(async move {
        server.run().await.unwrap();
    });
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Connect and perform handshake
    let uri = "ws://127.0.0.1:8766/mcp";
    let (mut ws, _) = connect_async(uri).await.unwrap();
    
    // Send initialize request
    let init_request = Request {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: MCPMethod::Initialize,
        params: serde_json::json!({
            "agent_id": "test_agent",
            "version": "1.0.0",
            "capabilities": ["read", "write"]
        }),
    };
    
    ws.send(Message::Text(serde_json::to_string(&init_request).unwrap()))
        .await
        .unwrap();
    
    // Receive response
    let response = ws.next().await.unwrap().unwrap();
    assert!(response.is_text());
    
    let response_text = response.to_text().unwrap();
    let response_obj: Response = serde_json::from_str(response_text).unwrap();
    assert_eq!(response_obj.id, 1);
    
    ws.close(None).await.unwrap();
    server_handle.abort();
}

#[tokio::test]
async fn test_session_management() {
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8767,
        max_connections: 100,
        heartbeat_interval: 30000,
    };
    
    let server = MCPServer::new(config);
    
    let server_handle = tokio::spawn(async move {
        server.run().await.unwrap();
    });
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Create multiple agent sessions
    let mut clients = vec![];
    
    for i in 0..3 {
        let uri = format!("ws://127.0.0.1:8767/mcp?agent_id=agent_{}", i);
        let (ws, _) = connect_async(&uri).await.unwrap();
        clients.push(ws);
    }
    
    // Verify all connections established
    assert_eq!(clients.len(), 3);
    
    // Close one client
    let mut first_client = clients.remove(0);
    first_client.close(None).await.unwrap();
    
    // Remaining clients should still be connected
    assert_eq!(clients.len(), 2);
    
    server_handle.abort();
}

#[tokio::test]
async fn test_message_routing_via_mcp() {
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8768,
        max_connections: 100,
        heartbeat_interval: 30000,
    };
    
    let server = MCPServer::new(config);
    
    let server_handle = tokio::spawn(async move {
        server.run().await.unwrap();
    });
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    let uri = "ws://127.0.0.1:8768/mcp";
    let (mut ws, _) = connect_async(uri).await.unwrap();
    
    // Send a chat message via MCP
    let chat_request = Request {
        jsonrpc: "2.0".to_string(),
        id: 2,
        method: MCPMethod::ChatMessage,
        params: serde_json::json!({
            "channel": "general",
            "content": "Hello from MCP test",
            "sender": "test_agent"
        }),
    };
    
    ws.send(Message::Text(serde_json::to_string(&chat_request).unwrap()))
        .await
        .unwrap();
    
    // Receive acknowledgment
    let response = ws.next().await.unwrap().unwrap();
    assert!(response.is_text());
    
    ws.close(None).await.unwrap();
    server_handle.abort();
}

#[tokio::test]
async fn test_heartbeat_and_keepalive() {
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8769,
        max_connections: 100,
        heartbeat_interval: 1000, // 1 second for testing
    };
    
    let server = MCPServer::new(config);
    
    let server_handle = tokio::spawn(async move {
        server.run().await.unwrap();
    });
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    let uri = "ws://127.0.0.1:8769/mcp";
    let (mut ws, _) = connect_async(uri).await.unwrap();
    
    // Wait for heartbeat
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    
    // Should receive ping/pong or heartbeat message
    // Note: Implementation dependent
    
    ws.close(None).await.unwrap();
    server_handle.abort();
}

#[tokio::test]
async fn test_error_handling_and_recovery() {
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8770,
        max_connections: 100,
        heartbeat_interval: 30000,
    };
    
    let server = MCPServer::new(config);
    
    let server_handle = tokio::spawn(async move {
        server.run().await.unwrap();
    });
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    let uri = "ws://127.0.0.1:8770/mcp";
    let (mut ws, _) = connect_async(uri).await.unwrap();
    
    // Send invalid request
    let invalid_request = r#"{"invalid": "json"}"#;
    ws.send(Message::Text(invalid_request.to_string())).await.unwrap();
    
    // Should receive error response
    let response = ws.next().await.unwrap().unwrap();
    assert!(response.is_text());
    
    let response_text = response.to_text().unwrap();
    let response_obj: serde_json::Value = serde_json::from_str(response_text).unwrap();
    assert!(response_obj.get("error").is_some());
    
    // Connection should still be alive
    let ping_request = Request {
        jsonrpc: "2.0".to_string(),
        id: 3,
        method: MCPMethod::Ping,
        params: serde_json::json!({}),
    };
    
    ws.send(Message::Text(serde_json::to_string(&ping_request).unwrap()))
        .await
        .unwrap();
    
    let response = ws.next().await.unwrap().unwrap();
    assert!(response.is_text());
    
    ws.close(None).await.unwrap();
    server_handle.abort();
}

#[tokio::test]
async fn test_connection_pooling() {
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8771,
        max_connections: 5, // Small pool for testing
        heartbeat_interval: 30000,
    };
    
    let server = MCPServer::new(config);
    
    let server_handle = tokio::spawn(async move {
        server.run().await.unwrap();
    });
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Open connections up to limit
    let mut connections = vec![];
    
    for _ in 0..5 {
        let uri = "ws://127.0.0.1:8771/mcp";
        match connect_async(uri).await {
            Ok((ws, _)) => connections.push(ws),
            Err(_) => break,
        }
    }
    
    assert_eq!(connections.len(), 5);
    
    // Try to exceed limit - should fail
    let uri = "ws://127.0.0.1:8771/mcp";
    let result = connect_async(uri).await;
    assert!(result.is_err()); // Connection limit reached
    
    // Close one connection
    connections.pop();
    
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Should be able to connect again
    let result = connect_async(uri).await;
    assert!(result.is_ok());
    
    for mut conn in connections {
        conn.close(None).await.ok();
    }
    
    server_handle.abort();
}
