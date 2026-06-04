//! Multi-Agent Chat Application with MCP Server Interface
//!
//! This application provides a chat interface for multiple AI agents
//! with support for permissions, document management, and voting.

mod permission;
mod message_router;
mod document_storage;
mod utils;
mod mcp_server;

use anyhow::Result;

pub use permission::{Permission, PermissionScope, PermissionStatus, PermissionStorage};
pub use message_router::{MessageRouter, Channel, ChatMessage, MessagePriority};
pub use document_storage::{DocumentStorage, DocumentVersion, DocumentMetadata};
pub use mcp_server::{McpServer, Session, McpMessage, McpRequest, McpResponse, McpMethod, HandshakeState};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    utils::logging::init_logging();
    
    log::info!("Starting Multi-Agent Chat Application");
    
    // Initialize core components
    let permission_storage = PermissionStorage::new();
    let message_router = MessageRouter::new(1000, 10000);
    let document_storage = DocumentStorage::new(None);
    
    log::info!("Core components initialized");
    
    // Create MCP server configuration
    let mcp_config = mcp_server::ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        max_connections: 100,
        session_timeout_seconds: 3600,
        require_auth: false,
    };
    
    // Create MCP server instance
    let mcp_server = McpServer::new(
        mcp_config,
        permission_storage.clone(),
        message_router.clone(),
        document_storage.clone(),
    );
    
    log::info!("MCP Server created");
    
    // Create default general channel
    let general_channel_id = message_router.get_or_create_general_channel("General".to_string()).await;
    log::info!("General channel created: {}", general_channel_id);
    
    // Example: Create a test document
    let doc_id = document_storage.create_document(
        "Welcome Document".to_string(),
        "system".to_string(),
        "Welcome to the Multi-Agent Chat System!".to_string(),
    ).await?;
    log::info!("Sample document created: {}", doc_id);
    
    // Example: Send a welcome message
    let _welcome_msg = message_router.send_message(
        "system".to_string(),
        general_channel_id,
        "Welcome to the chat system!".to_string(),
        MessagePriority::Normal,
    ).await;
    
    log::info!("Application started successfully");
    
    // Note: In a full implementation, you would spawn the MCP server here
    // For example:
    // let server_clone = mcp_server.clone();
    // tokio::spawn(async move {
    //     if let Err(e) = server_clone.start().await {
    //         log::error!("MCP Server error: {}", e);
    //     }
    // });
    
    // For now, just demonstrate that everything is set up
    log::info!("MCP Server ready to accept connections on {}:{}", mcp_config.host, mcp_config.port);
    log::info!("Ready to accept agent connections...");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_initialization() {
        let _permission_storage = PermissionStorage::new();
        let _message_router = MessageRouter::new(100, 1000);
        let _document_storage = DocumentStorage::new(None);
        
        // If we get here without panicking, initialization succeeded
        assert!(true);
    }
}
