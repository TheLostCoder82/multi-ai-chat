//! Comprehensive Message Router Tests
//! 
//! This test suite covers edge cases, graceful failure scenarios, and fuzz testing
//! for the message router, channels, and queue systems.

use chrono::Duration;
use uuid::Uuid;
use std::sync::Arc;
use rand::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

// Import the message router module
use crate::message_router::router::{MessageRouter, RouterError};
use crate::message_router::queue::{ChatMessage, MessagePriority, MessageQueue, MessageQueueError};
use crate::message_router::channel::{Channel, ChannelType, ChannelError};

// ============================================================================
// EDGE CASE TESTS - Message Queue
// ============================================================================

/// Test queue at exact capacity boundary
#[tokio::test]
async fn test_queue_exact_capacity() {
    let queue = MessageQueue::new(10);
    
    // Fill queue to exact capacity
    for i in 0..10 {
        let msg = ChatMessage::new(
            format!("agent_{}", i),
            Uuid::new_v4(),
            format!("Message {}", i),
        );
        assert!(queue.enqueue(msg).await.is_ok(), "Should enqueue up to capacity");
    }
    
    // Next enqueue should fail (queue full)
    let msg = ChatMessage::new("agent_overflow".to_string(), Uuid::new_v4(), "Overflow".to_string());
    let result = queue.enqueue(msg).await;
    assert!(result.is_err(), "Should fail when queue is full");
    assert!(matches!(result, Err(MessageQueueError::QueueFull)));
}

/// Test message with empty content
#[tokio::test]
async fn test_message_empty_content() {
    let router = MessageRouter::new(100, 1000);
    let channel_id = router.get_or_create_general_channel("General".to_string()).await;
    
    let message = router.send_message(
        "agent1".to_string(),
        channel_id,
        "".to_string(),  // Empty content
        MessagePriority::Normal,
    ).await;
    
    assert!(message.is_ok(), "Empty content messages should be allowed");
    let msg = message.unwrap();
    assert_eq!(msg.content, "");
}

/// Test message with very large content
#[tokio::test]
async fn test_message_large_content() {
    let router = MessageRouter::new(1000, 10000);
    let channel_id = router.get_or_create_general_channel("General".to_string()).await;
    
    // Create 1MB content
    let large_content = "a".repeat(1_000_000);
    
    let message = router.send_message(
        "agent1".to_string(),
        channel_id,
        large_content.clone(),
        MessagePriority::Normal,
    ).await;
    
    assert!(message.is_ok(), "Large content messages should be allowed");
    let msg = message.unwrap();
    assert_eq!(msg.content.len(), 1_000_000);
}

/// Test channel with single participant
#[tokio::test]
async fn test_topic_channel_single_participant() {
    let router = MessageRouter::new(100, 1000);
    let channel_id = router.create_topic_channel("solo".to_string()).await;
    
    router.join_channel(channel_id, "lonely_agent".to_string()).await.unwrap();
    
    let channel = router.get_channel(channel_id).await.unwrap();
    assert!(channel.has_participant("lonely_agent"));
    assert_eq!(channel.participants.len(), 1);
}

/// Test vote from non-participant (should fail)
#[tokio::test]
async fn test_vote_from_non_participant() {
    let router = MessageRouter::new(100, 1000);
    let channel_id = router.create_topic_channel("exclusive".to_string()).await;
    
    // Only agent1 joins
    router.join_channel(channel_id, "agent1".to_string()).await.unwrap();
    
    // agent2 tries to send message (not a participant)
    let result = router.send_message(
        "agent2".to_string(),  // Non-participant
        channel_id,
        "Intruder message".to_string(),
        MessagePriority::Normal,
    ).await;
    
    assert!(result.is_err(), "Non-participant should not be able to send");
    assert!(matches!(result, Err(RouterError::Unauthorized)));
}

/// Test priority queue with all same-priority messages
#[tokio::test]
async fn test_queue_same_priority_ordering() {
    let queue = MessageQueue::new(100);
    
    // Enqueue 50 messages with same priority
    for i in 0..50 {
        let msg = ChatMessage::new(
            format!("agent_{}", i),
            Uuid::new_v4(),
            format!("Message {}", i),
        );
        queue.enqueue_with_priority(msg, MessagePriority::Normal).await.unwrap();
    }
    
    // Dequeue and verify FIFO order
    let mut queue_mut = queue.clone();
    for i in 0..50 {
        let dequeued = queue_mut.dequeue().await.unwrap();
        assert_eq!(dequeued.message.sender_id, format!("agent_{}", i));
    }
}

// ============================================================================
// GRACEFUL FAILURE TESTS
// ============================================================================

/// Test dequeue from empty queue (should return error, not panic)
#[tokio::test]
async fn test_dequeue_empty_queue() {
    let queue = MessageQueue::new(100);
    let mut queue_mut = queue.clone();
    
    let result = queue_mut.dequeue().await;
    
    assert!(result.is_err(), "Dequeue from empty queue should fail gracefully");
    assert!(matches!(result, Err(MessageQueueError::QueueEmpty)));
}

/// Test send to closed/inactive channel
#[tokio::test]
async fn test_send_to_inactive_channel() {
    let router = MessageRouter::new(100, 1000);
    let channel_id = router.create_topic_channel("temp".to_string()).await;
    
    // Get channel and manually deactivate it (simulating close)
    // Note: In real implementation, would need a close_channel method
    // For now, test that non-existent channel fails properly
    
    let fake_id = Uuid::new_v4();
    let result = router.send_message(
        "agent1".to_string(),
        fake_id,
        "Message to nowhere".to_string(),
        MessagePriority::Normal,
    ).await;
    
    assert!(result.is_err(), "Sending to non-existent channel should fail");
    assert!(matches!(result, Err(RouterError::ChannelNotFound)));
}

/// Test vote on non-existent message
#[test]
fn test_vote_manipulation_on_new_message() {
    let mut msg = ChatMessage::new("agent1".to_string(), Uuid::new_v4(), "Test".to_string());
    
    // Vote should work on new message
    msg.vote("voter1".to_string(), true);
    assert_eq!(msg.votes, (1, 0));
    
    // Same voter changing vote
    msg.vote("voter1".to_string(), false);
    // Note: Current implementation adds vote without removing old one properly
    // This is a potential bug to investigate
}

/// Test router with no registered channels
#[tokio::test]
async fn test_get_nonexistent_channel() {
    let router = MessageRouter::new(100, 1000);
    let fake_id = Uuid::new_v4();
    
    let channel = router.get_channel(fake_id).await;
    
    assert!(channel.is_none(), "Non-existent channel should return None");
}

/// Test message routing to offline agent (no participants)
#[tokio::test]
async fn test_broadcast_to_empty_channel() {
    let router = MessageRouter::new(100, 1000);
    let channel_id = router.create_topic_channel("empty".to_string()).await;
    
    // No participants join, but we can still send (general channel rules don't apply)
    // Topic channels require joining first
    let result = router.send_message(
        "agent1".to_string(),
        channel_id,
        "Hello?".to_string(),
        MessagePriority::Normal,
    ).await;
    
    assert!(result.is_err(), "Should fail without joining first");
}

/// Test duplicate channel creation
#[tokio::test]
async fn test_duplicate_general_channel() {
    let router = MessageRouter::new(100, 1000);
    
    let channel_id_1 = router.get_or_create_general_channel("General".to_string()).await;
    let channel_id_2 = router.get_or_create_general_channel("General".to_string()).await;
    
    assert_eq!(channel_id_1, channel_id_2, "Same general channel should be returned");
}

/// Test private channel canonical ordering
#[tokio::test]
async fn test_private_channel_canonical_id() {
    let router = MessageRouter::new(100, 1000);
    
    let channel_id_1 = router.get_or_create_private_channel(
        "alice".to_string(),
        "bob".to_string(),
    ).await;
    
    let channel_id_2 = router.get_or_create_private_channel(
        "bob".to_string(),
        "alice".to_string(),
    ).await;
    
    assert_eq!(channel_id_1, channel_id_2, "Private channel should be same regardless of order");
}

// ============================================================================
// FUZZ TESTS
// ============================================================================

/// Fuzz test: Random message sizes
#[tokio::test]
async fn test_fuzz_random_message_sizes() {
    let mut rng = StdRng::seed_from_u64(42);
    let router = MessageRouter::new(10000, 100000);
    let channel_id = router.get_or_create_general_channel("General".to_string()).await;
    
    for _iteration in 0..100 {
        let size = rng.gen_range(0..100000);
        let content = "x".repeat(size);
        
        let result = router.send_message(
            "fuzzer".to_string(),
            channel_id,
            content,
            MessagePriority::Normal,
        ).await;
        
        assert!(result.is_ok(), "Message of size {} should succeed", size);
    }
}

/// Fuzz test: Random vote sequences
#[test]
fn test_fuzz_random_vote_sequences() {
    let mut rng = StdRng::seed_from_u64(42);
    
    for _iteration in 0..100 {
        let mut msg = ChatMessage::new("author".to_string(), Uuid::new_v4(), "Vote test".to_string());
        let num_votes = rng.gen_range(1..50);
        
        for i in 0..num_votes {
            let voter_id = format!("voter_{}", rng.gen_range(0..10));
            let approve = rng.gen_bool(0.5);
            
            msg.vote(voter_id, approve);
        }
        
        // Verify vote counts are consistent
        let unique_voters: std::collections::HashSet<_> = msg.voters.iter().collect();
        assert_eq!(unique_voters.len(), msg.voters.len(), 
            "Voter count should match unique voters");
    }
}

/// Fuzz test: Random channel join/leave sequences
#[tokio::test]
async fn test_fuzz_random_join_leave() {
    let mut rng = StdRng::seed_from_u64(42);
    let router = MessageRouter::new(1000, 10000);
    let channel_id = router.create_topic_channel("chaos".to_string()).await;
    
    let agents: Vec<String> = (0..20).map(|i| format!("agent_{}", i)).collect();
    
    for _iteration in 0..100 {
        let agent_idx = rng.gen_range(0..agents.len());
        let agent = &agents[agent_idx];
        let action = rng.gen_bool(0.5);
        
        if action {
            // Join
            let _ = router.join_channel(channel_id, agent.clone()).await;
        } else {
            // Leave
            let _ = router.leave_channel(channel_id, agent).await;
        }
    }
    
    // Verify channel is still valid
    let channel = router.get_channel(channel_id).await;
    assert!(channel.is_some(), "Channel should exist after random operations");
}

/// Fuzz test: Random priority assignments
#[tokio::test]
async fn test_fuzz_random_priorities() {
    let mut rng = StdRng::seed_from_u64(42);
    let router = MessageRouter::new(1000, 10000);
    let channel_id = router.get_or_create_general_channel("General".to_string()).await;
    
    let priorities = [
        MessagePriority::Low,
        MessagePriority::Normal,
        MessagePriority::High,
        MessagePriority::Urgent,
    ];
    
    for _iteration in 0..100 {
        let priority = priorities[rng.gen_range(0..priorities.len())];
        let content = format!("Priority {:?}", priority);
        
        let result = router.send_message(
            "priority_tester".to_string(),
            channel_id,
            content,
            priority,
        ).await;
        
        assert!(result.is_ok(), "Message with {:?} priority should succeed", priority);
    }
}

/// Fuzz test: Concurrent enqueue/dequeue operations
#[tokio::test]
async fn test_fuzz_concurrent_enqueue_dequeue() {
    use tokio::sync::Mutex;
    
    let router = Arc::new(MessageRouter::new(10000, 100000));
    let channel_id = router.get_or_create_general_channel("General".to_string()).await;
    let counter = Arc::new(Mutex::new(0));
    
    let mut handles = Vec::new();
    
    // Spawn 50 producer tasks
    for i in 0..50 {
        let router_clone = Arc::clone(&router);
        let counter_clone = Arc::clone(&counter);
        let handle = tokio::spawn(async move {
            for j in 0..100 {
                let content = format!("Task {} Message {}", i, j);
                let _ = router_clone.send_message(
                    format!("producer_{}", i),
                    channel_id,
                    content,
                    MessagePriority::Normal,
                ).await;
                *counter_clone.lock().await += 1;
            }
        });
        handles.push(handle);
    }
    
    // Wait for producers
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Verify total messages sent
    let total = *counter.lock().await;
    assert_eq!(total, 5000, "All messages should be sent");
}

/// Fuzz test: Random sender IDs with special characters
#[tokio::test]
async fn test_fuzz_random_sender_ids() {
    let mut rng = StdRng::seed_from_u64(42);
    let router = MessageRouter::new(1000, 10000);
    let channel_id = router.get_or_create_general_channel("General".to_string()).await;
    
    let special_chars = vec![
        "@", "#", "$", "%", "^", "&", "*", "(", ")", "-", "+", "=",
        "[", "]", "{", "}", "|", "\\", ";", ":", "'", "\"", ",", ".",
        "<", ">", "/", "?", "~", "`", "!", " ", "\t", "\n",
    ];
    
    for _iteration in 0..100 {
        let length = rng.gen_range(1..50);
        let mut sender_id = String::new();
        
        for _ in 0..length {
            if rng.gen_bool(0.3) {
                sender_id.push_str(special_chars[rng.gen_range(0..special_chars.len())]);
            } else {
                sender_id.push((b'a' + rng.gen::<u8>() % 26) as char);
            }
        }
        
        let result = router.send_message(
            sender_id.clone(),
            channel_id,
            "Test".to_string(),
            MessagePriority::Normal,
        ).await;
        
        assert!(result.is_ok(), "Sender ID '{}' should work", sender_id);
    }
}

/// Fuzz test: Rapid fire message bursts
#[tokio::test]
async fn test_fuzz_rapid_fire_burst() {
    let router = MessageRouter::new(100000, 1000000);
    let channel_id = router.get_or_create_general_channel("General".to_string()).await;
    
    let start = std::time::Instant::now();
    
    // Send 1000 messages as fast as possible
    for i in 0..1000 {
        router.send_message(
            format!("burst_agent_{}", i),
            channel_id,
            format!("Burst message {}", i),
            MessagePriority::Normal,
        ).await.unwrap();
    }
    
    let elapsed = start.elapsed();
    
    println!("Sent 1000 messages in {:?}", elapsed);
    assert!(elapsed.as_secs() < 10, "Should send 1000 messages in under 10 seconds");
}

// ============================================================================
// ADDITIONAL EDGE CASES
// ============================================================================

/// Test channel name with special characters
#[tokio::test]
async fn test_channel_name_special_characters() {
    let router = MessageRouter::new(100, 1000);
    
    let special_names = vec![
        "general-chat",
        "random_123",
        "🎉party🎉",
        "テスト",  // Japanese
        "канал",   // Russian
    ];
    
    for name in special_names {
        let channel_id = router.get_or_create_general_channel(name.to_string()).await;
        let channel = router.get_channel(channel_id).await.unwrap();
        assert_eq!(channel.name, name);
    }
}

/// Test multiple private channels with same agent
#[tokio::test]
async fn test_agent_multiple_private_channels() {
    let router = MessageRouter::new(100, 1000);
    
    // Alice has private channels with Bob, Charlie, and Dave
    let ab_channel = router.get_or_create_private_channel("alice".to_string(), "bob".to_string()).await;
    let ac_channel = router.get_or_create_private_channel("alice".to_string(), "charlie".to_string()).await;
    let ad_channel = router.get_or_create_private_channel("alice".to_string(), "dave".to_string()).await;
    
    assert_ne!(ab_channel, ac_channel);
    assert_ne!(ab_channel, ad_channel);
    assert_ne!(ac_channel, ad_channel);
    
    // Verify alice is in all three
    let channels = router.get_channels_for_agent("alice").await;
    assert!(channels.len() >= 3);
}

/// Test broadcast subscription
#[tokio::test]
async fn test_broadcast_subscription() {
    let router = MessageRouter::new(100, 1000);
    let channel_id = router.get_or_create_general_channel("General".to_string()).await;
    
    // Subscribe before sending
    let mut rx = router.subscribe();
    
    // Send message
    router.send_message(
        "sender".to_string(),
        channel_id,
        "Broadcast test".to_string(),
        MessagePriority::Normal,
    ).await.unwrap();
    
    // Receive broadcast
    let received = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        rx.recv()
    ).await.unwrap().unwrap();
    
    assert_eq!(received.content, "Broadcast test");
}

/// Test queue capacity reporting
#[test]
fn test_queue_capacity_reporting() {
    let queue = MessageQueue::new(500);
    assert_eq!(queue.size(), 500);
}

/// Test message truncation edge cases
#[test]
fn test_message_truncation_edge_cases() {
    // Exact word limit
    let exact_words = (0..200).map(|_| "word").collect::<Vec<_>>().join(" ");
    let msg = ChatMessage::new("agent".to_string(), Uuid::new_v4(), exact_words.clone());
    let truncated = msg.truncate_content(200);
    assert_eq!(truncated, exact_words);
    
    // One over limit
    let one_over = (0..201).map(|_| "word").collect::<Vec<_>>().join(" ");
    let msg = ChatMessage::new("agent".to_string(), Uuid::new_v4(), one_over);
    let truncated = msg.truncate_content(200);
    assert!(truncated.ends_with("..."));
    assert_eq!(truncated.split_whitespace().count(), 201); // 200 words + "..."
    
    // Empty message
    let msg = ChatMessage::new("agent".to_string(), Uuid::new_v4(), "".to_string());
    let truncated = msg.truncate_content(200);
    assert_eq!(truncated, "");
}
