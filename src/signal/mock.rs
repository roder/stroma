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
    group_info: HashMap<GroupId, GroupInfo>,
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

#[async_trait(?Send)]
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

    async fn create_group(
        &self,
        name: &str,
        members: &[ServiceId],
    ) -> SignalResult<(GroupId, Vec<ServiceId>)> {
        let mut state = self.state.lock().unwrap();
        let group_id = state.next_group_id;
        state.next_group_id += 1;

        // Create group ID from counter
        let group = GroupId(group_id.to_le_bytes().to_vec());

        // Initialize member list with provided members
        state.group_members.insert(group.clone(), members.to_vec());

        // Initialize group info with defaults
        state.group_info.insert(
            group.clone(),
            GroupInfo {
                name: name.to_string(),
                description: None,
                disappearing_messages_timer: None,
                announcements_only: false,
            },
        );

        // Mock: no pending members (all members are "full" in tests)
        Ok((group, vec![]))
    }

    async fn send_group_invite(&self, _group: &GroupId, _member: &ServiceId) -> SignalResult<()> {
        // Mock: no-op
        Ok(())
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
            Err(SignalError::MemberNotFound("[ServiceId]".to_string()))
        }
    }

    async fn create_poll(
        &self,
        group: &GroupId,
        question: &str,
        options: Vec<String>,
        _allow_multiple: bool,
    ) -> SignalResult<u64> {
        let mut state = self.state.lock().unwrap();

        if !state.group_members.contains_key(group) {
            return Err(SignalError::GroupNotFound(group.to_string()));
        }

        let poll_id = state.next_poll_id;
        state.next_poll_id += 1;
        let poll = Poll {
            question: question.to_string(),
            options,
        };
        state.polls.insert(poll_id, (group.clone(), poll));

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

    async fn get_group_info(&self, group: &GroupId) -> SignalResult<GroupInfo> {
        let state = self.state.lock().unwrap();
        state
            .group_info
            .get(group)
            .cloned()
            .ok_or_else(|| SignalError::GroupNotFound(group.to_string()))
    }

    async fn set_group_name(&self, group: &GroupId, name: &str) -> SignalResult<()> {
        let mut state = self.state.lock().unwrap();
        let info = state
            .group_info
            .get_mut(group)
            .ok_or_else(|| SignalError::GroupNotFound(group.to_string()))?;
        info.name = name.to_string();
        Ok(())
    }

    async fn set_group_description(&self, group: &GroupId, description: &str) -> SignalResult<()> {
        let mut state = self.state.lock().unwrap();
        let info = state
            .group_info
            .get_mut(group)
            .ok_or_else(|| SignalError::GroupNotFound(group.to_string()))?;
        info.description = if description.is_empty() {
            None
        } else {
            Some(description.to_string())
        };
        Ok(())
    }

    async fn set_disappearing_messages(&self, group: &GroupId, seconds: u32) -> SignalResult<()> {
        let mut state = self.state.lock().unwrap();
        let info = state
            .group_info
            .get_mut(group)
            .ok_or_else(|| SignalError::GroupNotFound(group.to_string()))?;
        info.disappearing_messages_timer = if seconds == 0 { None } else { Some(seconds) };
        Ok(())
    }

    async fn set_announcements_only(&self, group: &GroupId, enabled: bool) -> SignalResult<()> {
        let mut state = self.state.lock().unwrap();
        let info = state
            .group_info
            .get_mut(group)
            .ok_or_else(|| SignalError::GroupNotFound(group.to_string()))?;
        info.announcements_only = enabled;
        Ok(())
    }

    async fn resolve_identifier(&self, identifier: &str) -> SignalResult<ServiceId> {
        use crate::signal::pm::{parse_identifier, Identifier};

        let parsed = parse_identifier(identifier);

        match parsed {
            Identifier::Uuid(uuid_str) => {
                uuid::Uuid::parse_str(&uuid_str).map_err(|e| {
                    SignalError::InvalidMessage(format!("Invalid UUID '{}': {}", uuid_str, e))
                })?;
                Ok(ServiceId(uuid_str))
            }
            Identifier::Username(username) => {
                // Mock: For tests, treat usernames as direct service IDs
                Ok(ServiceId(username))
            }
            Identifier::Phone(_phone) => Err(SignalError::NotImplemented(
                "Phone number resolution not yet implemented".to_string(),
            )),
        }
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

        let poll_id = client
            .create_poll(
                &group,
                "Approve member?",
                vec!["Approve".to_string(), "Reject".to_string()],
                false,
            )
            .await
            .unwrap();
        assert_eq!(poll_id, 0);
    }

    #[tokio::test]
    async fn test_group_settings() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let (group, _pending) = client.create_group("Test Group", &[]).await.unwrap();

        // Check initial state
        let info = client.get_group_info(&group).await.unwrap();
        assert_eq!(info.name, "Test Group");
        assert_eq!(info.description, None);
        assert_eq!(info.disappearing_messages_timer, None);
        assert!(!info.announcements_only);

        // Update settings
        client.set_group_name(&group, "New Name").await.unwrap();
        client
            .set_group_description(&group, "Test description")
            .await
            .unwrap();
        client
            .set_disappearing_messages(&group, 86400)
            .await
            .unwrap();
        client.set_announcements_only(&group, true).await.unwrap();

        // Verify changes
        let info = client.get_group_info(&group).await.unwrap();
        assert_eq!(info.name, "New Name");
        assert_eq!(info.description, Some("Test description".to_string()));
        assert_eq!(info.disappearing_messages_timer, Some(86400));
        assert!(info.announcements_only);
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
