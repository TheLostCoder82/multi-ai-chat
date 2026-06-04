// Integration tests for document storage
use multi_agent_chat::document_storage::{DocumentStorage, Document, DocumentVersion};
use tokio;

#[tokio::test]
async fn test_document_crud_operations() {
    let storage = DocumentStorage::new();
    
    // Create a document
    let doc_id = storage.create_document(
        "Test Document".to_string(),
        "author_1".to_string(),
        "Initial content".to_string(),
        Some(vec!["tag1".to_string(), "tag2".to_string()]),
    ).await.unwrap();
    
    // Retrieve the document
    let doc = storage.get_document(&doc_id).await.unwrap();
    assert_eq!(doc.title, "Test Document");
    assert_eq!(doc.content, "Initial content");
    assert_eq!(doc.author, "author_1");
    
    // Update the document
    storage.update_document(&doc_id, "Updated content".to_string(), "author_1".to_string())
        .await.unwrap();
    
    // Verify update created new version
    let updated_doc = storage.get_document(&doc_id).await.unwrap();
    assert_eq!(updated_doc.content, "Updated content");
    assert_eq!(updated_doc.version, 2);
    
    // Delete the document
    storage.delete_document(&doc_id).await.unwrap();
    
    // Verify deletion
    let result = storage.get_document(&doc_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_version_history_and_comparison() {
    let storage = DocumentStorage::new();
    
    let doc_id = storage.create_document(
        "Version Test".to_string(),
        "author_1".to_string(),
        "Version 1 content".to_string(),
        None,
    ).await.unwrap();
    
    // Create multiple versions
    storage.update_document(&doc_id, "Version 2 content".to_string(), "author_1".to_string())
        .await.unwrap();
    storage.update_document(&doc_id, "Version 3 content".to_string(), "author_1".to_string())
        .await.unwrap();
    
    // Get version history
    let history = storage.get_version_history(&doc_id).await.unwrap();
    assert_eq!(history.len(), 3);
    
    // Compare versions
    let diff = storage.compare_versions(&doc_id, 1, 3).await.unwrap();
    assert!(!diff.is_empty());
    
    // Restore to previous version
    storage.restore_to_version(&doc_id, 1).await.unwrap();
    
    let restored_doc = storage.get_document(&doc_id).await.unwrap();
    assert_eq!(restored_doc.content, "Version 1 content");
    assert_eq!(restored_doc.version, 4); // New version created on restore
}

#[tokio::test]
async fn test_document_search() {
    let storage = DocumentStorage::new();
    
    // Create multiple documents
    storage.create_document(
        "Rust Programming Guide".to_string(),
        "author_1".to_string(),
        "Learn Rust programming basics".to_string(),
        Some(vec!["rust".to_string(), "programming".to_string()]),
    ).await.unwrap();
    
    storage.create_document(
        "Python Best Practices".to_string(),
        "author_2".to_string(),
        "Python coding standards and patterns".to_string(),
        Some(vec!["python".to_string(), "best-practices".to_string()]),
    ).await.unwrap();
    
    storage.create_document(
        "Web Development with Rust".to_string(),
        "author_1".to_string(),
        "Building web applications using Rust".to_string(),
        Some(vec!["rust".to_string(), "web".to_string()]),
    ).await.unwrap();
    
    // Search by title
    let results = storage.search_documents("Rust", None, None).await.unwrap();
    assert_eq!(results.len(), 2);
    
    // Search by tag
    let results = storage.search_documents(None, Some("rust".to_string()), None).await.unwrap();
    assert_eq!(results.len(), 2);
    
    // Search by author
    let results = storage.search_documents(None, None, Some("author_1".to_string())).await.unwrap();
    assert_eq!(results.len(), 2);
    
    // Combined search
    let results = storage.search_documents(
        Some("Web"),
        Some("rust".to_string()),
        Some("author_1".to_string()),
    ).await.unwrap();
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn test_document_metadata_and_linking() {
    let storage = DocumentStorage::new();
    
    let doc_id = storage.create_document(
        "Main Document".to_string(),
        "author_1".to_string(),
        "Content with references".to_string(),
        None,
    ).await.unwrap();
    
    // Add metadata
    storage.add_metadata(&doc_id, "project".to_string(), serde_json::json!("ChatApp"))
        .await.unwrap();
    storage.add_metadata(&doc_id, "status".to_string(), serde_json::json!("draft"))
        .await.unwrap();
    
    // Create linked documents
    let linked_doc_id = storage.create_document(
        "Linked Document".to_string(),
        "author_1".to_string(),
        "Related content".to_string(),
        None,
    ).await.unwrap();
    
    // Create link between documents
    storage.link_documents(&doc_id, &linked_doc_id, "references".to_string())
        .await.unwrap();
    
    // Get linked documents
    let links = storage.get_linked_documents(&doc_id).await.unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target_id, linked_doc_id);
    
    // Generate file path/link
    let link = storage.generate_document_link(&doc_id).await;
    assert!(link.contains(&doc_id));
}

#[tokio::test]
async fn test_concurrent_document_access() {
    let storage = DocumentStorage::new();
    
    let doc_id = storage.create_document(
        "Concurrent Test".to_string(),
        "author_1".to_string(),
        "Initial".to_string(),
        None,
    ).await.unwrap();
    
    // Simulate concurrent updates
    let mut handles = vec![];
    
    for i in 0..5 {
        let storage_clone = storage.clone();
        let doc_id_clone = doc_id.clone();
        
        let handle = tokio::spawn(async move {
            storage_clone.update_document(
                &doc_id_clone,
                format!("Update from task {}", i),
                format!("author_{}", i),
            ).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all updates to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
    
    // Verify final state
    let doc = storage.get_document(&doc_id).await.unwrap();
    assert!(doc.version >= 6); // Initial + 5 updates
}

#[tokio::test]
async fn test_external_editor_integration() {
    let storage = DocumentStorage::new();
    
    let doc_id = storage.create_document(
        "External Edit Test".to_string(),
        "author_1".to_string(),
        "Original content".to_string(),
        None,
    ).await.unwrap();
    
    // Export document for external editing
    let export_path = storage.export_for_external_edit(&doc_id).await.unwrap();
    assert!(!export_path.is_empty());
    
    // Simulate external edit (in real scenario, file would be edited externally)
    // For test, we'll just import back immediately
    
    // Import edited document
    let new_version = storage.import_from_external_edit(&doc_id, &export_path)
        .await.unwrap();
    
    assert!(new_version > 1);
    
    // Cleanup: remove exported file
    std::fs::remove_file(&export_path).ok();
}
