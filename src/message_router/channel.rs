use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Represents the type of channel
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ChannelType {
    /// General chat channel for broadcast messages
    General,
    /// Private message between two agents
    Private(String, String), // agent1_id, agent2_id
    /// Topic-specific channel
    Topic(String),
    /// Document collaboration channel
    Document(Uuid),
}

/// Represents a communication channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    /// Unique identifier for the channel
    pub id: Uuid,
    /// Type of the channel
    pub channel_type: ChannelType,
    /// Display name for the channel
    pub name: String,
    /// List of agent IDs that can access this channel
    pub participants: Vec<String>,
    /// When the channel was created
    pub created_at: DateTime<Utc>,
    /// Whether the channel is active
    pub active: bool,
}

impl Channel {
    /// Create a new general channel
    pub fn new_general(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            channel_type: ChannelType::General,
            name,
            participants: Vec::new(),
            created_at: Utc::now(),
            active: true,
        }
    }

    /// Create a new private channel between two agents
    pub fn new_private(agent1_id: String, agent2_id: String) -> Self {
        let mut participants = vec![agent1_id.clone(), agent2_id.clone()];
        participants.sort(); // Ensure consistent ordering
        
        Self {
            id: Uuid::new_v4(),
            channel_type: ChannelType::Private(agent1_id, agent2_id),
            name: format!("PM: {} <-> {}", agent1_id, agent2_id),
            participants,
            created_at: Utc::now(),
            active: true,
        }
    }

    /// Create a new topic channel
    pub fn new_topic(topic: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            channel_type: ChannelType::Topic(topic.clone()),
            name: format!("#{}", topic),
            participants: Vec::new(),
            created_at: Utc::now(),
            active: true,
        }
    }

    /// Create a new document channel
    pub fn new_document(doc_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            channel_type: ChannelType::Document(doc_id),
            name: format!("Document: {}", doc_id),
            participants: Vec::new(),
            created_at: Utc::now(),
            active: true,
        }
    }

    /// Add a participant to the channel
    pub fn add_participant(&mut self, agent_id: String) {
        if !self.participants.contains(&agent_id) {
            self.participants.push(agent_id);
        }
    }

    /// Remove a participant from the channel
    pub fn remove_participant(&mut self, agent_id: &str) {
        self.participants.retain(|id| id != agent_id);
    }

    /// Check if an agent is a participant in this channel
    pub fn has_participant(&self, agent_id: &str) -> bool {
        self.participants.contains(&agent_id)
    }

    /// Get the canonical ID for a private channel between two agents
    pub fn get_private_channel_id(agent1_id: &str, agent2_id: &str) -> String {
        let mut ids = vec![agent1_id.to_string(), agent2_id.to_string()];
        ids.sort();
        format!("{}:{}", ids[0], ids[1])
    }
}

/// Trait for handling channel-specific message processing
#[async_trait::async_trait]
pub trait ChannelHandler: Send + Sync {
    /// Process a message sent to this channel
    async fn handle_message(&self, message: &ChatMessage) -> Result<(), ChannelError>;
    
    /// Called when a participant joins the channel
    async fn on_join(&self, agent_id: &str) -> Result<(), ChannelError>;
    
    /// Called when a participant leaves the channel
    async fn on_leave(&self, agent_id: &str) -> Result<(), ChannelError>;
}

/// Errors that can occur in channel operations
#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("Channel not found")]
    NotFound,
    #[error("Agent not authorized for this channel")]
    Unauthorized,
    #[error("Channel is inactive")]
    Inactive,
    #[error("Invalid channel operation: {0}")]
    InvalidOperation(String),
    #[error("Handler error: {0}")]
    HandlerError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_general_channel_creation() {
        let channel = Channel::new_general("Main Chat".to_string());
        assert_eq!(channel.channel_type, ChannelType::General);
        assert_eq!(channel.name, "Main Chat");
        assert!(channel.active);
    }

    #[test]
    fn test_private_channel_creation() {
        let channel = Channel::new_private("agent1".to_string(), "agent2".to_string());
        match channel.channel_type {
            ChannelType::Private(ref a1, ref a2) => {
                assert_eq!(a1, "agent1");
                assert_eq!(a2, "agent2");
            }
            _ => panic!("Expected Private channel type"),
        }
        assert!(channel.has_participant("agent1"));
        assert!(channel.has_participant("agent2"));
    }

    #[test]
    fn test_private_channel_id_consistency() {
        let id1 = Channel::get_private_channel_id("agent1", "agent2");
        let id2 = Channel::get_private_channel_id("agent2", "agent1");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_add_remove_participant() {
        let mut channel = Channel::new_topic("testing".to_string());
        
        channel.add_participant("agent1".to_string());
        assert!(channel.has_participant("agent1"));
        
        channel.remove_participant("agent1");
        assert!(!channel.has_participant("agent1"));
    }
}
