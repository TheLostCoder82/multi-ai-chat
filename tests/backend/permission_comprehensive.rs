//! Comprehensive Permission Module Tests
//! 
//! This test suite covers edge cases, graceful failure scenarios, and fuzz testing
//! for the permission state machine and storage systems.

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;
use tokio::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;

// Import the permission module
use crate::permission::permission::{Permission, PermissionError, PermissionScope, PermissionStatus};
use crate::permission::storage::PermissionStorage;

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

/// Test permission expiration at exact boundary (expires_at == now)
#[tokio::test]
async fn test_permission_expiration_exact_boundary() {
    let storage = PermissionStorage::new();
    let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
    
    // Grant the permission first
    perm.grant("admin".to_string(), None).unwrap();
    
    // Set expiration to exactly now (within 1 second tolerance)
    let now = Utc::now();
    perm.set_expiration(now);
    
    storage.store(perm.clone()).await.unwrap();
    
    // The permission should be considered expired or about to expire
    // is_active() checks if expires_at > now, so at exact boundary it should be false
    assert!(!perm.is_active(), "Permission at exact expiration boundary should not be active");
}

/// Test permission with zero-duration expiration
#[tokio::test]
async fn test_permission_zero_duration_expiration() {
    let storage = PermissionStorage::new();
    let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
    perm.grant("admin".to_string(), None).unwrap();
    
    // Set expiration to creation time (zero duration)
    perm.set_expiration(perm.created_at);
    
    storage.store(perm.clone()).await.unwrap();
    
    assert!(!perm.is_active(), "Zero-duration permission should not be active");
}

/// Test permission scope Custom with empty string
#[test]
fn test_custom_scope_empty_string() {
    let perm = Permission::new("agent1".to_string(), PermissionScope::Custom("".to_string()));
    
    assert_eq!(perm.scope, PermissionScope::Custom("".to_string()));
    // Empty custom scope should still be valid
}

/// Test permission scope Custom with very long string
#[test]
fn test_custom_scope_very_long_string() {
    let long_string = "a".repeat(10000);
    let perm = Permission::new("agent1".to_string(), PermissionScope::Custom(long_string.clone()));
    
    match &perm.scope {
        PermissionScope::Custom(s) => {
            assert_eq!(s.len(), 10000);
            assert_eq!(s, &long_string);
        }
        _ => panic!("Expected Custom scope"),
    }
}

/// Test agent ID with special characters
#[test]
fn test_agent_id_special_characters() {
    let special_ids = vec![
        "agent@domain.com",
        "agent#123",
        "agent$special",
        "agent%encoded",
        "agent&ampsersand",
        "agent*star",
        "agent?question",
        "agent[bracket]",
        "agent{brace}",
        "agent|pipe",
        "agent\\backslash",
        "agent\"quote",
        "agent'apostrophe",
        "agent<angle>",
        "agent,comma",
        "agent.period",
        "agent space",
        "agent\ttab",
        "agent\nnewline",
    ];
    
    for agent_id in special_ids {
        let perm = Permission::new(agent_id.to_string(), PermissionScope::DocumentRead);
        assert_eq!(perm.agent_id, agent_id);
    }
}

/// Test agent ID with Unicode and emojis
#[test]
fn test_agent_id_unicode_and_emojis() {
    let unicode_ids = vec![
        "エージェント",  // Japanese
        "代理",         // Chinese
        "агент",       // Russian
        "🤖",          // Robot emoji
        "agent🤖123",  // Mixed
        "مرحبا",       // Arabic
        "שלום",        // Hebrew
        "🎉🚀💯",      // Multiple emojis
    ];
    
    for agent_id in unicode_ids {
        let perm = Permission::new(agent_id.to_string(), PermissionScope::DocumentRead);
        assert_eq!(perm.agent_id, agent_id);
    }
}

/// Test permission with maximum reasonable expiration
#[test]
fn test_permission_far_future_expiration() {
    let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
    perm.grant("admin".to_string(), None).unwrap();
    
    // Set expiration to 100 years in the future
    let far_future = Utc::now() + Duration::days(365 * 100);
    perm.set_expiration(far_future);
    
    assert!(perm.is_active(), "Far-future expiration should still be active");
}

/// Test permission with past expiration
#[test]
fn test_permission_past_expiration() {
    let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
    perm.grant("admin".to_string(), None).unwrap();
    
    // Set expiration to 1 year in the past
    let past = Utc::now() - Duration::days(365);
    perm.set_expiration(past);
    
    assert!(!perm.is_active(), "Past expiration should not be active");
}

// ============================================================================
// GRACEFUL FAILURE TESTS
// ============================================================================

/// Test granting an already granted permission (should fail gracefully)
#[test]
fn test_grant_already_granted_permission() {
    let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
    perm.grant("admin".to_string(), Some("First grant")).unwrap();
    
    // Attempt to grant again
    let result = perm.grant("admin2".to_string(), Some("Second grant"));
    
    assert!(result.is_err(), "Should not be able to grant already granted permission");
    assert!(matches!(result, Err(PermissionError::InvalidTransition)));
    
    // Original grant should remain unchanged
    assert_eq!(perm.status, PermissionStatus::Granted);
    assert_eq!(perm.granted_by, Some("admin".to_string()));
}

/// Test denying an already denied permission (should fail gracefully)
#[test]
fn test_deny_already_denied_permission() {
    let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
    perm.deny("admin".to_string(), Some("First denial")).unwrap();
    
    // Attempt to deny again
    let result = perm.deny("admin2".to_string(), Some("Second denial"));
    
    assert!(result.is_err(), "Should not be able to deny already denied permission");
    assert!(matches!(result, Err(PermissionError::InvalidTransition)));
    
    // Original denial should remain unchanged
    assert_eq!(perm.status, PermissionStatus::Denied);
}

/// Test revoking a non-granted permission (should fail gracefully)
#[test]
fn test_revoke_non_granted_permission() {
    let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
    
    // Try to revoke without granting first
    let result = perm.revoke("admin".to_string(), Some("Revocation"));
    
    assert!(result.is_err(), "Should not be able to revoke non-granted permission");
    assert!(matches!(result, Err(PermissionError::InvalidTransition)));
    assert_eq!(perm.status, PermissionStatus::Pending);
    
    // Try to revoke a denied permission
    perm.deny("admin".to_string(), None).unwrap();
    let result = perm.revoke("admin".to_string(), None);
    
    assert!(result.is_err(), "Should not be able to revoke denied permission");
    assert_eq!(perm.status, PermissionStatus::Denied);
}

/// Test revoking an already revoked permission
#[test]
fn test_revoke_already_revoked_permission() {
    let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
    perm.grant("admin".to_string(), None).unwrap();
    perm.revoke("admin".to_string(), Some("First revocation")).unwrap();
    
    // Try to revoke again
    let result = perm.revoke("admin2".to_string(), Some("Second revocation"));
    
    assert!(result.is_err(), "Should not be able to revoke already revoked permission");
    assert!(matches!(result, Err(PermissionError::InvalidTransition)));
    assert_eq!(perm.status, PermissionStatus::Revoked);
}

/// Test updating non-existent permission in storage
#[tokio::test]
async fn test_update_nonexistent_permission() {
    let storage = PermissionStorage::new();
    let perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
    
    // Try to update without storing first
    let result = storage.update(perm).await;
    
    assert!(result.is_err(), "Should not be able to update non-existent permission");
    assert!(matches!(result, Err(PermissionError::NotFound)));
}

/// Test getting non-existent permission from storage
#[tokio::test]
async fn test_get_nonexistent_permission() {
    let storage = PermissionStorage::new();
    let fake_id = Uuid::new_v4();
    
    let result = storage.get(fake_id).await;
    
    assert!(result.is_err(), "Should not be able to get non-existent permission");
    assert!(matches!(result, Err(PermissionError::NotFound)));
}

/// Test deleting non-existent permission from storage
#[tokio::test]
async fn test_delete_nonexistent_permission() {
    let storage = PermissionStorage::new();
    let fake_id = Uuid::new_v4();
    
    let result = storage.delete(fake_id).await;
    
    assert!(result.is_err(), "Should not be able to delete non-existent permission");
    assert!(matches!(result, Err(PermissionError::NotFound)));
}

/// Test concurrent read/write on same permission
#[tokio::test]
async fn test_concurrent_permission_access() {
    let storage = Arc::new(PermissionStorage::new());
    let mut perms = Vec::new();
    
    // Create multiple permissions for the same agent
    for i in 0..10 {
        let mut perm = Permission::new("agent1".to_string(), PermissionScope::Custom(format!("scope_{}", i)));
        perm.grant("admin".to_string(), None).unwrap();
        perms.push(perm);
    }
    
    // Store all permissions
    for perm in perms {
        let storage_clone = Arc::clone(&storage);
        tokio::spawn(async move {
            storage_clone.store(perm).await.unwrap();
        });
    }
    
    // Give tasks time to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Verify all permissions were stored
    let agent_perms = storage.get_by_agent("agent1").await;
    assert_eq!(agent_perms.len(), 10, "All concurrent writes should succeed");
}

// ============================================================================
// FUZZ TESTS (Property-Based Testing)
// ============================================================================

/// Fuzz test: Random state transition sequences
#[test]
fn test_fuzz_random_state_transitions() {
    use rand::prelude::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    
    // Use seeded RNG for reproducibility
    let mut rng = StdRng::seed_from_u64(42);
    
    #[derive(Debug, Clone, Copy)]
    enum Action {
        Grant,
        Deny,
        Revoke,
    }
    
    for _iteration in 0..100 {
        let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
        let mut expected_status = PermissionStatus::Pending;
        
        // Generate random sequence of 10 actions
        for _step in 0..10 {
            let action = match rng.gen_range(0..3) {
                0 => Action::Grant,
                1 => Action::Deny,
                _ => Action::Revoke,
            };
            
            let result = match action {
                Action::Grant => perm.grant("admin".to_string(), None),
                Action::Deny => perm.deny("admin".to_string(), None),
                Action::Revoke => perm.revoke("admin".to_string(), None),
            };
            
            // Update expected status based on valid transitions
            match (expected_status, action) {
                (PermissionStatus::Pending, Action::Grant) => expected_status = PermissionStatus::Granted,
                (PermissionStatus::Pending, Action::Deny) => expected_status = PermissionStatus::Denied,
                (PermissionStatus::Granted, Action::Deny) => expected_status = PermissionStatus::Denied,
                (PermissionStatus::Granted, Action::Revoke) => expected_status = PermissionStatus::Revoked,
                (PermissionStatus::Denied, Action::Grant) => expected_status = PermissionStatus::Granted,
                (PermissionStatus::Revoked, Action::Grant) => expected_status = PermissionStatus::Granted,
                _ => {
                    // Invalid transition expected
                    assert!(result.is_err(), "Expected error for invalid transition");
                    continue;
                }
            }
            
            if result.is_ok() {
                assert_eq!(perm.status, expected_status, "Status mismatch after action");
            }
        }
    }
}

/// Fuzz test: Random expiration times
#[test]
fn test_fuzz_random_expiration_times() {
    use rand::prelude::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    
    let mut rng = StdRng::seed_from_u64(42);
    
    for _iteration in 0..100 {
        let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
        perm.grant("admin".to_string(), None).unwrap();
        
        // Generate random offset from -1 year to +1 year
        let days_offset = rng.gen_range(-365..365);
        let expiration = Utc::now() + Duration::days(days_offset);
        perm.set_expiration(expiration);
        
        // Verify is_active() matches expectation
        let should_be_active = days_offset > 0;
        assert_eq!(perm.is_active(), should_be_active, 
            "is_active() should match expiration time (offset: {} days)", days_offset);
    }
}

/// Fuzz test: Random agent ID strings
#[test]
fn test_fuzz_random_agent_ids() {
    use rand::prelude::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    
    let mut rng = StdRng::seed_from_u64(42);
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()_+-=[]{}|;:,.<>?/~` ".chars().collect();
    
    for _iteration in 0..100 {
        // Generate random agent ID of length 0-1000
        let length = rng.gen_range(0..1000);
        let agent_id: String = (0..length)
            .map(|_| chars[rng.gen_range(0..chars.len())])
            .collect();
        
        let perm = Permission::new(agent_id.clone(), PermissionScope::DocumentRead);
        assert_eq!(perm.agent_id, agent_id);
        
        // Store and retrieve
        let storage = PermissionStorage::new();
        let stored_perm = perm.clone();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            storage.store(stored_perm).await.unwrap();
            let retrieved = storage.get_by_agent(&agent_id).await;
            assert_eq!(retrieved.len(), 1);
            assert_eq!(retrieved[0].agent_id, agent_id);
        });
    }
}

/// Fuzz test: Random scope types with invalid transitions
#[test]
fn test_fuzz_random_scope_types() {
    use rand::prelude::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    
    let mut rng = StdRng::seed_from_u64(42);
    
    for _iteration in 0..100 {
        // Generate random scope
        let scope = match rng.gen_range(0..7) {
            0 => PermissionScope::DocumentRead,
            1 => PermissionScope::DocumentWrite,
            2 => PermissionScope::PrivateMessage,
            3 => PermissionScope::Voting,
            4 => PermissionScope::Broadcast,
            5 => PermissionScope::Custom("custom".to_string()),
            _ => PermissionScope::Custom(format!("random_{}", rng.gen::<u32>())),
        };
        
        let mut perm = Permission::new("agent1".to_string(), scope.clone());
        
        // Verify all state transitions work correctly for this scope
        assert!(perm.grant("admin".to_string(), None).is_ok());
        assert_eq!(perm.status, PermissionStatus::Granted);
        
        assert!(perm.deny("admin".to_string(), None).is_ok());
        assert_eq!(perm.status, PermissionStatus::Denied);
        
        // Can't revoke denied permission
        assert!(perm.revoke("admin".to_string(), None).is_err());
        
        // Grant again
        assert!(perm.grant("admin".to_string(), None).is_ok());
        assert!(perm.revoke("admin".to_string(), None).is_ok());
        assert_eq!(perm.status, PermissionStatus::Revoked);
    }
}

/// Fuzz test: Concurrent permission requests from same agent
#[tokio::test]
async fn test_fuzz_concurrent_requests() {
    use rand::prelude::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    
    let storage = Arc::new(PermissionStorage::new());
    let mut handles = Vec::new();
    
    // Spawn 100 concurrent permission creation tasks
    for i in 0..100 {
        let storage_clone = Arc::clone(&storage);
        let handle = tokio::spawn(async move {
            let mut perm = Permission::new("agent1".to_string(), PermissionScope::Custom(format!("scope_{}", i)));
            perm.grant("admin".to_string(), None).unwrap();
            storage_clone.store(perm).await.unwrap();
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Verify all permissions were stored
    let agent_perms = storage.get_by_agent("agent1").await;
    assert_eq!(agent_perms.len(), 100, "All concurrent requests should succeed");
    
    // Verify no duplicates
    let ids: Vec<Uuid> = agent_perms.iter().map(|p| p.id).collect();
    let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique_ids.len(), 100, "All permission IDs should be unique");
}

// ============================================================================
// ADDITIONAL EDGE CASES
// ============================================================================

/// Test permission reason with very long string
#[test]
fn test_permission_reason_very_long() {
    let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
    let long_reason = "a".repeat(100000);
    
    perm.grant("admin".to_string(), Some(long_reason.clone())).unwrap();
    
    assert_eq!(perm.reason, Some(long_reason));
}

/// Test permission with None reason vs Some empty string
#[test]
fn test_permission_reason_none_vs_empty() {
    let mut perm1 = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
    perm1.grant("admin".to_string(), None).unwrap();
    
    let mut perm2 = Permission::new("agent2".to_string(), PermissionScope::DocumentRead);
    perm2.grant("admin".to_string(), Some("".to_string())).unwrap();
    
    assert_eq!(perm1.reason, None);
    assert_eq!(perm2.reason, Some("".to_string()));
}

/// Test clear_expired with mixed permissions
#[tokio::test]
async fn test_clear_expired_mixed_permissions() {
    let storage = PermissionStorage::new();
    
    // Create permissions with different expiration states
    let mut expired = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
    expired.grant("admin".to_string(), None).unwrap();
    expired.set_expiration(Utc::now() - Duration::hours(1));
    
    let mut active = Permission::new("agent2".to_string(), PermissionScope::DocumentRead);
    active.grant("admin".to_string(), None).unwrap();
    active.set_expiration(Utc::now() + Duration::hours(1));
    
    let mut no_expiration = Permission::new("agent3".to_string(), PermissionScope::DocumentRead);
    no_expiration.grant("admin".to_string(), None).unwrap();
    
    storage.store(expired).await.unwrap();
    storage.store(active).await.unwrap();
    storage.store(no_expiration).await.unwrap();
    
    // Clear expired
    let cleared_count = storage.clear_expired().await;
    
    assert_eq!(cleared_count, 1, "Should clear exactly one expired permission");
    
    // Verify only active and no_expiration remain
    assert_eq!(storage.get_all().await.len(), 2);
    assert!(storage.has_permission("agent2", &PermissionScope::DocumentRead).await);
    assert!(storage.has_permission("agent3", &PermissionScope::DocumentRead).await);
    assert!(!storage.has_permission("agent1", &PermissionScope::DocumentRead).await);
}

/// Test storage with maximum number of permissions
#[tokio::test]
async fn test_storage_large_number_of_permissions() {
    let storage = PermissionStorage::new();
    
    // Create 10000 permissions
    for i in 0..10000 {
        let mut perm = Permission::new(
            format!("agent_{}", i % 100),  // 100 different agents
            PermissionScope::Custom(format!("scope_{}", i)),
        );
        perm.grant("admin".to_string(), None).unwrap();
        storage.store(perm).await.unwrap();
    }
    
    // Verify count
    let all_perms = storage.get_all().await;
    assert_eq!(all_perms.len(), 10000);
    
    // Verify per-agent count
    for i in 0..100 {
        let agent_perms = storage.get_by_agent(&format!("agent_{}", i)).await;
        assert_eq!(agent_perms.len(), 100);  // 10000 / 100 agents
    }
}
