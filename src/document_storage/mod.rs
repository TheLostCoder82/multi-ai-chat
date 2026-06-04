//! Document Storage module for managing documents and versions
pub mod storage;
pub mod version;
pub mod metadata;

pub use storage::DocumentStorage;
pub use version::{DocumentVersion, VersionInfo};
pub use metadata::DocumentMetadata;
