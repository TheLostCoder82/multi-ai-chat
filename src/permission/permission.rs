use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Represents the scope of a permission
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PermissionScope {
    /// Access to read documents
    DocumentRead,
    /// Access to write/modify documents
    DocumentWrite,
    /// Access to send private messages
    PrivateMessage,
    /// Access to participate in voting
    Voting,
    /// Access to broadcast messages
    Broadcast,
    /// Custom scope with identifier
    Custom(String),
}

/// Represents the current status of a permission
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionStatus {
    /// Permission is pending approval
    Pending,
    /// Permission has been granted
    Granted,
    /// Permission has been denied
    Denied,
    /// Permission has been revoked after being granted
    Revoked,
}

/// Represents a permission with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    /// Unique identifier for this permission
    pub id: Uuid,
    /// The agent ID this permission belongs to
    pub agent_id: String,
    /// The scope of this permission
    pub scope: PermissionScope,
    /// Current status of the permission
    pub status: PermissionStatus,
    /// When this permission was created
    pub created_at: DateTime<Utc>,
    /// When this permission was last updated
    pub updated_at: DateTime<Utc>,
    /// Optional expiration time
    pub expires_at: Option<DateTime<Utc>>,
    /// Reason for grant/denial (optional)
    pub reason: Option<String>,
    /// Who granted/denied this permission (if applicable)
    pub granted_by: Option<String>,
}

impl Permission {
    /// Create a new pending permission request
    pub fn new(agent_id: String, scope: PermissionScope) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            agent_id,
            scope,
            status: PermissionStatus::Pending,
            created_at: now,
            updated_at: now,
            expires_at: None,
            reason: None,
            granted_by: None,
        }
    }

    /// Grant this permission
    pub fn grant(&mut self, granted_by: String, reason: Option<String>) -> Result<(), PermissionError> {
        match self.status {
            PermissionStatus::Pending | PermissionStatus::Denied | PermissionStatus::Revoked => {
                self.status = PermissionStatus::Granted;
                self.updated_at = Utc::now();
                self.granted_by = Some(granted_by);
                self.reason = reason;
                Ok(())
            }
            _ => Err(PermissionError::InvalidTransition),
        }
    }

    /// Deny this permission
    pub fn deny(&mut self, granted_by: String, reason: Option<String>) -> Result<(), PermissionError> {
        match self.status {
            PermissionStatus::Pending | PermissionStatus::Granted => {
                self.status = PermissionStatus::Denied;
                self.updated_at = Utc::now();
                self.granted_by = Some(granted_by);
                self.reason = reason;
                Ok(())
            }
            _ => Err(PermissionError::InvalidTransition),
        }
    }

    /// Revoke a granted permission
    pub fn revoke(&mut self, granted_by: String, reason: Option<String>) -> Result<(), PermissionError> {
        if self.status == PermissionStatus::Granted {
            self.status = PermissionStatus::Revoked;
            self.updated_at = Utc::now();
            self.granted_by = Some(granted_by);
            self.reason = reason;
            Ok(())
        } else {
            Err(PermissionError::InvalidTransition)
        }
    }

    /// Check if this permission is currently active
    pub fn is_active(&self) -> bool {
        self.status == PermissionStatus::Granted && 
        self.expires_at.map_or(true, |exp| exp > Utc::now())
    }

    /// Set an expiration time for this permission
    pub fn set_expiration(&mut self, expires_at: DateTime<Utc>) {
        self.expires_at = Some(expires_at);
        self.updated_at = Utc::now();
    }
}

/// Represents a permission request from an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    /// The agent requesting permission
    pub agent_id: String,
    /// The requested permission scope
    pub scope: PermissionScope,
    /// Optional reason for the request
    pub reason: Option<String>,
}

/// Errors that can occur during permission operations
#[derive(Debug, thiserror::Error)]
pub enum PermissionError {
    #[error("Invalid state transition")]
    InvalidTransition,
    #[error("Permission not found")]
    NotFound,
    #[error("Permission expired")]
    Expired,
    #[error("Storage error: {0}")]
    StorageError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_creation() {
        let perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
        assert_eq!(perm.status, PermissionStatus::Pending);
        assert_eq!(perm.agent_id, "agent1");
        assert_eq!(perm.scope, PermissionScope::DocumentRead);
    }

    #[test]
    fn test_permission_grant() {
        let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
        assert!(perm.grant("admin".to_string(), Some("Test reason".to_string())).is_ok());
        assert_eq!(perm.status, PermissionStatus::Granted);
        assert_eq!(perm.granted_by, Some("admin".to_string()));
    }

    #[test]
    fn test_permission_deny() {
        let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
        assert!(perm.deny("admin".to_string(), Some("Not authorized".to_string())).is_ok());
        assert_eq!(perm.status, PermissionStatus::Denied);
    }

    #[test]
    fn test_permission_revoke() {
        let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
        perm.grant("admin".to_string(), None).unwrap();
        assert!(perm.revoke("admin".to_string(), Some("Policy change".to_string())).is_ok());
        assert_eq!(perm.status, PermissionStatus::Revoked);
    }

    #[test]
    fn test_invalid_transition() {
        let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
        perm.deny("admin".to_string(), None).unwrap();
        // Cannot revoke a denied permission
        assert!(perm.revoke("admin".to_string(), None).is_err());
    }
}
