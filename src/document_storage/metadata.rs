use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Metadata associated with a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Unique identifier for the document
    pub id: Uuid,
    /// Title of the document
    pub title: String,
    /// Optional description
    pub description: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// When the document was created
    pub created_at: DateTime<Utc>,
    /// When the document was last modified
    pub modified_at: DateTime<Utc>,
    /// Who created the document
    pub created_by: String,
    /// Who last modified the document
    pub modified_by: String,
    /// Current version number
    pub current_version: u32,
    /// Whether the document is archived
    pub archived: bool,
    /// Access level (public, private, restricted)
    pub access_level: AccessLevel,
}

/// Access level for documents
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccessLevel {
    /// Anyone can read
    Public,
    /// Only participants can read
    Private,
    /// Requires specific permission
    Restricted,
}

impl Default for AccessLevel {
    fn default() -> Self {
        AccessLevel::Private
    }
}

impl DocumentMetadata {
    /// Create new metadata for a document
    pub fn new(id: Uuid, title: String, created_by: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            description: None,
            tags: Vec::new(),
            created_at: now,
            modified_at: now,
            created_by,
            modified_by: created_by,
            current_version: 0,
            archived: false,
            access_level: AccessLevel::Private,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Remove a tag
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
    }

    /// Update the modified timestamp and user
    pub fn touch(&mut self, modified_by: String) {
        self.modified_at = Utc::now();
        self.modified_by = modified_by;
    }

    /// Increment the version number
    pub fn increment_version(&mut self) {
        self.current_version += 1;
        self.touch(self.modified_by.clone());
    }

    /// Set the access level
    pub fn set_access_level(&mut self, level: AccessLevel) {
        self.access_level = level;
        self.touch(self.modified_by.clone());
    }

    /// Archive the document
    pub fn archive(&mut self) {
        self.archived = true;
        self.touch(self.modified_by.clone());
    }

    /// Unarchive the document
    pub fn unarchive(&mut self) {
        self.archived = false;
        self.touch(self.modified_by.clone());
    }

    /// Check if document is active (not archived)
    pub fn is_active(&self) -> bool {
        !self.archived
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_creation() {
        let meta = DocumentMetadata::new(
            Uuid::new_v4(),
            "Test Document".to_string(),
            "agent1".to_string(),
        );
        
        assert_eq!(meta.title, "Test Document");
        assert_eq!(meta.created_by, "agent1");
        assert_eq!(meta.current_version, 0);
        assert!(!meta.archived);
    }

    #[test]
    fn test_add_remove_tags() {
        let mut meta = DocumentMetadata::new(
            Uuid::new_v4(),
            "Test".to_string(),
            "agent1".to_string(),
        );
        
        meta.add_tag("rust".to_string());
        meta.add_tag("programming".to_string());
        meta.add_tag("rust".to_string()); // Duplicate
        
        assert_eq!(meta.tags.len(), 2);
        assert!(meta.tags.contains(&"rust".to_string()));
        
        meta.remove_tag("rust");
        assert_eq!(meta.tags.len(), 1);
        assert!(!meta.tags.contains(&"rust".to_string()));
    }

    #[test]
    fn test_version_increment() {
        let mut meta = DocumentMetadata::new(
            Uuid::new_v4(),
            "Test".to_string(),
            "agent1".to_string(),
        );
        
        meta.increment_version();
        assert_eq!(meta.current_version, 1);
        
        meta.increment_version();
        assert_eq!(meta.current_version, 2);
    }

    #[test]
    fn test_archive_unarchive() {
        let mut meta = DocumentMetadata::new(
            Uuid::new_v4(),
            "Test".to_string(),
            "agent1".to_string(),
        );
        
        assert!(meta.is_active());
        
        meta.archive();
        assert!(!meta.is_active());
        assert!(meta.archived);
        
        meta.unarchive();
        assert!(meta.is_active());
        assert!(!meta.archived);
    }

    #[test]
    fn test_access_level() {
        let mut meta = DocumentMetadata::new(
            Uuid::new_v4(),
            "Test".to_string(),
            "agent1".to_string(),
        );
        
        assert_eq!(meta.access_level, AccessLevel::Private);
        
        meta.set_access_level(AccessLevel::Public);
        assert_eq!(meta.access_level, AccessLevel::Public);
        
        meta.set_access_level(AccessLevel::Restricted);
        assert_eq!(meta.access_level, AccessLevel::Restricted);
    }
}
