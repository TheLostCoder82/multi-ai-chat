//! MCP Session Management
//! 
//! This module handles individual agent sessions and their state.

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::permission::{Permission, PermissionStorage};
use super::handshake::HandshakeState;

/// Agent session information
#[derive(Debug, Clone)]
pub struct AgentInfo {
    /// Unique agent identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Agent capabilities
    pub capabilities: Vec<String>,
    /// Connection timestamp
    pub connected_at: DateTime<Utc>,
}

/// Session state
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    /// Handshake in progress
    Handshaking,
    /// Authenticated and active
    Active,
    /// Session suspended
    Suspended,
    /// Session terminated
    Terminated,
}

/// Individual agent session
#[derive(Debug)]
pub struct Session {
    /// Unique session ID
    pub id: String,
    /// Associated agent info
    pub agent: Option<AgentInfo>,
    /// Current session state
    pub state: SessionState,
    /// Handshake state tracker
    pub handshake_state: HandshakeState,
    /// Outbound message sender
    pub tx: mpsc::Sender<String>,
    /// Inbound message receiver
    pub rx: Arc<RwLock<mpsc::Receiver<String>>>,
    /// Permission storage reference
    pub permissions: PermissionStorage,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
    /// Private channels this agent is part of
    pub private_channels: Vec<String>,
}

impl Session {
    /// Create a new session
    pub fn new(tx: mpsc::Sender<String>, permissions: PermissionStorage) -> Self {
        let (tx_inner, rx) = mpsc::channel(1000);
        
        Self {
            id: Uuid::new_v4().to_string(),
            agent: None,
            state: SessionState::Handshaking,
            handshake_state: HandshakeState::new(),
            tx: tx_inner,
            rx: Arc::new(RwLock::new(rx)),
            permissions,
            last_activity: Utc::now(),
            private_channels: Vec::new(),
        }
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Register agent info after successful handshake
    pub fn register_agent(&mut self, agent_id: String, name: String, capabilities: Vec<String>) {
        self.agent = Some(AgentInfo {
            id: agent_id,
            name,
            capabilities,
            connected_at: Utc::now(),
        });
    }

    /// Activate session after successful authentication
    pub fn activate(&mut self) {
        self.state = SessionState::Active;
    }

    /// Suspend session
    pub fn suspend(&mut self) {
        self.state = SessionState::Suspension;
    }

    /// Terminate session
    pub fn terminate(&mut self) {
        self.state = SessionState::Terminated;
    }

    /// Check if session is active
    pub fn is_active(&self) -> bool {
        self.state == SessionState::Active
    }

    /// Send a message to this session
    pub async fn send(&self, message: String) -> Result<(), mpsc::error::SendError<String>> {
        self.tx.send(message).await
    }

    /// Get agent ID if available
    pub fn agent_id(&self) -> Option<&str> {
        self.agent.as_ref().map(|a| a.id.as_str())
    }

    /// Check if agent has specific permission
    pub async fn has_permission(&self, scope: crate::permission::PermissionScope) -> bool {
        if let Some(agent) = &self.agent {
            self.permissions.has_permission(&agent.id, scope).await
        } else {
            false
        }
    }

    /// Add agent to a private channel
    pub fn join_private_channel(&mut self, channel_id: String) {
        if !self.private_channels.contains(&channel_id) {
            self.private_channels.push(channel_id);
        }
    }

    /// Remove agent from a private channel
    pub fn leave_private_channel(&mut self, channel_id: &str) {
        self.private_channels.retain(|id| id != channel_id);
    }

    /// Check if session has expired (no activity for extended period)
    pub fn is_expired(&self, timeout_seconds: i64) -> bool {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(self.last_activity)
            .num_seconds();
        elapsed > timeout_seconds
    }
}

/// Session manager for tracking all active sessions
#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<std::collections::HashMap<String, Arc<RwLock<Session>>>>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Add a new session
    pub async fn add_session(&self, session: Session) -> String {
        let session_id = session.id.clone();
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), Arc::new(RwLock::new(session)));
        session_id
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<Arc<RwLock<Session>>> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Get session by agent ID
    pub async fn get_session_by_agent(&self, agent_id: &str) -> Option<Arc<RwLock<Session>>> {
        let sessions = self.sessions.read().await;
        for session in sessions.values() {
            let s = session.read().await;
            if let Some(agent) = &s.agent {
                if agent.id == agent_id {
                    return Some(session.clone());
                }
            }
        }
        None
    }

    /// Remove a session
    pub async fn remove_session(&self, session_id: &str) -> Option<Arc<RwLock<Session>>> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id)
    }

    /// Get all active sessions
    pub async fn get_all_sessions(&self) -> Vec<Arc<RwLock<Session>>> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }

    /// Get session count
    pub async fn count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired(&self, timeout_seconds: i64) -> Vec<String> {
        let mut expired_ids = Vec::new();
        
        {
            let sessions = self.sessions.read().await;
            for (id, session) in sessions.iter() {
                let s = session.read().await;
                if s.is_expired(timeout_seconds) {
                    expired_ids.push(id.clone());
                }
            }
        }

        for id in &expired_ids {
            self.remove_session(id).await;
        }

        expired_ids
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_session_creation() {
        let (tx, _rx) = mpsc::channel(100);
        let permissions = PermissionStorage::new();
        let session = Session::new(tx, permissions);

        assert_eq!(session.state, SessionState::Handshaking);
        assert!(session.agent.is_none());
        assert!(!session.is_active());
    }

    #[tokio::test]
    async fn test_session_activation() {
        let (tx, _rx) = mpsc::channel(100);
        let permissions = PermissionStorage::new();
        let mut session = Session::new(tx, permissions);

        session.register_agent(
            "agent-1".to_string(),
            "Test Agent".to_string(),
            vec!["chat".to_string()],
        );
        session.activate();

        assert_eq!(session.state, SessionState::Active);
        assert!(session.is_active());
        assert_eq!(session.agent_id(), Some("agent-1"));
    }

    #[tokio::test]
    async fn test_session_manager() {
        let manager = SessionManager::new();
        let (tx, _rx) = mpsc::channel(100);
        let permissions = PermissionStorage::new();
        let session = Session::new(tx, permissions);

        let session_id = manager.add_session(session).await;
        assert_eq!(manager.count().await, 1);

        let retrieved = manager.get_session(&session_id).await;
        assert!(retrieved.is_some());

        manager.remove_session(&session_id).await;
        assert_eq!(manager.count().await, 0);
    }

    #[tokio::test]
    async fn test_private_channels() {
        let (tx, _rx) = mpsc::channel(100);
        let permissions = PermissionStorage::new();
        let mut session = Session::new(tx, permissions);

        session.join_private_channel("pm-1".to_string());
        session.join_private_channel("pm-2".to_string());
        
        assert_eq!(session.private_channels.len(), 2);
        assert!(session.private_channels.contains(&"pm-1".to_string()));

        session.leave_private_channel("pm-1");
        assert_eq!(session.private_channels.len(), 1);
    }
}
