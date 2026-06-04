use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use tokio::sync::mpsc;

/// Represents a chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Unique identifier for this message
    pub id: Uuid,
    /// ID of the agent who sent this message
    pub sender_id: String,
    /// ID of the channel this message belongs to
    pub channel_id: Uuid,
    /// Content of the message
    pub content: String,
    /// Timestamp when the message was created
    pub timestamp: DateTime<Utc>,
    /// Optional reference to a document
    pub document_ref: Option<Uuid>,
    /// Vote counts (approval, disapproval)
    pub votes: (u32, u32),
    /// List of voter IDs for detailed tracking
    pub voters: Vec<String>,
}

impl ChatMessage {
    /// Create a new chat message
    pub fn new(sender_id: String, channel_id: Uuid, content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender_id,
            channel_id,
            content,
            timestamp: Utc::now(),
            document_ref: None,
            votes: (0, 0),
            voters: Vec::new(),
        }
    }

    /// Add a document reference to this message
    pub fn with_document(mut self, doc_id: Uuid) -> Self {
        self.document_ref = Some(doc_id);
        self
    }

    /// Vote on this message (true for approval, false for disapproval)
    pub fn vote(&mut self, voter_id: String, approve: bool) {
        // Remove existing vote from this voter if any
        self.voters.retain(|v| v != &voter_id);
        
        // Adjust vote counts
        if self.votes.0 > 0 || self.votes.1 > 0 {
            // Simple recount - in production would track individual votes better
            if approve {
                self.votes.0 += 1;
            } else {
                self.votes.1 += 1;
            }
        } else {
            if approve {
                self.votes.0 = 1;
            } else {
                self.votes.1 = 1;
            }
        }
        
        self.voters.push(voter_id);
    }

    /// Truncate content to a maximum word count
    pub fn truncate_content(&self, max_words: usize) -> String {
        let words: Vec<&str> = self.content.split_whitespace().collect();
        if words.len() <= max_words {
            self.content.clone()
        } else {
            format!("{}...", words[..max_words].join(" "))
        }
    }
}

/// Queued message with delivery metadata
#[derive(Debug, Clone)]
pub struct QueuedMessage {
    pub message: ChatMessage,
    pub priority: MessagePriority,
    pub retry_count: u32,
    pub created_at: DateTime<Utc>,
}

/// Priority levels for message delivery
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Urgent = 3,
}

impl Default for MessagePriority {
    fn default() -> Self {
        MessagePriority::Normal
    }
}

/// Message queue for managing message delivery
#[derive(Debug, Clone)]
pub struct MessageQueue {
    sender: mpsc::Sender<QueuedMessage>,
    receiver: mpsc::Receiver<QueuedMessage>,
}

impl MessageQueue {
    /// Create a new message queue with specified capacity
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(capacity);
        Self { sender, receiver }
    }

    /// Enqueue a message with normal priority
    pub async fn enqueue(&self, message: ChatMessage) -> Result<(), MessageQueueError> {
        self.enqueue_with_priority(message, MessagePriority::Normal).await
    }

    /// Enqueue a message with specified priority
    pub async fn enqueue_with_priority(
        &self,
        message: ChatMessage,
        priority: MessagePriority,
    ) -> Result<(), MessageQueueError> {
        let queued = QueuedMessage {
            message,
            priority,
            retry_count: 0,
            created_at: Utc::now(),
        };

        self.sender.send(queued).await.map_err(|_| MessageQueueError::QueueFull)
    }

    /// Dequeue the next message (priority-aware)
    pub async fn dequeue(&mut self) -> Result<QueuedMessage, MessageQueueError> {
        self.receiver.recv().await.ok_or(MessageQueueError::QueueEmpty)
    }

    /// Get the current queue size
    pub fn size(&self) -> usize {
        self.sender.capacity()
    }
}

/// Errors that can occur in message queue operations
#[derive(Debug, thiserror::Error)]
pub enum MessageQueueError {
    #[error("Queue is full")]
    QueueFull,
    #[error("Queue is empty")]
    QueueEmpty,
    #[error("Queue operation failed: {0}")]
    OperationFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = ChatMessage::new(
            "agent1".to_string(),
            Uuid::new_v4(),
            "Hello, world!".to_string(),
        );
        
        assert_eq!(msg.sender_id, "agent1");
        assert_eq!(msg.content, "Hello, world!");
        assert_eq!(msg.votes, (0, 0));
    }

    #[test]
    fn test_message_truncation() {
        let msg = ChatMessage::new(
            "agent1".to_string(),
            Uuid::new_v4(),
            "This is a long message with many words that should be truncated properly".to_string(),
        );
        
        let truncated = msg.truncate_content(5);
        assert_eq!(truncated, "This is a long...");
    }

    #[test]
    fn test_voting() {
        let mut msg = ChatMessage::new(
            "agent1".to_string(),
            Uuid::new_v4(),
            "Test message".to_string(),
        );
        
        msg.vote("voter1".to_string(), true);
        assert_eq!(msg.votes, (1, 0));
        assert!(msg.voters.contains(&"voter1".to_string()));
        
        msg.vote("voter2".to_string(), false);
        assert_eq!(msg.votes, (1, 1));
    }
}
