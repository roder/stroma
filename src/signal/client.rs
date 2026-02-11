//! Production Signal Client Implementation
//!
//! Implements SignalClient trait using presage Manager for Signal Protocol operations.
//! Falls back to stub implementations when no Manager is available (e.g., tests).
//!
//! ## Send workaround
//!
//! Presage Manager's async methods produce !Send futures (due to ThreadRng and
//! non-Send protocol store trait objects). Since `#[async_trait]` requires Send
//! futures, we use `spawn_blocking` + `Handle::block_on` to run Manager calls
//! on a blocking thread where Send isn't required.

use super::stroma_store::StromaStore;
use super::traits::*;
use async_trait::async_trait;
use presage::manager::{Manager, Registered};
use presage::model::messages::Received;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Production Signal client implementation
///
/// When constructed with a presage Manager (`with_manager`), delegates
/// operations to the real Signal Protocol. Without a Manager (`new`),
/// uses in-memory stubs suitable for testing.
pub struct LibsignalClient {
    service_id: ServiceId,
    store: StromaStore,
    manager: Option<Arc<Mutex<Manager<StromaStore, Registered>>>>,
    /// Maps group ID to master key bytes
    group_keys: Arc<Mutex<HashMap<GroupId, [u8; 32]>>>,
    /// Tracks group members in-memory
    group_members: Arc<Mutex<HashMap<GroupId, HashSet<ServiceId>>>>,
}

impl LibsignalClient {
    /// Create client without a Manager (for testing)
    pub fn new(service_id: ServiceId, store: StromaStore) -> Self {
        Self {
            service_id,
            store,
            manager: None,
            group_keys: Arc::new(Mutex::new(HashMap::new())),
            group_members: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create client with a presage Manager (for production)
    pub fn with_manager(
        service_id: ServiceId,
        store: StromaStore,
        manager: Manager<StromaStore, Registered>,
    ) -> Self {
        Self {
            service_id,
            store,
            manager: Some(Arc::new(Mutex::new(manager))),
            group_keys: Arc::new(Mutex::new(HashMap::new())),
            group_members: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Clone for LibsignalClient {
    fn clone(&self) -> Self {
        Self {
            service_id: self.service_id.clone(),
            store: self.store.clone(),
            manager: self.manager.as_ref().map(Arc::clone),
            group_keys: Arc::clone(&self.group_keys),
            group_members: Arc::clone(&self.group_members),
        }
    }
}

/// Run a !Send presage Manager future on a blocking thread.
///
/// Presage Manager methods produce futures that capture `ThreadRng` and non-Send
/// protocol store trait objects. This helper moves the async work to a blocking
/// thread where Send constraints don't apply.
async fn run_manager_op<F, T>(
    manager: &Arc<Mutex<Manager<StromaStore, Registered>>>,
    op: F,
) -> Result<T, SignalError>
where
    F: FnOnce(
            &mut Manager<StromaStore, Registered>,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = T> + '_>>
        + Send
        + 'static,
    T: Send + 'static,
{
    let manager = Arc::clone(manager);
    let handle = tokio::runtime::Handle::current();

    tokio::task::spawn_blocking(move || {
        handle.block_on(async {
            let mut mgr = manager.lock().await;
            op(&mut mgr).await
        })
    })
    .await
    .map_err(|e| SignalError::Network(format!("Manager task failed: {:?}", e)))
}

#[async_trait]
impl SignalClient for LibsignalClient {
    async fn send_message(&self, recipient: &ServiceId, text: &str) -> SignalResult<()> {
        let manager = self.manager.as_ref().ok_or_else(|| {
            SignalError::NotImplemented("send_message: no Manager configured".to_string())
        })?;

        use presage::libsignal_service::content::ContentBody;
        use presage::proto::DataMessage;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let data_message = DataMessage {
            body: Some(text.to_string()),
            timestamp: Some(timestamp),
            ..Default::default()
        };

        let presage_service_id =
            presage::libsignal_service::protocol::ServiceId::parse_from_service_id_string(
                &recipient.0,
            )
            .ok_or_else(|| SignalError::Protocol(format!("Invalid service ID: {}", recipient.0)))?;

        run_manager_op(manager, move |mgr| {
            Box::pin(async move {
                mgr.send_message(
                    presage_service_id,
                    ContentBody::DataMessage(data_message),
                    timestamp,
                )
                .await
                .map_err(|e| SignalError::Network(format!("send_message failed: {:?}", e)))
            })
        })
        .await??;

        Ok(())
    }

    async fn send_group_message(&self, group: &GroupId, text: &str) -> SignalResult<()> {
        let manager = self.manager.as_ref().ok_or_else(|| {
            SignalError::NotImplemented("send_group_message: no Manager configured".to_string())
        })?;

        let master_key = {
            let keys = self.group_keys.lock().await;
            *keys.get(group).ok_or_else(|| {
                SignalError::GroupNotFound(format!("No master key for group: {}", group))
            })?
        };

        use presage::libsignal_service::content::ContentBody;
        use presage::proto::DataMessage;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let data_message = DataMessage {
            body: Some(text.to_string()),
            timestamp: Some(timestamp),
            ..Default::default()
        };

        run_manager_op(manager, move |mgr| {
            Box::pin(async move {
                mgr.send_message_to_group(
                    &master_key,
                    ContentBody::DataMessage(data_message),
                    timestamp,
                )
                .await
                .map_err(|e| SignalError::Network(format!("send_group_message failed: {:?}", e)))
            })
        })
        .await??;

        Ok(())
    }

    async fn create_group(&self, name: &str) -> SignalResult<GroupId> {
        if let Some(manager) = &self.manager {
            let name = name.to_string();
            let master_key = run_manager_op(manager, move |mgr| {
                Box::pin(async move {
                    mgr.create_group(name, vec![])
                        .await
                        .map_err(|e| SignalError::Network(format!("create_group failed: {:?}", e)))
                })
            })
            .await??;

            let group_id = GroupId(master_key.to_vec());

            {
                let mut keys = self.group_keys.lock().await;
                keys.insert(group_id.clone(), master_key);
            }
            {
                let mut members = self.group_members.lock().await;
                members.insert(group_id.clone(), HashSet::new());
            }

            Ok(group_id)
        } else {
            // Stub: in-memory only (for tests without Manager)
            let group_id = GroupId(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
            let master_key = [0u8; 32];
            {
                let mut keys = self.group_keys.lock().await;
                keys.insert(group_id.clone(), master_key);
            }
            {
                let mut members = self.group_members.lock().await;
                members.insert(group_id.clone(), HashSet::new());
            }
            Ok(group_id)
        }
    }

    async fn add_group_member(&self, group: &GroupId, member: &ServiceId) -> SignalResult<()> {
        {
            let keys = self.group_keys.lock().await;
            if !keys.contains_key(group) {
                return Err(SignalError::GroupNotFound(format!(
                    "Group not found: {}",
                    group
                )));
            }
        }

        {
            let mut members = self.group_members.lock().await;
            members
                .entry(group.clone())
                .or_insert_with(HashSet::new)
                .insert(member.clone());
        }

        // TODO: Wire to presage Manager group membership update when GV2 APIs available
        Ok(())
    }

    async fn remove_group_member(&self, group: &GroupId, member: &ServiceId) -> SignalResult<()> {
        {
            let keys = self.group_keys.lock().await;
            if !keys.contains_key(group) {
                return Err(SignalError::MemberNotFound(format!(
                    "Member not found in unknown group: {}",
                    group
                )));
            }
        }

        {
            let mut members = self.group_members.lock().await;
            let group_members = members.get_mut(group).ok_or_else(|| {
                SignalError::MemberNotFound(format!("Group not found: {}", group))
            })?;

            if !group_members.remove(member) {
                return Err(SignalError::MemberNotFound(format!(
                    "Member not found: {}",
                    member.0
                )));
            }
        }

        // TODO: Wire to presage Manager group membership update when GV2 APIs available
        Ok(())
    }

    async fn create_poll(&self, group: &GroupId, poll: &Poll) -> SignalResult<u64> {
        let manager = self.manager.as_ref().ok_or_else(|| {
            SignalError::NotImplemented("create_poll: no Manager configured".to_string())
        })?;

        let master_key = {
            let keys = self.group_keys.lock().await;
            *keys.get(group).ok_or_else(|| {
                SignalError::GroupNotFound(format!("No master key for group: {}", group))
            })?
        };

        let question = poll.question.clone();
        let options = poll.options.clone();

        run_manager_op(manager, move |mgr| {
            Box::pin(async move {
                mgr.send_poll(&master_key, question, options, false)
                    .await
                    .map_err(|e| SignalError::Network(format!("create_poll failed: {:?}", e)))
            })
        })
        .await?
    }

    async fn terminate_poll(&self, group: &GroupId, poll_timestamp: u64) -> SignalResult<()> {
        let manager = self.manager.as_ref().ok_or_else(|| {
            SignalError::NotImplemented("terminate_poll: no Manager configured".to_string())
        })?;

        let master_key = {
            let keys = self.group_keys.lock().await;
            *keys.get(group).ok_or_else(|| {
                SignalError::GroupNotFound(format!("No master key for group: {}", group))
            })?
        };

        run_manager_op(manager, move |mgr| {
            Box::pin(async move {
                mgr.terminate_poll(&master_key, poll_timestamp)
                    .await
                    .map_err(|e| SignalError::Network(format!("terminate_poll failed: {:?}", e)))
            })
        })
        .await??;

        Ok(())
    }

    async fn receive_messages(&self) -> SignalResult<Vec<Message>> {
        let manager = self.manager.as_ref().ok_or_else(|| {
            SignalError::NotImplemented("receive_messages: no Manager configured".to_string())
        })?;

        use futures::StreamExt;

        let manager = Arc::clone(manager);
        let handle = tokio::runtime::Handle::current();

        let messages = tokio::task::spawn_blocking(move || {
            handle.block_on(async {
                let mut mgr = manager.lock().await;
                let stream = mgr.receive_messages().await.map_err(|e| {
                    SignalError::Network(format!("receive_messages failed: {:?}", e))
                })?;

                let mut messages = Vec::new();
                futures::pin_mut!(stream);
                while let Some(received) = stream.next().await {
                    if let Some(msg) = convert_received(received) {
                        messages.push(msg);
                    }
                    if !messages.is_empty() {
                        break;
                    }
                }

                Ok::<_, SignalError>(messages)
            })
        })
        .await
        .map_err(|e| SignalError::Network(format!("receive task failed: {:?}", e)))??;

        Ok(messages)
    }

    fn service_id(&self) -> &ServiceId {
        &self.service_id
    }
}

/// Convert presage Received message to our Message type
fn convert_received(received: Received) -> Option<Message> {
    match received {
        Received::Content(content) => {
            let sender_id = content.metadata.sender.raw_uuid().to_string();
            let timestamp = content.metadata.timestamp;

            if let presage::libsignal_service::content::ContentBody::DataMessage(dm) = &content.body
            {
                if let Some(body) = &dm.body {
                    return Some(Message {
                        sender: ServiceId(sender_id),
                        content: MessageContent::Text(body.clone()),
                        timestamp,
                    });
                }
            }
            None
        }
        _ => None,
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
        let client = create_test_client("create_group").await;
        let group_name = "Test Group";

        let result = client.create_group(group_name).await;

        assert!(result.is_ok(), "create_group should succeed");
        let group_id = result.unwrap();
        assert!(!group_id.0.is_empty(), "GroupId should not be empty");
    }

    #[tokio::test]
    async fn test_add_group_member() {
        let client = create_test_client("add_group_member").await;
        let group_name = "Test Group";
        let group_id = client.create_group(group_name).await.unwrap();
        let member_id = ServiceId("member-aci".to_string());

        let result = client.add_group_member(&group_id, &member_id).await;

        assert!(result.is_ok(), "add_group_member should succeed");
    }

    #[tokio::test]
    async fn test_remove_group_member() {
        let client = create_test_client("remove_group_member").await;
        let group_name = "Test Group";
        let group_id = client.create_group(group_name).await.unwrap();
        let member_id = ServiceId("member-aci".to_string());

        client
            .add_group_member(&group_id, &member_id)
            .await
            .unwrap();

        let result = client.remove_group_member(&group_id, &member_id).await;

        assert!(result.is_ok(), "remove_group_member should succeed");
    }

    #[tokio::test]
    async fn test_create_group_with_empty_name() {
        let client = create_test_client("create_group_empty_name").await;
        let group_name = "";

        let result = client.create_group(group_name).await;

        assert!(
            result.is_ok(),
            "create_group with empty name should succeed"
        );
    }

    #[tokio::test]
    async fn test_add_group_member_to_nonexistent_group() {
        let client = create_test_client("add_member_nonexistent_group").await;
        let fake_group_id = GroupId(vec![1, 2, 3, 4]);
        let member_id = ServiceId("member-aci".to_string());

        let result = client.add_group_member(&fake_group_id, &member_id).await;

        assert!(
            result.is_err(),
            "add_group_member to nonexistent group should fail"
        );
        match result {
            Err(SignalError::GroupNotFound(_)) => {}
            _ => panic!("Expected GroupNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_remove_nonexistent_member() {
        let client = create_test_client("remove_nonexistent_member").await;
        let group_name = "Test Group";
        let group_id = client.create_group(group_name).await.unwrap();
        let member_id = ServiceId("nonexistent-member".to_string());

        let result = client.remove_group_member(&group_id, &member_id).await;

        assert!(
            result.is_err(),
            "remove_group_member for nonexistent member should fail"
        );
        match result {
            Err(SignalError::MemberNotFound(_)) => {}
            _ => panic!("Expected MemberNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_send_message_not_implemented() {
        let client = create_test_client("send_message_not_impl").await;
        let recipient = ServiceId("recipient-aci".to_string());

        let result = client.send_message(&recipient, "hello").await;

        match result {
            Err(SignalError::NotImplemented(msg)) => {
                assert!(msg.contains("send_message"));
            }
            other => panic!("Expected NotImplemented, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_send_group_message_not_implemented() {
        let client = create_test_client("send_group_message_not_impl").await;
        let group_id = client.create_group("Test Group").await.unwrap();

        let result = client.send_group_message(&group_id, "hello group").await;

        match result {
            Err(SignalError::NotImplemented(msg)) => {
                assert!(msg.contains("send_group_message"));
            }
            other => panic!("Expected NotImplemented, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_create_poll_not_implemented() {
        let client = create_test_client("create_poll_not_impl").await;
        let group_id = client.create_group("Test Group").await.unwrap();
        let poll = Poll {
            question: "Test?".to_string(),
            options: vec!["Yes".to_string(), "No".to_string()],
        };

        let result = client.create_poll(&group_id, &poll).await;

        match result {
            Err(SignalError::NotImplemented(msg)) => {
                assert!(msg.contains("create_poll"));
            }
            other => panic!("Expected NotImplemented, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_terminate_poll_not_implemented() {
        let client = create_test_client("terminate_poll_not_impl").await;
        let group_id = client.create_group("Test Group").await.unwrap();

        let result = client.terminate_poll(&group_id, 12345).await;

        match result {
            Err(SignalError::NotImplemented(msg)) => {
                assert!(msg.contains("terminate_poll"));
            }
            other => panic!("Expected NotImplemented, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_receive_messages_not_implemented() {
        let client = create_test_client("receive_messages_not_impl").await;

        let result = client.receive_messages().await;

        match result {
            Err(SignalError::NotImplemented(msg)) => {
                assert!(msg.contains("receive_messages"));
            }
            other => panic!("Expected NotImplemented, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_service_id() {
        let client = create_test_client("service_id").await;

        assert_eq!(client.service_id(), &ServiceId("test".to_string()));
    }

    #[tokio::test]
    async fn test_clone_shares_state() {
        let client = create_test_client("clone_shares_state").await;
        let cloned = client.clone();

        let group_id = client.create_group("Shared Group").await.unwrap();

        let member = ServiceId("new-member".to_string());
        let result = cloned.add_group_member(&group_id, &member).await;
        assert!(
            result.is_ok(),
            "Clone should share group state with original"
        );
    }
}
