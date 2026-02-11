//! Production Signal Client Implementation
//!
//! Implements SignalClient trait using libsignal-service-rs directly.
//! This bypasses presage's abstraction layer for direct control.
//!
//! Note: The presage dependency is now available and compiles successfully
//! (st-rvzl complete). This implementation provides an alternative direct
//! integration path if needed.

use super::stroma_store::StromaStore;
use super::traits::*;
use async_trait::async_trait;

/// Production Signal client implementation
///
/// Uses libsignal-service-rs directly without presage abstraction.
pub struct LibsignalClient {
    service_id: ServiceId,
    _store: StromaStore,
}

impl LibsignalClient {
    /// Create new libsignal client
    ///
    /// # Arguments
    /// * `service_id` - Bot's Signal service ID (ACI)
    /// * `store` - Protocol store for encryption state
    pub fn new(service_id: ServiceId, store: StromaStore) -> Self {
        Self {
            service_id,
            _store: store,
        }
    }
}

impl Clone for LibsignalClient {
    fn clone(&self) -> Self {
        // Note: Store cannot be cloned (contains mutex), so we create a reference
        // In production, this should use Arc<Mutex<Store>> or similar
        // For now, this is a placeholder
        todo!("LibsignalClient cloning requires Arc wrapper")
    }
}

#[async_trait]
impl SignalClient for LibsignalClient {
    async fn send_message(&self, _recipient: &ServiceId, _text: &str) -> SignalResult<()> {
        // TODO: Implement using PushService
        // 1. Create ContentMessage with text
        // 2. Encrypt message for recipient
        // 3. Send via PushService.send_message()
        Err(SignalError::NotImplemented("send_message".to_string()))
    }

    async fn send_group_message(&self, _group: &GroupId, _text: &str) -> SignalResult<()> {
        // TODO: Implement using PushService
        // 1. Create ContentMessage with text
        // 2. Encrypt message for all group members
        // 3. Send via PushService.send_message() to each member
        Err(SignalError::NotImplemented(
            "send_group_message".to_string(),
        ))
    }

    async fn create_group(&self, _name: &str) -> SignalResult<GroupId> {
        // TODO: Implement using GroupsV2Manager
        // 1. Generate group master key
        // 2. Create group with name
        // 3. Return group ID
        Err(SignalError::NotImplemented("create_group".to_string()))
    }

    async fn add_group_member(&self, _group: &GroupId, _member: &ServiceId) -> SignalResult<()> {
        // TODO: Implement using GroupsV2Manager
        // 1. Fetch current group state
        // 2. Add member to group
        // 3. Commit group change
        Err(SignalError::NotImplemented("add_group_member".to_string()))
    }

    async fn remove_group_member(&self, _group: &GroupId, _member: &ServiceId) -> SignalResult<()> {
        // TODO: Implement using GroupsV2Manager
        // 1. Fetch current group state
        // 2. Remove member from group
        // 3. Commit group change
        Err(SignalError::NotImplemented(
            "remove_group_member".to_string(),
        ))
    }

    async fn create_poll(&self, _group: &GroupId, _poll: &Poll) -> SignalResult<u64> {
        // TODO: Implement using Protocol v8 poll support
        // 1. Create PollCreate message
        // 2. Send to group
        // 3. Return poll timestamp as ID
        Err(SignalError::NotImplemented("create_poll".to_string()))
    }

    async fn terminate_poll(&self, _group: &GroupId, _poll_timestamp: u64) -> SignalResult<()> {
        // TODO: Implement using Protocol v8 poll support
        // 1. Create PollTerminate message
        // 2. Send to group
        Err(SignalError::NotImplemented("terminate_poll".to_string()))
    }

    async fn receive_messages(&self) -> SignalResult<Vec<Message>> {
        // TODO: Implement using SignalWebSocket
        // 1. Connect to Signal WebSocket
        // 2. Receive envelope
        // 3. Decrypt and parse message
        // 4. Return parsed messages
        Err(SignalError::NotImplemented("receive_messages".to_string()))
    }

    fn service_id(&self) -> &ServiceId {
        &self.service_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_client(test_name: &str) -> LibsignalClient {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = format!("/tmp/test_libsignal_client_{}_{}.db", test_name, timestamp);
        let service_id = ServiceId("test".to_string());
        let store = StromaStore::open(&db_path, "passphrase".to_string())
            .await
            .expect("Failed to open test store");
        LibsignalClient::new(service_id, store)
    }

    #[tokio::test]
    async fn test_client_creation() {
        let _client = create_test_client("client_creation").await;
    }

    #[tokio::test]
    async fn test_create_group() {
        // Arrange
        let client = create_test_client("create_group").await;
        let group_name = "Test Group";

        // Act
        let result = client.create_group(group_name).await;

        // Assert
        assert!(result.is_ok(), "create_group should succeed");
        let group_id = result.unwrap();
        assert!(!group_id.0.is_empty(), "GroupId should not be empty");
    }

    #[tokio::test]
    async fn test_add_group_member() {
        // Arrange
        let client = create_test_client("add_group_member").await;
        let group_name = "Test Group";
        let group_id = client.create_group(group_name).await.unwrap();
        let member_id = ServiceId("member-aci".to_string());

        // Act
        let result = client.add_group_member(&group_id, &member_id).await;

        // Assert
        assert!(result.is_ok(), "add_group_member should succeed");
    }

    #[tokio::test]
    async fn test_remove_group_member() {
        // Arrange
        let client = create_test_client("remove_group_member").await;
        let group_name = "Test Group";
        let group_id = client.create_group(group_name).await.unwrap();
        let member_id = ServiceId("member-aci".to_string());

        // Add member first
        client.add_group_member(&group_id, &member_id).await.unwrap();

        // Act
        let result = client.remove_group_member(&group_id, &member_id).await;

        // Assert
        assert!(result.is_ok(), "remove_group_member should succeed");
    }

    #[tokio::test]
    async fn test_create_group_with_empty_name() {
        // Arrange
        let client = create_test_client("create_group_empty_name").await;
        let group_name = "";

        // Act
        let result = client.create_group(group_name).await;

        // Assert
        // Empty names should be allowed - Signal Protocol doesn't enforce this
        assert!(result.is_ok(), "create_group with empty name should succeed");
    }

    #[tokio::test]
    async fn test_add_group_member_to_nonexistent_group() {
        // Arrange
        let client = create_test_client("add_member_nonexistent_group").await;
        let fake_group_id = GroupId(vec![1, 2, 3, 4]);
        let member_id = ServiceId("member-aci".to_string());

        // Act
        let result = client.add_group_member(&fake_group_id, &member_id).await;

        // Assert
        assert!(result.is_err(), "add_group_member to nonexistent group should fail");
        match result {
            Err(SignalError::GroupNotFound(_)) => {},
            _ => panic!("Expected GroupNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_remove_nonexistent_member() {
        // Arrange
        let client = create_test_client("remove_nonexistent_member").await;
        let group_name = "Test Group";
        let group_id = client.create_group(group_name).await.unwrap();
        let member_id = ServiceId("nonexistent-member".to_string());

        // Act
        let result = client.remove_group_member(&group_id, &member_id).await;

        // Assert
        assert!(result.is_err(), "remove_group_member for nonexistent member should fail");
        match result {
            Err(SignalError::MemberNotFound(_)) => {},
            _ => panic!("Expected MemberNotFound error"),
        }
    }
}
