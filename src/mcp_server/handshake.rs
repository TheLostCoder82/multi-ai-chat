//! MCP Handshake Protocol
//! 
//! This module implements the authentication handshake for agent connections.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Handshake state machine states
#[derive(Debug, Clone, PartialEq)]
pub enum HandshakeState {
    /// Initial state - waiting for client hello
    Init,
    /// Received hello - sent challenge
    ChallengeSent,
    /// Received response - verifying
    Verifying,
    /// Handshake complete - authenticated
    Authenticated,
    /// Handshake failed
    Failed(String),
}

/// Client hello message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientHello {
    /// Protocol version
    pub version: String,
    /// Agent identifier
    pub agent_id: String,
    /// Agent name
    pub agent_name: String,
    /// Supported capabilities
    pub capabilities: Vec<String>,
    /// Optional authentication token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
}

/// Server challenge message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerChallenge {
    /// Challenge ID
    pub challenge_id: String,
    /// Nonce for verification
    pub nonce: String,
    /// Required authentication method
    pub auth_method: AuthMethod,
}

/// Authentication methods
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    /// No authentication required
    None,
    /// Token-based authentication
    Token,
    /// Challenge-response authentication
    ChallengeResponse,
    /// Custom authentication
    Custom(String),
}

/// Client response to challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientResponse {
    /// Original challenge ID
    pub challenge_id: String,
    /// Response data (signature, token, etc.)
    pub response_data: String,
}

/// Handshake result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResult {
    /// Success status
    pub success: bool,
    /// Session token if successful
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Agent info if authenticated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_info: Option<AgentInfoMinimal>,
}

/// Minimal agent info for handshake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfoMinimal {
    pub agent_id: String,
    pub agent_name: String,
    pub capabilities: Vec<String>,
}

impl HandshakeState {
    /// Create a new initial handshake state
    pub fn new() -> Self {
        Self::Init
    }

    /// Process client hello and transition state
    pub fn receive_hello(&mut self, hello: &ClientHello) -> Result<ServerChallenge, String> {
        match self {
            HandshakeState::Init => {
                // Validate protocol version
                if !Self::is_supported_version(&hello.version) {
                    *self = HandshakeState::Failed(format!(
                        "Unsupported protocol version: {}",
                        hello.version
                    ));
                    return Err(format!("Unsupported protocol version: {}", hello.version));
                }

                // Generate challenge
                let challenge_id = Uuid::new_v4().to_string();
                let nonce = Uuid::new_v4().to_string();
                
                let auth_method = if hello.auth_token.is_some() {
                    AuthMethod::Token
                } else {
                    AuthMethod::ChallengeResponse
                };

                let challenge = ServerChallenge {
                    challenge_id,
                    nonce,
                    auth_method,
                };

                *self = HandshakeState::ChallengeSent;
                Ok(challenge)
            }
            _ => Err("Invalid handshake state".to_string()),
        }
    }

    /// Process client response and transition state
    pub fn receive_response(
        &mut self,
        response: &ClientResponse,
        expected_challenge_id: &str,
    ) -> Result<HandshakeResult, String> {
        match self {
            HandshakeState::ChallengeSent => {
                *self = HandshakeState::Verifying;

                // Verify challenge ID matches
                if response.challenge_id != expected_challenge_id {
                    *self = HandshakeState::Failed("Challenge ID mismatch".to_string());
                    return Ok(HandshakeResult {
                        success: false,
                        session_token: None,
                        error: Some("Challenge ID mismatch".to_string()),
                        agent_info: None,
                    });
                }

                // In a real implementation, verify the response data here
                // For now, we'll accept any valid response
                let session_token = Uuid::new_v4().to_string();

                *self = HandshakeState::Authenticated;
                Ok(HandshakeResult {
                    success: true,
                    session_token: Some(session_token),
                    error: None,
                    agent_info: None, // Will be populated by caller
                })
            }
            _ => Err("Invalid handshake state".to_string()),
        }
    }

    /// Check if handshake is complete
    pub fn is_authenticated(&self) -> bool {
        matches!(self, HandshakeState::Authenticated)
    }

    /// Check if handshake has failed
    pub fn is_failed(&self) -> bool {
        matches!(self, HandshakeState::Failed(_))
    }

    /// Get supported protocol versions
    fn is_supported_version(version: &str) -> bool {
        // Support versions 1.x
        version.starts_with("1.")
    }
}

impl Default for HandshakeState {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for handshake processing
pub fn generate_nonce() -> String {
    Uuid::new_v4().to_string()
}

pub fn validate_agent_id(agent_id: &str) -> bool {
    // Basic validation - non-empty and reasonable length
    !agent_id.is_empty() && agent_id.len() <= 256
}

pub fn sanitize_agent_name(name: &str) -> String {
    // Remove potentially dangerous characters
    name.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-' || *c == '_')
        .take(128)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let state = HandshakeState::new();
        assert_eq!(state, HandshakeState::Init);
    }

    #[test]
    fn test_receive_hello() {
        let mut state = HandshakeState::new();
        let hello = ClientHello {
            version: "1.0".to_string(),
            agent_id: "agent-123".to_string(),
            agent_name: "Test Agent".to_string(),
            capabilities: vec!["chat".to_string()],
            auth_token: None,
        };

        let result = state.receive_hello(&hello);
        assert!(result.is_ok());
        assert_eq!(state, HandshakeState::ChallengeSent);

        let challenge = result.unwrap();
        assert!(!challenge.challenge_id.is_empty());
        assert!(!challenge.nonce.is_empty());
    }

    #[test]
    fn test_unsupported_version() {
        let mut state = HandshakeState::new();
        let hello = ClientHello {
            version: "0.9".to_string(),
            agent_id: "agent-123".to_string(),
            agent_name: "Test Agent".to_string(),
            capabilities: vec![],
            auth_token: None,
        };

        let result = state.receive_hello(&hello);
        assert!(result.is_err());
        assert!(state.is_failed());
    }

    #[test]
    fn test_receive_response() {
        let mut state = HandshakeState::new();
        let hello = ClientHello {
            version: "1.0".to_string(),
            agent_id: "agent-123".to_string(),
            agent_name: "Test Agent".to_string(),
            capabilities: vec![],
            auth_token: None,
        };

        let challenge = state.receive_hello(&hello).unwrap();
        
        let response = ClientResponse {
            challenge_id: challenge.challenge_id.clone(),
            response_data: "valid-response".to_string(),
        };

        let result = state.receive_response(&response, &challenge.challenge_id);
        assert!(result.is_ok());
        assert!(state.is_authenticated());

        let handshake_result = result.unwrap();
        assert!(handshake_result.success);
        assert!(handshake_result.session_token.is_some());
    }

    #[test]
    fn test_challenge_mismatch() {
        let mut state = HandshakeState::new();
        let hello = ClientHello {
            version: "1.0".to_string(),
            agent_id: "agent-123".to_string(),
            agent_name: "Test Agent".to_string(),
            capabilities: vec![],
            auth_token: None,
        };

        let challenge = state.receive_hello(&hello).unwrap();
        
        let response = ClientResponse {
            challenge_id: "wrong-id".to_string(),
            response_data: "invalid".to_string(),
        };

        let result = state.receive_response(&response, &challenge.challenge_id);
        assert!(result.is_ok());
        
        let handshake_result = result.unwrap();
        assert!(!handshake_result.success);
        assert!(handshake_result.error.is_some());
    }

    #[test]
    fn test_validate_agent_id() {
        assert!(validate_agent_id("agent-123"));
        assert!(validate_agent_id("a"));
        assert!(!validate_agent_id(""));
        assert!(!validate_agent_id(&"a".repeat(257)));
    }

    #[test]
    fn test_sanitize_agent_name() {
        assert_eq!(sanitize_agent_name("Test Agent"), "Test Agent");
        assert_eq!(sanitize_agent_name("Agent<script>"), "Agentscript");
        assert_eq!(sanitize_agent_name("A-B_C"), "A-B_C");
        assert_eq!(sanitize_agent_name(&"a".repeat(200)), "a".repeat(128));
    }
}
