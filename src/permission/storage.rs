use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::permission::{Permission, PermissionError, PermissionScope, PermissionStatus};

/// In-memory storage for permissions
/// In production, this would be backed by a database
#[derive(Debug, Clone)]
pub struct PermissionStorage {
    permissions: Arc<RwLock<HashMap<Uuid, Permission>>>,
    agent_permissions: Arc<RwLock<HashMap<String, Vec<Uuid>>>>,
}

impl PermissionStorage {
    /// Create a new permission storage instance
    pub fn new() -> Self {
        Self {
            permissions: Arc::new(RwLock::new(HashMap::new())),
            agent_permissions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a new permission
    pub async fn store(&self, permission: Permission) -> Result<(), PermissionError> {
        let mut perms = self.permissions.write().await;
        let mut agent_perms = self.agent_permissions.write().await;
        
        // Add to agent's permission list if not already there
        agent_perms
            .entry(permission.agent_id.clone())
            .or_insert_with(Vec::new)
            .push(permission.id);
        
        perms.insert(permission.id, permission);
        Ok(())
    }

    /// Get a permission by ID
    pub async fn get(&self, id: Uuid) -> Result<Permission, PermissionError> {
        let perms = self.permissions.read().await;
        perms.get(&id).cloned().ok_or(PermissionError::NotFound)
    }

    /// Update an existing permission
    pub async fn update(&self, permission: Permission) -> Result<(), PermissionError> {
        let mut perms = self.permissions.write().await;
        if perms.contains_key(&permission.id) {
            perms.insert(permission.id, permission);
            Ok(())
        } else {
            Err(PermissionError::NotFound)
        }
    }

    /// Get all permissions for an agent
    pub async fn get_by_agent(&self, agent_id: &str) -> Vec<Permission> {
        let agent_perms = self.agent_permissions.read().await;
        let perms = self.permissions.read().await;
        
        if let Some(ids) = agent_perms.get(agent_id) {
            ids.iter()
                .filter_map(|id| perms.get(id))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get active permissions for an agent with a specific scope
    pub async fn get_active_by_scope(&self, agent_id: &str, scope: &PermissionScope) -> Vec<Permission> {
        let all_perms = self.get_by_agent(agent_id).await;
        all_perms.into_iter()
            .filter(|p| p.scope == *scope && p.is_active())
            .collect()
    }

    /// Check if an agent has an active permission for a scope
    pub async fn has_permission(&self, agent_id: &str, scope: &PermissionScope) -> bool {
        !self.get_active_by_scope(agent_id, scope).await.is_empty()
    }

    /// Get pending permission requests
    pub async fn get_pending_requests(&self) -> Vec<Permission> {
        let perms = self.permissions.read().await;
        perms.values()
            .filter(|p| p.status == PermissionStatus::Pending)
            .cloned()
            .collect()
    }

    /// Get pending requests for a specific agent
    pub async fn get_pending_by_agent(&self, agent_id: &str) -> Vec<Permission> {
        let all_perms = self.get_by_agent(agent_id).await;
        all_perms.into_iter()
            .filter(|p| p.status == PermissionStatus::Pending)
            .collect()
    }

    /// Delete a permission
    pub async fn delete(&self, id: Uuid) -> Result<(), PermissionError> {
        let mut perms = self.permissions.write().await;
        let mut agent_perms = self.agent_permissions.write().await;
        
        if let Some(permission) = perms.remove(&id) {
            // Remove from agent's permission list
            if let Some(ids) = agent_perms.get_mut(&permission.agent_id) {
                ids.retain(|pid| *pid != id);
            }
            Ok(())
        } else {
            Err(PermissionError::NotFound)
        }
    }

    /// Get all permissions (for admin purposes)
    pub async fn get_all(&self) -> Vec<Permission> {
        let perms = self.permissions.read().await;
        perms.values().cloned().collect()
    }

    /// Clear expired permissions
    pub async fn clear_expired(&self) -> usize {
        let mut perms = self.permissions.write().await;
        let mut agent_perms = self.agent_permissions.write().await;
        
        let expired_ids: Vec<Uuid> = perms.values()
            .filter(|p| p.expires_at.map_or(false, |exp| exp < chrono::Utc::now()))
            .map(|p| p.id)
            .collect();
        
        for id in &expired_ids {
            if let Some(permission) = perms.remove(id) {
                if let Some(ids) = agent_perms.get_mut(&permission.agent_id) {
                    ids.retain(|pid| pid != id);
                }
            }
        }
        
        expired_ids.len()
    }
}

impl Default for PermissionStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_get() {
        let storage = PermissionStorage::new();
        let perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
        let id = perm.id;
        
        storage.store(perm).await.unwrap();
        let retrieved = storage.get(id).await.unwrap();
        
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.agent_id, "agent1");
    }

    #[tokio::test]
    async fn test_get_by_agent() {
        let storage = PermissionStorage::new();
        
        let perm1 = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
        let perm2 = Permission::new("agent1".to_string(), PermissionScope::PrivateMessage);
        let perm3 = Permission::new("agent2".to_string(), PermissionScope::Voting);
        
        storage.store(perm1).await.unwrap();
        storage.store(perm2).await.unwrap();
        storage.store(perm3).await.unwrap();
        
        let agent1_perms = storage.get_by_agent("agent1").await;
        assert_eq!(agent1_perms.len(), 2);
        
        let agent2_perms = storage.get_by_agent("agent2").await;
        assert_eq!(agent2_perms.len(), 1);
    }

    #[tokio::test]
    async fn test_has_permission() {
        let storage = PermissionStorage::new();
        
        let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
        perm.grant("admin".to_string(), None).unwrap();
        
        storage.store(perm).await.unwrap();
        
        assert!(storage.has_permission("agent1", &PermissionScope::DocumentRead).await);
        assert!(!storage.has_permission("agent1", &PermissionScope::PrivateMessage).await);
        assert!(!storage.has_permission("agent2", &PermissionScope::DocumentRead).await);
    }

    #[tokio::test]
    async fn test_update_permission() {
        let storage = PermissionStorage::new();
        
        let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
        let id = perm.id;
        storage.store(perm).await.unwrap();
        
        let mut updated = storage.get(id).await.unwrap();
        updated.grant("admin".to_string(), Some("Approved".to_string())).unwrap();
        
        storage.update(updated).await.unwrap();
        
        let retrieved = storage.get(id).await.unwrap();
        assert_eq!(retrieved.status, PermissionStatus::Granted);
    }

    #[tokio::test]
    async fn test_delete_permission() {
        let storage = PermissionStorage::new();
        
        let perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
        let id = perm.id;
        storage.store(perm).await.unwrap();
        
        storage.delete(id).await.unwrap();
        
        assert!(storage.get(id).await.is_err());
        assert!(storage.get_by_agent("agent1").await.is_empty());
    }
}
