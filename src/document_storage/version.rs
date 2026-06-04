use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Information about a document version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Version number (starts at 1)
    pub version_number: u32,
    /// When this version was created
    pub created_at: DateTime<Utc>,
    /// Who created this version
    pub created_by: String,
    /// Optional commit message
    pub message: Option<String>,
    /// SHA256 hash of the content
    pub content_hash: String,
}

/// Represents a specific version of a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentVersion {
    /// The version info
    pub info: VersionInfo,
    /// The content of this version
    pub content: String,
    /// Size in bytes
    pub size_bytes: usize,
}

impl DocumentVersion {
    /// Create a new document version
    pub fn new(version_number: u32, created_by: String, content: String) -> Self {
        let content_hash = sha256_content(&content);
        let size_bytes = content.len();
        
        Self {
            info: VersionInfo {
                version_number,
                created_at: Utc::now(),
                created_by,
                message: None,
                content_hash,
            },
            content,
            size_bytes,
        }
    }

    /// Set a commit message for this version
    pub fn with_message(mut self, message: String) -> Self {
        self.info.message = Some(message);
        self
    }

    /// Get the content hash
    pub fn hash(&self) -> &str {
        &self.info.content_hash
    }
}

/// Compute SHA256 hash of content
fn sha256_content(content: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_creation() {
        let version = DocumentVersion::new(
            1,
            "agent1".to_string(),
            "Hello, world!".to_string(),
        );
        
        assert_eq!(version.info.version_number, 1);
        assert_eq!(version.info.created_by, "agent1");
        assert!(!version.info.content_hash.is_empty());
    }

    #[test]
    fn test_version_with_message() {
        let version = DocumentVersion::new(
            1,
            "agent1".to_string(),
            "Content".to_string(),
        ).with_message("Initial commit".to_string());
        
        assert_eq!(version.info.message, Some("Initial commit".to_string()));
    }

    #[test]
    fn test_content_hash_consistency() {
        let content = "Test content";
        let v1 = DocumentVersion::new(1, "agent1".to_string(), content.to_string());
        let v2 = DocumentVersion::new(1, "agent1".to_string(), content.to_string());
        
        assert_eq!(v1.hash(), v2.hash());
    }

    #[test]
    fn test_different_content_hashes() {
        let v1 = DocumentVersion::new(1, "agent1".to_string(), "Content A".to_string());
        let v2 = DocumentVersion::new(1, "agent1".to_string(), "Content B".to_string());
        
        assert_ne!(v1.hash(), v2.hash());
    }
}
