use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::metadata::{DocumentMetadata, AccessLevel};
use super::version::DocumentVersion;

/// Error types for document storage operations
#[derive(Debug, thiserror::Error)]
pub enum DocumentStorageError {
    #[error("Document not found: {0}")]
    NotFound(Uuid),
    #[error("Version not found: {0}")]
    VersionNotFound(u32),
    #[error("Access denied")]
    AccessDenied,
    #[error("Document is archived")]
    DocumentArchived,
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// In-memory document storage with version control
/// In production, this would be backed by a file system or database
#[derive(Clone)]
pub struct DocumentStorage {
    documents: Arc<RwLock<HashMap<Uuid, DocumentMetadata>>>,
    versions: Arc<RwLock<HashMap<Uuid, Vec<DocumentVersion>>>>, // doc_id -> versions
    storage_path: PathBuf,
}

impl DocumentStorage {
    /// Create a new document storage instance
    pub fn new(storage_path: Option<PathBuf>) -> Self {
        let storage_path = storage_path.unwrap_or_else(|| PathBuf::from("./documents"));
        
        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
            versions: Arc::new(RwLock::new(HashMap::new())),
            storage_path,
        }
    }

    /// Create a new document with initial content
    pub async fn create_document(
        &self,
        title: String,
        created_by: String,
        initial_content: String,
    ) -> Result<Uuid, DocumentStorageError> {
        let doc_id = Uuid::new_v4();
        
        let mut metadata = DocumentMetadata::new(doc_id, title, created_by.clone());
        metadata.increment_version(); // First version
        
        let version = DocumentVersion::new(1, created_by, initial_content);
        
        // Store metadata and version
        {
            let mut docs = self.documents.write().await;
            docs.insert(doc_id, metadata);
        }
        
        {
            let mut vers = self.versions.write().await;
            vers.entry(doc_id).or_insert_with(Vec::new).push(version);
        }
        
        Ok(doc_id)
    }

    /// Get document metadata
    pub async fn get_metadata(&self, doc_id: Uuid) -> Result<DocumentMetadata, DocumentStorageError> {
        let docs = self.documents.read().await;
        docs.get(&doc_id)
            .cloned()
            .ok_or(DocumentStorageError::NotFound(doc_id))
    }

    /// Get a specific version of a document
    pub async fn get_version(
        &self,
        doc_id: Uuid,
        version_number: u32,
    ) -> Result<DocumentVersion, DocumentStorageError> {
        let vers = self.versions.read().await;
        let versions = vers.get(&doc_id)
            .ok_or(DocumentStorageError::NotFound(doc_id))?;
        
        // Find version (index is version_number - 1)
        versions.get((version_number - 1) as usize)
            .cloned()
            .ok_or(DocumentStorageError::VersionNotFound(version_number))
    }

    /// Get the latest version of a document
    pub async fn get_latest_version(&self, doc_id: Uuid) -> Result<DocumentVersion, DocumentStorageError> {
        let metadata = self.get_metadata(doc_id).await?;
        self.get_version(doc_id, metadata.current_version).await
    }

    /// Update a document with new content
    pub async fn update_document(
        &self,
        doc_id: Uuid,
        modified_by: String,
        new_content: String,
        commit_message: Option<String>,
    ) -> Result<u32, DocumentStorageError> {
        let mut docs = self.documents.write().await;
        let metadata = docs.get_mut(&doc_id)
            .ok_or(DocumentStorageError::NotFound(doc_id))?;
        
        if !metadata.is_active() {
            return Err(DocumentStorageError::DocumentArchived);
        }
        
        // Increment version
        metadata.touch(modified_by.clone());
        metadata.increment_version();
        let new_version_number = metadata.current_version;
        
        drop(docs);
        
        // Create new version
        let mut version = DocumentVersion::new(new_version_number, modified_by, new_content);
        if let Some(msg) = commit_message {
            version = version.with_message(msg);
        }
        
        // Store version
        let mut vers = self.versions.write().await;
        vers.entry(doc_id).or_insert_with(Vec::new).push(version);
        
        Ok(new_version_number)
    }

    /// Get all versions of a document
    pub async fn get_all_versions(&self, doc_id: Uuid) -> Result<Vec<DocumentVersion>, DocumentStorageError> {
        let vers = self.versions.read().await;
        vers.get(&doc_id)
            .cloned()
            .ok_or(DocumentStorageError::NotFound(doc_id))
    }

    /// Compare two versions of a document
    pub async fn compare_versions(
        &self,
        doc_id: Uuid,
        version_a: u32,
        version_b: u32,
    ) -> Result<VersionComparison, DocumentStorageError> {
        let ver_a = self.get_version(doc_id, version_a).await?;
        let ver_b = self.get_version(doc_id, version_b).await?;
        
        Ok(VersionComparison {
            version_a: ver_a.info,
            version_b: ver_b.info,
            content_changed: ver_a.content != ver_b.content,
            hash_changed: ver_a.hash() != ver_b.hash(),
        })
    }

    /// Delete a document and all its versions
    pub async fn delete_document(&self, doc_id: Uuid) -> Result<(), DocumentStorageError> {
        {
            let mut docs = self.documents.write().await;
            docs.remove(&doc_id)
                .ok_or(DocumentStorageError::NotFound(doc_id))?;
        }
        
        {
            let mut vers = self.versions.write().await;
            vers.remove(&doc_id);
        }
        
        Ok(())
    }

    /// Archive a document
    pub async fn archive_document(&self, doc_id: Uuid, modified_by: String) -> Result<(), DocumentStorageError> {
        let mut docs = self.documents.write().await;
        let metadata = docs.get_mut(&doc_id)
            .ok_or(DocumentStorageError::NotFound(doc_id))?;
        
        metadata.archive();
        Ok(())
    }

    /// Unarchive a document
    pub async fn unarchive_document(&self, doc_id: Uuid, modified_by: String) -> Result<(), DocumentStorageError> {
        let mut docs = self.documents.write().await;
        let metadata = docs.get_mut(&doc_id)
            .ok_or(DocumentStorageError::NotFound(doc_id))?;
        
        metadata.unarchive();
        Ok(())
    }

    /// List all active documents
    pub async fn list_documents(&self) -> Vec<DocumentMetadata> {
        let docs = self.documents.read().await;
        docs.values()
            .filter(|d| d.is_active())
            .cloned()
            .collect()
    }

    /// Search documents by title or tags
    pub async fn search_documents(&self, query: &str) -> Vec<DocumentMetadata> {
        let docs = self.documents.read().await;
        let query_lower = query.to_lowercase();
        
        docs.values()
            .filter(|d| {
                d.is_active() && (
                    d.title.to_lowercase().contains(&query_lower) ||
                    d.tags.iter().any(|t| t.to_lowercase().contains(&query_lower)) ||
                    d.description.as_ref().map_or(false, |desc| desc.to_lowercase().contains(&query_lower))
                )
            })
            .cloned()
            .collect()
    }

    /// Get the storage path
    pub fn storage_path(&self) -> &PathBuf {
        &self.storage_path
    }

    /// Generate a link/URI for a document
    pub fn generate_document_link(&self, doc_id: Uuid) -> String {
        format!("document://{}", doc_id)
    }

    /// Generate a link/URI for a specific document version
    pub fn generate_version_link(&self, doc_id: Uuid, version: u32) -> String {
        format!("document://{}?version={}", doc_id, version)
    }
}

impl Default for DocumentStorage {
    fn default() -> Self {
        Self::new(None)
    }
}

/// Result of comparing two document versions
#[derive(Debug, Clone)]
pub struct VersionComparison {
    pub version_a: super::version::VersionInfo,
    pub version_b: super::version::VersionInfo,
    pub content_changed: bool,
    pub hash_changed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_document() {
        let storage = DocumentStorage::new(None);
        
        let doc_id = storage.create_document(
            "Test Doc".to_string(),
            "agent1".to_string(),
            "Initial content".to_string(),
        ).await.unwrap();
        
        let metadata = storage.get_metadata(doc_id).await.unwrap();
        assert_eq!(metadata.title, "Test Doc");
        assert_eq!(metadata.current_version, 1);
    }

    #[tokio::test]
    async fn test_update_document() {
        let storage = DocumentStorage::new(None);
        
        let doc_id = storage.create_document(
            "Test Doc".to_string(),
            "agent1".to_string(),
            "Initial content".to_string(),
        ).await.unwrap();
        
        let new_version = storage.update_document(
            doc_id,
            "agent1".to_string(),
            "Updated content".to_string(),
            Some("First update".to_string()),
        ).await.unwrap();
        
        assert_eq!(new_version, 2);
        
        let metadata = storage.get_metadata(doc_id).await.unwrap();
        assert_eq!(metadata.current_version, 2);
        
        let version = storage.get_version(doc_id, 2).await.unwrap();
        assert_eq!(version.content, "Updated content");
        assert_eq!(version.info.message, Some("First update".to_string()));
    }

    #[tokio::test]
    async fn test_get_all_versions() {
        let storage = DocumentStorage::new(None);
        
        let doc_id = storage.create_document(
            "Test".to_string(),
            "agent1".to_string(),
            "v1".to_string(),
        ).await.unwrap();
        
        storage.update_document(doc_id, "agent1".to_string(), "v2".to_string(), None).await.unwrap();
        storage.update_document(doc_id, "agent1".to_string(), "v3".to_string(), None).await.unwrap();
        
        let versions = storage.get_all_versions(doc_id).await.unwrap();
        assert_eq!(versions.len(), 3);
    }

    #[tokio::test]
    async fn test_search_documents() {
        let storage = DocumentStorage::new(None);
        
        storage.create_document("Rust Guide".to_string(), "agent1".to_string(), "Content".to_string()).await.unwrap();
        storage.create_document("Python Basics".to_string(), "agent1".to_string(), "Content".to_string()).await.unwrap();
        
        let results = storage.search_documents("rust").await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Guide");
    }

    #[tokio::test]
    async fn test_archive_unarchive() {
        let storage = DocumentStorage::new(None);
        
        let doc_id = storage.create_document(
            "Test".to_string(),
            "agent1".to_string(),
            "Content".to_string(),
        ).await.unwrap();
        
        storage.archive_document(doc_id, "agent1".to_string()).await.unwrap();
        
        let docs = storage.list_documents().await;
        assert!(docs.is_empty()); // Archived documents not in list
        
        storage.unarchive_document(doc_id, "agent1".to_string()).await.unwrap();
        
        let docs = storage.list_documents().await;
        assert_eq!(docs.len(), 1);
    }
}
