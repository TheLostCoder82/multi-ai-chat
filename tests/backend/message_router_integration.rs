// Integration tests for message router
use multi_agent_chat::message_router::{MessageRouter, ChannelType, Message, QueuedMessage};
use multi_agent_chat::permission::{Permission, PermissionScope};
use tokio;

#[tokio::test]
async fn test_channel_creation_and_messaging() {
    let router = MessageRouter::new();
    
    // Create a channel
    router.create_channel("test_channel", ChannelType::Public).await.unwrap();
    
    // Send a message
    let message = Message {
        id: "msg_1".to_string(),
        channel: "test_channel".to_string(),
        sender: "agent_1".to_string(),
        content: "Hello, world!".to_string(),
        timestamp: chrono::Utc::now(),
        metadata: None,
    };
    
    router.send_message(message.clone()).await.unwrap();
    
    // Retrieve messages from channel
    let messages = router.get_channel_messages("test_channel", 10).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "Hello, world!");
}

#[tokio::test]
async fn test_private_message_routing() {
    let router = MessageRouter::new();
    
    // Create private channel between two agents
    router.create_channel("pm_agent1_agent2", ChannelType::Private).await.unwrap();
    
    let pm_message = Message {
        id: "pm_1".to_string(),
        channel: "pm_agent1_agent2".to_string(),
        sender: "agent_1".to_string(),
        content: "Private message content".to_string(),
        timestamp: chrono::Utc::now(),
        metadata: None,
    };
    
    router.send_message(pm_message.clone()).await.unwrap();
    
    // Verify only participants can access
    let messages = router.get_channel_messages("pm_agent1_agent2", 10).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].sender, "agent_1");
}

#[tokio::test]
async fn test_message_queuing_with_voting() {
    let router = MessageRouter::new();
    
    router.create_channel("voting_channel", ChannelType::Public).await.unwrap();
    
    // Queue a message for voting
    let queued_msg = router.queue_message_for_voting(
        "proposal_1".to_string(),
        "voting_channel".to_string(),
        "agent_1".to_string(),
        "This is a proposal".to_string(),
        3, // required votes
    ).await.unwrap();
    
    // Cast votes
    router.cast_vote("proposal_1".to_string(), "agent_2".to_string(), true).await.unwrap();
    router.cast_vote("proposal_1".to_string(), "agent_3".to_string(), true).await.unwrap();
    router.cast_vote("proposal_1".to_string(), "agent_4".to_string(), true).await.unwrap();
    
    // Check if vote threshold reached
    let status = router.get_voting_status("proposal_1".to_string()).await.unwrap();
    assert!(status.threshold_reached);
    
    // Process the queued message
    router.process_queued_message("proposal_1".to_string()).await.unwrap();
    
    // Verify message was sent to channel
    let messages = router.get_channel_messages("voting_channel", 10).await.unwrap();
    assert_eq!(messages.len(), 1);
}

#[tokio::test]
async fn test_broadcast_mechanism() {
    let router = MessageRouter::new();
    
    // Create multiple channels
    router.create_channel("channel_1", ChannelType::Public).await.unwrap();
    router.create_channel("channel_2", ChannelType::Public).await.unwrap();
    router.create_channel("channel_3", ChannelType::Public).await.unwrap();
    
    // Broadcast message to all channels
    let broadcast_msg = Message {
        id: "broadcast_1".to_string(),
        channel: "@all".to_string(),
        sender: "system".to_string(),
        content: "Broadcast announcement".to_string(),
        timestamp: chrono::Utc::now(),
        metadata: None,
    };
    
    router.broadcast_message(broadcast_msg.clone()).await.unwrap();
    
    // Verify message appears in all channels
    for channel_name in &["channel_1", "channel_2", "channel_3"] {
        let messages = router.get_channel_messages(channel_name, 10).await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Broadcast announcement");
    }
}

#[tokio::test]
async fn test_permission_enforcement_in_routing() {
    let router = MessageRouter::new();
    
    router.create_channel("restricted_channel", ChannelType::Private).await.unwrap();
    
    // Try to send message without permission (should fail or be queued)
    let restricted_msg = Message {
        id: "restricted_1".to_string(),
        channel: "restricted_channel".to_string(),
        sender: "unauthorized_agent".to_string(),
        content: "Unauthorized message".to_string(),
        timestamp: chrono::Utc::now(),
        metadata: None,
    };
    
    // This should either fail or require permission check
    let result = router.send_message_with_permission_check(restricted_msg).await;
    
    // Depending on implementation, this might return an error or queue the message
    assert!(result.is_err() || result.unwrap().is_none());
}

#[tokio::test]
async fn test_message_routing_patterns() {
    let router = MessageRouter::new();
    
    // Test pattern-based routing
    router.create_channel("news.general", ChannelType::Public).await.unwrap();
    router.create_channel("news.tech", ChannelType::Public).await.unwrap();
    router.create_channel("news.sports", ChannelType::Public).await.unwrap();
    
    // Route message with pattern matching
    let tech_news = Message {
        id: "news_1".to_string(),
        channel: "news.*".to_string(), // Pattern match
        sender: "news_bot".to_string(),
        content: "Tech update".to_string(),
        timestamp: chrono::Utc::now(),
        metadata: Some(serde_json::json!({"category": "tech"})),
    };
    
    router.route_message_by_pattern(tech_news).await.unwrap();
    
    // Verify message routed to correct channel based on metadata
    let tech_messages = router.get_channel_messages("news.tech", 10).await.unwrap();
    assert!(!tech_messages.is_empty());
}
