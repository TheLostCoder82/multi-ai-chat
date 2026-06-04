use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

use super::channel::{Channel, ChannelType, ChannelHandler, ChannelError};
use super::queue::{ChatMessage, MessageQueue, MessagePriority, MessageQueueError};

/// Main message router for managing channels and routing messages
#[derive(Clone)]
pub struct MessageRouter {
    channels: Arc<RwLock<HashMap<Uuid, Channel>>>,
    private_channels: Arc<RwLock<HashMap<String, Uuid>>>, // canonical_id -> channel_id
    handlers: Arc<RwLock<HashMap<Uuid, Box<dyn ChannelHandler>>>>,
    message_queue: MessageQueue,
    broadcast_tx: broadcast::Sender<ChatMessage>,
}

impl MessageRouter {
    /// Create a new message router
    pub fn new(queue_capacity: usize, broadcast_buffer: usize) -> Self {
        let (broadcast_tx, _) = broadcast::channel(broadcast_buffer);
        
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            private_channels: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            message_queue: MessageQueue::new(queue_capacity),
            broadcast_tx,
        }
    }

    /// Create or get the general channel
    pub async fn get_or_create_general_channel(&self, name: String) -> Uuid {
        let mut channels = self.channels.write().await;
        
        // Check if a general channel with this name exists
        for channel in channels.values() {
            if channel.channel_type == ChannelType::General && channel.name == name {
                return channel.id;
            }
        }
        
        // Create new general channel
        let channel = Channel::new_general(name);
        let id = channel.id;
        channels.insert(id, channel);
        id
    }

    /// Create or get a private channel between two agents
    pub async fn get_or_create_private_channel(&self, agent1_id: String, agent2_id: String) -> Uuid {
        let canonical_id = Channel::get_private_channel_id(&agent1_id, &agent2_id);
        
        // Check if private channel already exists
        {
            let private_channels = self.private_channels.read().await;
            if let Some(channel_id) = private_channels.get(&canonical_id) {
                return *channel_id;
            }
        }
        
        // Create new private channel
        let mut channel = Channel::new_private(agent1_id.clone(), agent2_id.clone());
        let channel_id = channel.id;
        
        // Store in both maps
        {
            let mut channels = self.channels.write().await;
            let mut private_channels = self.private_channels.write().await;
            
            channel.add_participant(agent1_id);
            channel.add_participant(agent2_id);
            
            channels.insert(channel_id, channel);
            private_channels.insert(canonical_id, channel_id);
        }
        
        channel_id
    }

    /// Create a topic channel
    pub async fn create_topic_channel(&self, topic: String) -> Uuid {
        let mut channel = Channel::new_topic(topic);
        let id = channel.id;
        
        self.channels.write().await.insert(id, channel);
        id
    }

    /// Create a document channel
    pub async fn create_document_channel(&self, doc_id: Uuid) -> Uuid {
        let mut channel = Channel::new_document(doc_id);
        let id = channel.id;
        
        self.channels.write().await.insert(id, channel);
        id
    }

    /// Send a message to a channel
    pub async fn send_message(
        &self,
        sender_id: String,
        channel_id: Uuid,
        content: String,
        priority: MessagePriority,
    ) -> Result<ChatMessage, RouterError> {
        // Verify channel exists and is active
        let channels = self.channels.read().await;
        let channel = channels.get(&channel_id)
            .ok_or(RouterError::ChannelNotFound)?;
        
        if !channel.active {
            return Err(RouterError::ChannelInactive);
        }
        
        // Verify sender is a participant (except for general channels)
        if channel.channel_type != ChannelType::General && !channel.has_participant(&sender_id) {
            return Err(RouterError::Unauthorized);
        }
        
        drop(channels);
        
        // Create the message
        let mut message = ChatMessage::new(sender_id, channel_id, content);
        
        // Handle channel-specific processing
        if let Some(handler) = self.handlers.read().await.get(&channel_id) {
            handler.handle_message(&message).await
                .map_err(|e| RouterError::HandlerError(e.to_string()))?;
        }
        
        // Enqueue the message
        self.message_queue.enqueue_with_priority(message.clone(), priority).await?;
        
        // Broadcast to subscribers
        let _ = self.broadcast_tx.send(message.clone());
        
        Ok(message)
    }

    /// Add a participant to a channel
    pub async fn join_channel(&self, channel_id: Uuid, agent_id: String) -> Result<(), RouterError> {
        let mut channels = self.channels.write().await;
        let channel = channels.get_mut(&channel_id)
            .ok_or(RouterError::ChannelNotFound)?;
        
        if !channel.active {
            return Err(RouterError::ChannelInactive);
        }
        
        channel.add_participant(agent_id.clone());
        
        // Notify handler if exists
        if let Some(handler) = self.handlers.read().await.get(&channel_id) {
            handler.on_join(&agent_id).await
                .map_err(|e| RouterError::HandlerError(e.to_string()))?;
        }
        
        Ok(())
    }

    /// Remove a participant from a channel
    pub async fn leave_channel(&self, channel_id: Uuid, agent_id: &str) -> Result<(), RouterError> {
        let mut channels = self.channels.write().await;
        let channel = channels.get_mut(&channel_id)
            .ok_or(RouterError::ChannelNotFound)?;
        
        channel.remove_participant(agent_id);
        
        // Notify handler if exists
        if let Some(handler) = self.handlers.read().await.get(&channel_id) {
            handler.on_leave(agent_id).await
                .map_err(|e| RouterError::HandlerError(e.to_string()))?;
        }
        
        Ok(())
    }

    /// Register a channel handler
    pub async fn register_handler(&self, channel_id: Uuid, handler: Box<dyn ChannelHandler>) {
        self.handlers.write().await.insert(channel_id, handler);
    }

    /// Get a channel by ID
    pub async fn get_channel(&self, channel_id: Uuid) -> Option<Channel> {
        self.channels.read().await.get(&channel_id).cloned()
    }

    /// Get all channels for an agent
    pub async fn get_channels_for_agent(&self, agent_id: &str) -> Vec<Channel> {
        let channels = self.channels.read().await;
        channels.values()
            .filter(|c| c.channel_type == ChannelType::General || c.has_participant(agent_id))
            .cloned()
            .collect()
    }

    /// Subscribe to broadcast messages
    pub fn subscribe(&self) -> broadcast::Receiver<ChatMessage> {
        self.broadcast_tx.subscribe()
    }

    /// Dequeue the next message
    pub async fn dequeue_message(&mut self) -> Result<ChatMessage, MessageQueueError> {
        self.message_queue.dequeue().await.map(|qm| qm.message)
    }

    /// Get queue size
    pub fn queue_size(&self) -> usize {
        self.message_queue.size()
    }

    /// Broadcast a message to all general channel participants
    pub async fn broadcast_to_general(&self, sender_id: String, content: String) -> Result<ChatMessage, RouterError> {
        let general_channel_id = self.get_or_create_general_channel("General".to_string()).await;
        self.send_message(sender_id, general_channel_id, content, MessagePriority::Normal).await
    }
}

/// Errors that can occur in router operations
#[derive(Debug, thiserror::Error)]
pub enum RouterError {
    #[error("Channel not found")]
    ChannelNotFound,
    #[error("Channel is inactive")]
    ChannelInactive,
    #[error("Agent not authorized")]
    Unauthorized,
    #[error("Handler error: {0}")]
    HandlerError(String),
    #[error("Queue error: {0}")]
    QueueError(#[from] MessageQueueError),
    #[error("Channel error: {0}")]
    ChannelError(#[from] ChannelError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_general_channel_creation() {
        let router = MessageRouter::new(100, 1000);
        let channel_id = router.get_or_create_general_channel("General".to_string()).await;
        
        let channel = router.get_channel(channel_id).await.unwrap();
        assert_eq!(channel.channel_type, ChannelType::General);
    }

    #[tokio::test]
    async fn test_private_channel_creation() {
        let router = MessageRouter::new(100, 1000);
        let channel_id = router.get_or_create_private_channel(
            "agent1".to_string(),
            "agent2".to_string(),
        ).await;
        
        let channel = router.get_channel(channel_id).await.unwrap();
        match channel.channel_type {
            ChannelType::Private(ref a1, ref a2) => {
                assert_eq!(a1, "agent1");
                assert_eq!(a2, "agent2");
            }
            _ => panic!("Expected Private channel"),
        }
    }

    #[tokio::test]
    async fn test_send_message() {
        let router = MessageRouter::new(100, 1000);
        let channel_id = router.get_or_create_general_channel("General".to_string()).await;
        
        let message = router.send_message(
            "agent1".to_string(),
            channel_id,
            "Hello!".to_string(),
            MessagePriority::Normal,
        ).await.unwrap();
        
        assert_eq!(message.sender_id, "agent1");
        assert_eq!(message.content, "Hello!");
    }

    #[tokio::test]
    async fn test_join_leave_channel() {
        let router = MessageRouter::new(100, 1000);
        let channel_id = router.create_topic_channel("testing".to_string()).await;
        
        router.join_channel(channel_id, "agent1".to_string()).await.unwrap();
        
        let channel = router.get_channel(channel_id).await.unwrap();
        assert!(channel.has_participant("agent1"));
        
        router.leave_channel(channel_id, "agent1").await.unwrap();
        
        let channel = router.get_channel(channel_id).await.unwrap();
        assert!(!channel.has_participant("agent1"));
    }
}
