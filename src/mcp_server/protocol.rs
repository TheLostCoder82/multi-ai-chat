//! MCP Protocol Message Types
//! 
//! This module defines the message structures for the Model Context Protocol.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// MCP Message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMessage {
    /// Unique message ID
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// The actual request or response
    pub payload: McpPayload,
}

/// MCP Payload - either request or response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum McpPayload {
    Request(McpRequest),
    Response(McpResponse),
}

/// MCP Request methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum McpMethod {
    /// Initialize connection
    Initialize,
    /// Send a chat message
    SendMessage,
    /// Request permission
    RequestPermission,
    /// Submit work for review
    Submit,
    /// Vote on a message
    Vote,
    /// Request counter-proposal
    RequestCounterProposal,
    /// State reason for vote
    StateVoteReason,
    /// Open private message channel
    OpenPrivateChannel,
    /// Close private message channel
    ClosePrivateChannel,
    /// Get document content
    GetDocument,
    /// List documents
    ListDocuments,
    /// Heartbeat/keepalive
    Ping,
    /// Custom method
    Custom(String),
}

/// MCP Request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    /// Method to invoke
    pub method: McpMethod,
    /// Request parameters
    pub params: serde_json::Value,
    /// Optional request ID for matching responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// MCP Response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    /// Corresponding request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Success result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

/// MCP Error structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl McpMessage {
    /// Create a new request message
    pub fn request(method: McpMethod, params: serde_json::Value) -> Self {
        let request_id = Uuid::new_v4().to_string();
        Self {
            id: request_id.clone(),
            timestamp: Utc::now(),
            payload: McpPayload::Request(McpRequest {
                method,
                params,
                request_id: Some(request_id),
            }),
        }
    }

    /// Create a new response message
    pub fn response(request_id: String, result: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            payload: McpPayload::Response(McpResponse {
                request_id: Some(request_id),
                result: Some(result),
                error: None,
            }),
        }
    }

    /// Create an error response
    pub fn error_response(request_id: String, code: i32, message: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            payload: McpPayload::Response(McpResponse {
                request_id: Some(request_id),
                result: None,
                error: Some(McpError {
                    code,
                    message,
                    data: None,
                }),
            }),
        }
    }

    /// Extract request from message
    pub fn as_request(&self) -> Option<&McpRequest> {
        match &self.payload {
            McpPayload::Request(req) => Some(req),
            _ => None,
        }
    }

    /// Extract response from message
    pub fn as_response(&self) -> Option<&McpResponse> {
        match &self.payload {
            McpPayload::Response(resp) => Some(resp),
            _ => None,
        }
    }
}

impl McpError {
    /// Parse error
    pub const PARSE_ERROR: i32 = -32700;
    /// Invalid request
    pub const INVALID_REQUEST: i32 = -32600;
    /// Method not found
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// Invalid params
    pub const INVALID_PARAMS: i32 = -32602;
    /// Internal error
    pub const INTERNAL_ERROR: i32 = -32603;
    /// Permission denied
    pub const PERMISSION_DENIED: i32 = -32000;
    /// Session expired
    pub const SESSION_EXPIRED: i32 = -32001;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_request() {
        let msg = McpMessage::request(
            McpMethod::Initialize,
            json!({"agentId": "test-agent"}),
        );
        
        assert!(msg.as_request().is_some());
        assert_eq!(msg.as_request().unwrap().method, McpMethod::Initialize);
    }

    #[test]
    fn test_create_response() {
        let request_id = "test-123".to_string();
        let msg = McpMessage::response(
            request_id.clone(),
            json!({"status": "ok"}),
        );
        
        assert!(msg.as_response().is_some());
        let resp = msg.as_response().unwrap();
        assert_eq!(resp.request_id, Some(request_id));
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_error_response() {
        let request_id = "test-456".to_string();
        let msg = McpMessage::error_response(
            request_id.clone(),
            McpError::PERMISSION_DENIED,
            "Access denied".to_string(),
        );
        
        let resp = msg.as_response().unwrap();
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, McpError::PERMISSION_DENIED);
    }

    #[test]
    fn test_serialization() {
        let msg = McpMessage::request(
            McpMethod::SendMessage,
            json!({
                "channelId": "general",
                "content": "Hello, world!",
                "senderId": "agent-1"
            }),
        );
        
        let serialized = serde_json::to_string(&msg).unwrap();
        let deserialized: McpMessage = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(msg.id, deserialized.id);
        assert!(deserialized.as_request().is_some());
    }
}
