//! Mock Signal Client for Testing
//!
//! Provides MockSignalClient for 100% test coverage without real Signal network.

use super::traits::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock Signal client for testing
#[derive(Clone)]
pub struct MockSignalClient {
    state: Arc<Mutex<MockState>>,
    service_id: ServiceId,
}

#[derive(Default)]
struct MockState {
    sent_messages: Vec<SentMessage>,
    group_members: HashMap<GroupId, Vec<ServiceId>>,
    incoming_messages: Vec<Message>,
    polls: HashMap<u64, (GroupId, Poll)>,
    next_poll_id: u64,
    next_group_id: u64,
}

#[derive(Debug, Clone)]
pub struct SentMessage {
    pub recipient: Recipient,
    pub content: String,
}

#[derive(Debug, Clone)]
pub enum Recipient {
    User(ServiceId),
    Group(GroupId),
}

impl MockSignalClient {
    /// Create new mock client
    pub fn new(service_id: ServiceId) -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState::default())),
            service_id,
        }
    }

    /// Add incoming message for testing
    pub fn add_incoming_message(&self, message: Message) {
        let mut state = self.state.lock().unwrap();
        state.incoming_messages.push(message);
    }

    /// Get sent messages for assertions
    pub fn sent_messages(&self) -> Vec<SentMessage> {
        self.state.lock().unwrap().sent_messages.clone()
    }

    /// Get sent group messages for a specific group
    pub fn sent_group_messages(&self, group: &GroupId) -> Vec<String> {
        self.state
            .lock()
            .unwrap()
            .sent_messages
            .iter()
            .filter_map(|msg| {
                if let Recipient::Group(ref msg_group) = msg.recipient {
                    if msg_group == group {
                        return Some(msg.content.clone());
                    }
                }
                None
            })
            .collect()
    }

    /// Get group members for assertions
    pub fn group_members(&self, group: &GroupId) -> Option<Vec<ServiceId>> {
        self.state.lock().unwrap().group_members.get(group).cloned()
    }

    /// Check if member is in group
    pub fn is_member(&self, group: &GroupId, member: &ServiceId) -> bool {
        self.state
            .lock()
            .unwrap()
            .group_members
            .get(group)
            .map(|members| members.contains(member))
            .unwrap_or(false)
    }

    /// Clear all state
    pub fn clear(&self) {
        let mut state = self.state.lock().unwrap();
        *state = MockState::default();
    }
}

#[async_trait]
impl SignalClient for MockSignalClient {
    async fn send_message(&self, recipient: &ServiceId, text: &str) -> SignalResult<()> {
        let mut state = self.state.lock().unwrap();
        state.sent_messages.push(SentMessage {
            recipient: Recipient::User(recipient.clone()),
            content: text.to_string(),
        });
        Ok(())
    }

    async fn send_group_message(&self, group: &GroupId, text: &str) -> SignalResult<()> {
        let state = self.state.lock().unwrap();
        if !state.group_members.contains_key(group) {
            return Err(SignalError::GroupNotFound(group.to_string()));
        }
        drop(state);

        let mut state = self.state.lock().unwrap();
        state.sent_messages.push(SentMessage {
            recipient: Recipient::Group(group.clone()),
            content: text.to_string(),
        });
        Ok(())
    }

    async fn create_group(&self, _name: &str) -> SignalResult<GroupId> {
        let mut state = self.state.lock().unwrap();
        let group_id = state.next_group_id;
        state.next_group_id += 1;

        // Create group ID from counter
        let group = GroupId(group_id.to_le_bytes().to_vec());

        // Initialize empty member list
        state.group_members.insert(group.clone(), Vec::new());

        Ok(group)
    }

    async fn add_group_member(&self, group: &GroupId, member: &ServiceId) -> SignalResult<()> {
        let mut state = self.state.lock().unwrap();
        state
            .group_members
            .entry(group.clone())
            .or_default()
            .push(member.clone());
        Ok(())
    }

    async fn remove_group_member(&self, group: &GroupId, member: &ServiceId) -> SignalResult<()> {
        let mut state = self.state.lock().unwrap();
        let members = state
            .group_members
            .get_mut(group)
            .ok_or_else(|| SignalError::GroupNotFound(group.to_string()))?;

        if let Some(pos) = members.iter().position(|m| m == member) {
            members.remove(pos);
            Ok(())
        } else {
            Err(SignalError::MemberNotFound(member.to_string()))
        }
    }

    async fn create_poll(&self, group: &GroupId, poll: &Poll) -> SignalResult<u64> {
        let mut state = self.state.lock().unwrap();

        if !state.group_members.contains_key(group) {
            return Err(SignalError::GroupNotFound(group.to_string()));
        }

        let poll_id = state.next_poll_id;
        state.next_poll_id += 1;
        state.polls.insert(poll_id, (group.clone(), poll.clone()));

        Ok(poll_id)
    }

    async fn terminate_poll(&self, group: &GroupId, poll_timestamp: u64) -> SignalResult<()> {
        let mut state = self.state.lock().unwrap();

        if !state.group_members.contains_key(group) {
            return Err(SignalError::GroupNotFound(group.to_string()));
        }

        // In mock, just track that poll was terminated
        // Real implementation would send PollTerminate message to group
        state.sent_messages.push(SentMessage {
            recipient: Recipient::Group(group.clone()),
            content: format!("Poll {} terminated", poll_timestamp),
        });

        Ok(())
    }

    async fn receive_messages(&self) -> SignalResult<Vec<Message>> {
        let mut state = self.state.lock().unwrap();
        let messages = state.incoming_messages.drain(..).collect();
        Ok(messages)
    }

    fn service_id(&self) -> &ServiceId {
        &self.service_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_send_message() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let recipient = ServiceId("user1".to_string());

        client.send_message(&recipient, "Hello").await.unwrap();

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].content, "Hello");
    }

    #[tokio::test]
    async fn test_group_operations() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let group = GroupId(vec![1, 2, 3]);
        let member1 = ServiceId("user1".to_string());
        let member2 = ServiceId("user2".to_string());

        // Add members
        client.add_group_member(&group, &member1).await.unwrap();
        client.add_group_member(&group, &member2).await.unwrap();

        assert!(client.is_member(&group, &member1));
        assert!(client.is_member(&group, &member2));

        // Remove member
        client.remove_group_member(&group, &member1).await.unwrap();
        assert!(!client.is_member(&group, &member1));
        assert!(client.is_member(&group, &member2));
    }

    #[tokio::test]
    async fn test_send_group_message() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let group = GroupId(vec![1, 2, 3]);
        let member = ServiceId("user1".to_string());

        // Must add member first
        client.add_group_member(&group, &member).await.unwrap();

        // Send group message
        client
            .send_group_message(&group, "Hello group")
            .await
            .unwrap();

        let sent = client.sent_messages();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].content, "Hello group");
    }

    #[tokio::test]
    async fn test_create_poll() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let group = GroupId(vec![1, 2, 3]);
        let member = ServiceId("user1".to_string());

        client.add_group_member(&group, &member).await.unwrap();

        let poll = Poll {
            question: "Approve member?".to_string(),
            options: vec!["Approve".to_string(), "Reject".to_string()],
        };

        let poll_id = client.create_poll(&group, &poll).await.unwrap();
        assert_eq!(poll_id, 0);
    }

    #[tokio::test]
    async fn test_receive_messages() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let sender = ServiceId("user1".to_string());

        let message = Message {
            sender: sender.clone(),
            content: MessageContent::Text("/status".to_string()),
            timestamp: 1234567890,
        };

        client.add_incoming_message(message);

        let received = client.receive_messages().await.unwrap();
        assert_eq!(received.len(), 1);

        match &received[0].content {
            MessageContent::Text(text) => assert_eq!(text, "/status"),
            _ => panic!("Expected text message"),
        }
    }
}
