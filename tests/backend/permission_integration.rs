// Integration tests for permission system
use multi_agent_chat::permission::{Permission, PermissionScope, PermissionStatus, PermissionManager};
use multi_agent_chat::mcp_server::session::AgentSession;
use tokio;

#[tokio::test]
async fn test_permission_request_approval_workflow() {
    let manager = PermissionManager::new();
    
    // Create a permission request
    let scope = PermissionScope::Channel("general".to_string());
    let permission = Permission::Read;
    
    let request_id = manager.request_permission(
        "agent_1".to_string(),
        scope.clone(),
        permission,
        "Testing access".to_string(),
    ).await.unwrap();
    
    // Verify initial status is pending
    let status = manager.get_permission_status(request_id).await.unwrap();
    assert_eq!(status, PermissionStatus::Pending);
    
    // Approve the permission
    manager.approve_permission(request_id, "admin".to_string()).await.unwrap();
    
    // Verify status changed to approved
    let status = manager.get_permission_status(request_id).await.unwrap();
    assert_eq!(status, PermissionStatus::Approved);
}

#[tokio::test]
async fn test_permission_expiration() {
    let manager = PermissionManager::new();
    
    let scope = PermissionScope::Global;
    let permission = Permission::Write;
    
    // Create permission with 1 second expiration
    let request_id = manager.request_permission_with_expiry(
        "agent_2".to_string(),
        scope.clone(),
        permission,
        "Temporary access".to_string(),
        1000, // 1 second
    ).await.unwrap();
    
    manager.approve_permission(request_id, "admin".to_string()).await.unwrap();
    
    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;
    
    // Verify permission has expired
    let has_permission = manager.check_permission("agent_2", scope, permission).await;
    assert!(!has_permission);
}

#[tokio::test]
async fn test_multiple_agents_permissions() {
    let manager = PermissionManager::new();
    
    // Grant different permissions to multiple agents
    let scope = PermissionScope::Channel("private".to_string());
    
    let req1 = manager.request_permission(
        "agent_a".to_string(),
        scope.clone(),
        Permission::Read,
        "Access request".to_string(),
    ).await.unwrap();
    
    let req2 = manager.request_permission(
        "agent_b".to_string(),
        scope.clone(),
        Permission::Write,
        "Access request".to_string(),
    ).await.unwrap();
    
    manager.approve_permission(req1, "admin".to_string()).await.unwrap();
    manager.approve_permission(req2, "admin".to_string()).await.unwrap();
    
    // Verify both agents have their respective permissions
    assert!(manager.check_permission("agent_a", scope.clone(), Permission::Read).await);
    assert!(!manager.check_permission("agent_a", scope.clone(), Permission::Write).await);
    
    assert!(manager.check_permission("agent_b", scope.clone(), Permission::Write).await);
    assert!(manager.check_permission("agent_b", scope.clone(), Permission::Read).await); // Write implies Read
}

#[tokio::test]
async fn test_permission_denial_and_revoke() {
    let manager = PermissionManager::new();
    
    let scope = PermissionScope::Global;
    let permission = Permission::Admin;
    
    let request_id = manager.request_permission(
        "agent_3".to_string(),
        scope.clone(),
        permission,
        "Admin access".to_string(),
    ).await.unwrap();
    
    // Deny the permission
    manager.deny_permission(request_id, "security_policy".to_string()).await.unwrap();
    
    let status = manager.get_permission_status(request_id).await.unwrap();
    assert_eq!(status, PermissionStatus::Denied);
    
    // Request again and approve
    let request_id2 = manager.request_permission(
        "agent_3".to_string(),
        scope.clone(),
        permission,
        "Admin access retry".to_string(),
    ).await.unwrap();
    
    manager.approve_permission(request_id2, "admin".to_string()).await.unwrap();
    
    // Revoke the permission
    manager.revoke_permission("agent_3".to_string(), scope.clone(), permission)
        .await.unwrap();
    
    // Verify permission is revoked
    assert!(!manager.check_permission("agent_3", scope, permission).await);
}
