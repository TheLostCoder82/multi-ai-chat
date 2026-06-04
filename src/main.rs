//! Multi-Agent Chat Application with MCP Server Interface
//!
//! This application provides a chat interface for multiple AI agents
//! with support for permissions, document management, and voting.

mod permission;
mod message_router;
mod document_storage;
mod utils;

use anyhow::Result;

pub use permission::{Permission, PermissionScope, PermissionStatus, PermissionStorage};
pub use message_router::{MessageRouter, Channel, ChatMessage, MessagePriority};
pub use document_storage::{DocumentStorage, DocumentVersion, DocumentMetadata};

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
    
    // In a real application, you would now start the MCP server
    // and begin listening for agent connections
    
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
