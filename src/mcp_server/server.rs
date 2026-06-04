//! MCP WebSocket Server
//! 
//! This module implements the main MCP server that accepts WebSocket connections
//! from AI agents and manages their sessions.

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, RwLock};
use futures_util::{stream::{SplitSink, SplitStream}, SinkExt, StreamExt};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::{Message, Error as WsError};
use uuid::Uuid;
use log::{info, warn, error, debug};

use crate::permission::PermissionStorage;
use crate::message_router::MessageRouter;
use crate::document_storage::DocumentStorage;
use super::session::{Session, SessionManager};
use super::protocol::{McpMessage, McpPayload, McpMethod, McpRequest, McpError};
use super::handshake::{HandshakeState, ClientHello, ClientResponse};

/// MCP Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Host address to bind to
    pub host: String,
    /// Port to listen on
    pub port: u16,
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Session timeout in seconds
    pub session_timeout_seconds: i64,
    /// Enable authentication
    pub require_auth: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 100,
            session_timeout_seconds: 3600, // 1 hour
            require_auth: false,
        }
    }
}

/// Main MCP Server instance
pub struct McpServer {
    config: ServerConfig,
    session_manager: SessionManager,
    permission_storage: PermissionStorage,
    message_router: MessageRouter,
    document_storage: DocumentStorage,
    shutdown_tx: broadcast::Sender<()>,
    is_running: Arc<RwLock<bool>>,
}

impl McpServer {
    /// Create a new MCP server instance
    pub fn new(
        config: ServerConfig,
        permission_storage: PermissionStorage,
        message_router: MessageRouter,
        document_storage: DocumentStorage,
    ) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        
        Self {
            config,
            session_manager: SessionManager::new(),
            permission_storage,
            message_router,
            document_storage,
            shutdown_tx,
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the MCP server
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await?;
        
        info!("MCP Server listening on {}", addr);
        
        *self.is_running.write().await = true;
        
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        
        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((socket, peer_addr)) => {
                            debug!("New connection from {}", peer_addr);
                            
                            // Check connection limit
                            if self.session_manager.count().await >= self.config.max_connections {
                                warn!("Max connections reached, rejecting {}", peer_addr);
                                drop(socket);
                                continue;
                            }
                            
                            // Clone necessary components for the connection handler
                            let session_manager = self.session_manager.clone();
                            let permissions = self.permission_storage.clone();
                            let router = self.message_router.clone();
                            let docs = self.document_storage.clone();
                            let config = self.config.clone();
                            let is_running = self.is_running.clone();
                            
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(
                                    socket,
                                    peer_addr,
                                    session_manager,
                                    permissions,
                                    router,
                                    docs,
                                    config,
                                    is_running,
                                ).await {
                                    error!("Connection error from {}: {}", peer_addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Accept error: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received");
                    break;
                }
            }
        }
        
        *self.is_running.write().await = false;
        Ok(())
    }

    /// Stop the MCP server
    pub async fn stop(&self) -> Result<(), broadcast::error::SendError<()>> {
        self.shutdown_tx.send(())?;
        Ok(())
    }

    /// Check if server is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Get active session count
    pub async fn session_count(&self) -> usize {
        self.session_manager.count().await
    }

    /// Broadcast a message to all connected agents
    pub async fn broadcast_message(&self, message: String) -> Result<(), Box<dyn std::error::Error>> {
        let sessions = self.session_manager.get_all_sessions().await;
        for session in sessions {
            let s = session.read().await;
            if s.is_active() {
                if let Err(e) = s.send(message.clone()).await {
                    warn!("Failed to send to session {}: {}", s.id, e);
                }
            }
        }
        Ok(())
    }

    /// Send a message to a specific agent
    pub async fn send_to_agent(&self, agent_id: &str, message: String) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(session) = self.session_manager.get_session_by_agent(agent_id).await {
            let s = session.read().await;
            s.send(message).await?;
            Ok(())
        } else {
            Err(format!("Agent {} not found", agent_id).into())
        }
    }
}

/// Handle an individual WebSocket connection
async fn handle_connection(
    stream: TcpStream,
    peer_addr: SocketAddr,
    session_manager: SessionManager,
    permissions: PermissionStorage,
    router: MessageRouter,
    docs: DocumentStorage,
    config: ServerConfig,
    _is_running: Arc<RwLock<bool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Upgrade to WebSocket
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    let (mut tx, mut rx) = ws_stream.split();
    
    // Create channel for session communication
    let (session_tx, mut session_rx) = mpsc::channel::<String>(1000);
    
    // Create session
    let session = Session::new(session_tx, permissions.clone());
    let session_id = session_manager.add_session(session).await;
    
    info!("New session created: {} from {}", session_id, peer_addr);
    
    // Get session reference
    let session_arc = session_manager.get_session(&session_id).await
        .ok_or("Session not found")?;
    
    // Spawn task to forward messages from session to WebSocket
    let tx_handle = tokio::spawn(async move {
        while let Some(msg) = session_rx.recv().await {
            if let Err(e) = tx.send(Message::Text(msg)).await {
                warn!("Failed to send message: {}", e);
                break;
            }
        }
    });
    
    // Handle incoming messages
    let result = handle_messages(
        &mut rx,
        &session_arc,
        &router,
        &docs,
        config.require_auth,
    ).await;
    
    // Cleanup on disconnect
    session_manager.remove_session(&session_id).await;
    tx_handle.abort();
    
    info!("Session {} disconnected", session_id);
    
    result
}

/// Process incoming WebSocket messages
async fn handle_messages(
    rx: &mut SplitStream<WebSocketStream<TcpStream>>,
    session: &Arc<RwLock<Session>>,
    router: &MessageRouter,
    docs: &DocumentStorage,
    require_auth: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut handshake_state = HandshakeState::new();
    let mut challenge_id: Option<String> = None;
    
    while let Some(msg_result) = rx.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                // Parse MCP message
                let mcp_msg: McpMessage = match serde_json::from_str(&text) {
                    Ok(msg) => msg,
                    Err(e) => {
                        let error_msg = McpMessage::error_response(
                            String::new(),
                            McpError::PARSE_ERROR,
                            format!("Invalid JSON: {}", e),
                        );
                        let response = serde_json::to_string(&error_msg)?;
                        let s = session.read().await;
                        s.send(response).await?;
                        continue;
                    }
                };
                
                // Update session activity
                {
                    let mut s = session.write().await;
                    s.touch();
                }
                
                // Process based on handshake state
                if !handshake_state.is_authenticated() {
                    // Still in handshake phase
                    if let Some(request) = mcp_msg.as_request() {
                        if matches!(request.method, McpMethod::Initialize) {
                            // Extract hello from params
                            let hello: ClientHello = serde_json::from_value(request.params.clone())
                                .map_err(|e| format!("Invalid hello: {}", e))?;
                            
                            match handshake_state.receive_hello(&hello) {
                                Ok(challenge) => {
                                    challenge_id = Some(challenge.challenge_id.clone());
                                    
                                    // Send challenge back
                                    let response_data = serde_json::json!({
                                        "challenge": challenge,
                                        "authRequired": require_auth && hello.auth_token.is_none(),
                                    });
                                    
                                    let response_msg = McpMessage::response(
                                        request.request_id.clone().unwrap_or_default(),
                                        response_data,
                                    );
                                    
                                    let response = serde_json::to_string(&response_msg)?;
                                    let s = session.read().await;
                                    s.send(response).await?;
                                }
                                Err(e) => {
                                    let error_msg = McpMessage::error_response(
                                        request.request_id.clone().unwrap_or_default(),
                                        McpError::INVALID_REQUEST,
                                        e,
                                    );
                                    let response = serde_json::to_string(&error_msg)?;
                                    let s = session.read().await;
                                    s.send(response).await?;
                                    return Err("Handshake failed".into());
                                }
                            }
                        } else {
                            // Must initialize first
                            let error_msg = McpMessage::error_response(
                                mcp_msg.id,
                                McpError::INVALID_REQUEST,
                                "Must initialize connection first".to_string(),
                            );
                            let response = serde_json::to_string(&error_msg)?;
                            let s = session.read().await;
                            s.send(response).await?;
                        }
                    }
                } else {
                    // Authenticated - process normal requests
                    if let Some(request) = mcp_msg.as_request() {
                        let response = process_request(
                            request,
                            session,
                            router,
                            docs,
                        ).await?;
                        
                        let s = session.read().await;
                        s.send(response).await?;
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                // Respond to ping with pong
                let s = session.read().await;
                s.send(Message::Pong(data).to_string()).await?;
            }
            Ok(Message::Close(_)) => {
                info!("Client requested close");
                break;
            }
            Ok(_) => {
                // Ignore other message types
            }
            Err(WsError::ConnectionClosed) => {
                info!("Connection closed by client");
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                return Err(Box::new(e));
            }
        }
    }
    
    Ok(())
}

/// Process an authenticated MCP request
async fn process_request(
    request: &McpRequest,
    session: &Arc<RwLock<Session>>,
    router: &MessageRouter,
    docs: &DocumentStorage,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let response = match &request.method {
        McpMethod::SendMessage => {
            // Handle chat message
            handle_send_message(request, session, router).await?
        }
        McpMethod::RequestPermission => {
            // Handle permission request
            handle_permission_request(request, session).await?
        }
        McpMethod::Submit => {
            // Handle work submission
            handle_submit(request, session, router).await?
        }
        McpMethod::Vote => {
            // Handle vote
            handle_vote(request, session, router).await?
        }
        McpMethod::GetDocument => {
            // Handle document retrieval
            handle_get_document(request, docs).await?
        }
        McpMethod::ListDocuments => {
            // Handle document listing
            handle_list_documents(request, docs).await?
        }
        McpMethod::Ping => {
            // Heartbeat response
            serde_json::json!({"status": "pong", "timestamp": chrono::Utc::now()})
        }
        _ => {
            return Ok(serde_json::to_string(&McpMessage::error_response(
                request.request_id.clone().unwrap_or_default(),
                McpError::METHOD_NOT_FOUND,
                format!("Method {:?} not implemented", request.method),
            ))?);
        }
    };
    
    Ok(serde_json::to_string(&McpMessage::response(
        request.request_id.clone().unwrap_or_default(),
        response,
    ))?)
}

/// Handle send message request
async fn handle_send_message(
    request: &McpRequest,
    session: &Arc<RwLock<Session>>,
    router: &MessageRouter,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let s = session.read().await;
    let agent_id = s.agent_id().ok_or("Agent not registered")?.to_string();
    drop(s);
    
    let channel_id: String = serde_json::from_value(request.params["channelId"].clone())
        .map_err(|_| "Missing channelId")?;
    let content: String = serde_json::from_value(request.params["content"].clone())
        .map_err(|_| "Missing content")?;
    
    let priority = serde_json::from_value(request.params["priority"].clone())
        .unwrap_or(crate::message_router::MessagePriority::Normal);
    
    let msg = router.send_message(agent_id, channel_id, content, priority).await;
    
    Ok(serde_json::json!({
        "messageId": msg.id,
        "timestamp": msg.timestamp,
    }))
}

/// Handle permission request
async fn handle_permission_request(
    request: &McpRequest,
    session: &Arc<RwLock<Session>>,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let s = session.read().await;
    let agent_id = s.agent_id().ok_or("Agent not registered")?.to_string();
    drop(s);
    
    let scope_str: String = serde_json::from_value(request.params["scope"].clone())
        .map_err(|_| "Missing scope")?;
    
    // In a real implementation, this would create a pending permission request
    // and notify the human user for approval
    
    Ok(serde_json::json!({
        "status": "pending",
        "message": "Permission request sent to user",
        "agentId": agent_id,
        "scope": scope_str,
    }))
}

/// Handle submit request
async fn handle_submit(
    request: &McpRequest,
    session: &Arc<RwLock<Session>>,
    router: &MessageRouter,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let mut s = session.write().await;
    let agent_id = s.agent_id().ok_or("Agent not registered")?.to_string();
    
    // Revoke write permissions upon submission
    // In a real implementation, this would update permission status
    
    drop(s);
    
    let channel_id: String = serde_json::from_value(request.params["channelId"].clone())
        .map_err(|_| "Missing channelId")?;
    let summary: String = serde_json::from_value(request.params["summary"].clone())
        .map_err(|_| "Missing summary")?;
    
    // Send summary to general chat
    let _msg = router.send_message(
        agent_id,
        channel_id,
        format!("📋 Submission from {}: {}", agent_id, summary),
        crate::message_router::MessagePriority::Normal,
    ).await;
    
    Ok(serde_json::json!({
        "status": "submitted",
        "message": "Work submitted for review",
    }))
}

/// Handle vote request
async fn handle_vote(
    request: &McpRequest,
    session: &Arc<RwLock<Session>>,
    router: &MessageRouter,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let s = session.read().await;
    let agent_id = s.agent_id().ok_or("Agent not registered")?.to_string();
    drop(s);
    
    let message_id: String = serde_json::from_value(request.params["messageId"].clone())
        .map_err(|_| "Missing messageId")?;
    let vote_type: String = serde_json::from_value(request.params["voteType"].clone())
        .map_err(|_| "Missing voteType")?;
    
    // In a real implementation, this would record the vote
    // For now, just acknowledge
    
    Ok(serde_json::json!({
        "status": "recorded",
        "messageId": message_id,
        "voteType": vote_type,
        "voterId": agent_id,
    }))
}

/// Handle get document request
async fn handle_get_document(
    request: &McpRequest,
    docs: &DocumentStorage,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let doc_id: String = serde_json::from_value(request.params["documentId"].clone())
        .map_err(|_| "Missing documentId")?;
    
    let metadata = docs.get_metadata(&doc_id).await
        .ok_or_else(|| format!("Document {} not found", doc_id))?;
    
    let version = docs.get_latest_version(&doc_id).await
        .ok_or_else(|| format!("No versions found for document {}", doc_id))?;
    
    Ok(serde_json::json!({
        "metadata": metadata,
        "content": version.content,
        "version": version.version.number,
    }))
}

/// Handle list documents request
async fn handle_list_documents(
    request: &McpRequest,
    docs: &DocumentStorage,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let query: Option<String> = serde_json::from_value(request.params["query"].clone()).ok();
    
    let documents = if let Some(q) = query {
        docs.search_documents(&q).await
    } else {
        docs.list_documents().await
    };
    
    Ok(serde_json::json!({
        "documents": documents,
        "count": documents.len(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.max_connections, 100);
        assert!(!config.require_auth);
    }
    
    #[tokio::test]
    async fn test_server_creation() {
        let config = ServerConfig::default();
        let permissions = PermissionStorage::new();
        let router = MessageRouter::new(100, 1000);
        let docs = DocumentStorage::new(None);
        
        let server = McpServer::new(config, permissions, router, docs);
        assert!(!server.is_running().await);
    }
}
