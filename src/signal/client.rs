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
use tracing::{debug, error, info, warn};

/// Production Signal client implementation
///
/// When constructed with a presage Manager (`with_manager`), delegates
/// operations to the real Signal Protocol. Without a Manager (`new`),
/// uses in-memory stubs suitable for testing.
///
/// ## Websocket Architecture
///
/// The Manager is cloned: one copy runs a persistent `receive_messages` stream
/// in a background `spawn_local` task, feeding messages to an mpsc channel.
/// The other copy (behind `Arc<Mutex>`) handles sends (create_group, send_message, etc).
/// Both share the underlying store via Arc, avoiding the 4409 "Connected elsewhere"
/// error that occurred when receive_messages recreated websockets every poll cycle.
pub struct LibsignalClient {
    service_id: ServiceId,
    store: StromaStore,
    /// Manager for send operations (create_group, send_message, etc.)
    manager: Option<Arc<Mutex<Manager<StromaStore, Registered>>>>,
    /// Cloned Manager for the receive loop (consumed by start_receive_loop)
    manager_for_receive: Option<Manager<StromaStore, Registered>>,
    /// Channel receiver for incoming messages (populated by background receive task)
    message_rx: Option<Arc<std::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<Message>>>>,
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
            manager_for_receive: None,
            message_rx: None,
            group_keys: Arc::new(Mutex::new(HashMap::new())),
            group_members: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create client with a presage Manager (for production)
    ///
    /// The manager is cloned: the original is used for sends, the clone
    /// is reserved for the persistent receive_messages stream.
    pub fn with_manager(
        service_id: ServiceId,
        store: StromaStore,
        manager: Manager<StromaStore, Registered>,
    ) -> Self {
        let manager_for_receive = manager.clone();
        Self {
            service_id,
            store,
            manager: Some(Arc::new(Mutex::new(manager))),
            manager_for_receive: Some(manager_for_receive),
            message_rx: None,
            group_keys: Arc::new(Mutex::new(HashMap::new())),
            group_members: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Load existing groups from presage store
    ///
    /// Populates group_keys and group_members from persisted group data.
    /// Should be called after construction to restore state after restart.
    pub async fn load_groups_from_store(&mut self) -> SignalResult<()> {
        use presage::store::ContentsStore;
        use tracing::info;

        let manager = self.manager.as_ref().ok_or_else(|| {
            SignalError::NotImplemented("load_groups_from_store: no Manager configured".to_string())
        })?;

        let mgr = manager.lock().await;
        let groups = mgr.store().groups().await.map_err(|e| {
            SignalError::Store(format!("Failed to load groups from store: {:?}", e))
        })?;

        let mut group_keys = self.group_keys.lock().await;
        let mut group_members = self.group_members.lock().await;

        for group_result in groups {
            let (master_key, group) = group_result
                .map_err(|e| SignalError::Store(format!("Failed to load group: {:?}", e)))?;

            let group_id = GroupId(master_key.to_vec());

            group_keys.insert(group_id.clone(), master_key);

            // Initialize empty member set (will be populated as we see members)
            group_members.entry(group_id.clone()).or_default();

            info!(
                "Loaded group from store: {} ({} members in revision {})",
                hex::encode(master_key),
                group.members.len(),
                group.revision
            );
        }

        info!("Loaded {} group(s) from presage store", group_keys.len());

        Ok(())
    }

    /// Start the background receive loop.
    ///
    /// Must be called from within a `tokio::task::LocalSet` context (uses `spawn_local`).
    /// Consumes the receive Manager clone and spawns a background task that feeds
    /// incoming messages to an mpsc channel. Call this once before the bot event loop.
    pub async fn start_receive_loop(&mut self) -> SignalResult<()> {
        let mut manager = self.manager_for_receive.take().ok_or_else(|| {
            SignalError::NotImplemented(
                "start_receive_loop: no Manager available (already started or test mode)"
                    .to_string(),
            )
        })?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        self.message_rx = Some(Arc::new(std::sync::Mutex::new(rx)));

        info!("starting persistent receive_messages stream");

        tokio::task::spawn_local(async move {
            use futures::StreamExt;

            let stream = match manager.receive_messages().await {
                Ok(s) => s,
                Err(e) => {
                    error!("failed to start receive_messages stream: {:?}", e);
                    return;
                }
            };

            futures::pin_mut!(stream);

            while let Some(received) = stream.next().await {
                if let Some(msg) = convert_received(received) {
                    if tx.send(msg).is_err() {
                        debug!("message channel closed, stopping receive loop");
                        break;
                    }
                }
            }

            warn!("receive_messages stream ended");
        });

        Ok(())
    }
}

impl Clone for LibsignalClient {
    fn clone(&self) -> Self {
        Self {
            service_id: self.service_id.clone(),
            store: self.store.clone(),
            manager: self.manager.as_ref().map(Arc::clone),
            manager_for_receive: None, // Clone doesn't get the receive manager
            message_rx: self.message_rx.as_ref().map(Arc::clone),
            group_keys: Arc::clone(&self.group_keys),
            group_members: Arc::clone(&self.group_members),
        }
    }
}

#[async_trait(?Send)]
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

        let mut mgr = manager.lock().await;
        mgr.send_message(
            presage_service_id,
            ContentBody::DataMessage(data_message),
            timestamp,
        )
        .await
        .map_err(|e| SignalError::Network(format!("send_message failed: {:?}", e)))?;

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
        use presage::proto::{DataMessage, GroupContextV2};

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let data_message = DataMessage {
            body: Some(text.to_string()),
            timestamp: Some(timestamp),
            group_v2: Some(GroupContextV2 {
                master_key: Some(master_key.to_vec()),
                revision: Some(0),
                ..Default::default()
            }),
            ..Default::default()
        };

        let mut mgr = manager.lock().await;
        mgr.send_message_to_group(
            &master_key,
            ContentBody::DataMessage(data_message),
            timestamp,
        )
        .await
        .map_err(|e| SignalError::Network(format!("send_group_message failed: {:?}", e)))?;

        Ok(())
    }

    async fn create_group(
        &self,
        name: &str,
        members: &[ServiceId],
    ) -> SignalResult<(GroupId, Vec<ServiceId>)> {
        if let Some(manager) = &self.manager {
            use presage::store::ContentsStore;

            let mut mgr = manager.lock().await;

            // Look up profile keys for each member (None if not found)
            let mut members_with_keys = Vec::new();
            for member in members {
                let uuid: uuid::Uuid = member.0.parse().map_err(|e| {
                    SignalError::InvalidMessage(format!("Invalid UUID '{}': {}", member.0, e))
                })?;
                let aci: presage::libsignal_service::protocol::Aci = uuid.into();
                let service_id: presage::libsignal_service::protocol::ServiceId = aci.into();

                // Try contacts first, then profile_keys store
                let profile_key: Option<presage::libsignal_service::zkgroup::profiles::ProfileKey> =
                    if let Some(pk) = mgr
                        .store()
                        .contact_by_id(&uuid)
                        .await
                        .ok()
                        .flatten()
                        .and_then(|c| <Vec<u8> as TryInto<[u8; 32]>>::try_into(c.profile_key).ok())
                        .map(presage::libsignal_service::zkgroup::profiles::ProfileKey::create)
                    {
                        Some(pk)
                    } else {
                        mgr.store().profile_key(&service_id).await.ok().flatten()
                    };

                // None = member will be added as pending invite (no profile key needed)
                members_with_keys.push((aci, profile_key));
            }

            let (master_key, pending_sids) = mgr
                .create_group(name, members_with_keys)
                .await
                .map_err(|e| SignalError::Network(format!("create_group failed: {:?}", e)))?;

            let group_id = GroupId(master_key.to_vec());

            {
                let mut keys = self.group_keys.lock().await;
                keys.insert(group_id.clone(), master_key);
            }
            {
                let mut member_set = self.group_members.lock().await;
                let set = member_set.entry(group_id.clone()).or_default();
                for m in members {
                    set.insert(m.clone());
                }
            }

            // Convert presage ServiceIds to stroma ServiceIds
            let pending_members: Vec<ServiceId> = pending_sids
                .iter()
                .map(|sid| ServiceId(sid.service_id_string()))
                .collect();

            Ok((group_id, pending_members))
        } else {
            // Stub: in-memory only (for tests without Manager)
            let group_id = GroupId(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
            let master_key = [0u8; 32];
            {
                let mut keys = self.group_keys.lock().await;
                keys.insert(group_id.clone(), master_key);
            }
            {
                let mut member_set = self.group_members.lock().await;
                let set = member_set.entry(group_id.clone()).or_default();
                for m in members {
                    set.insert(m.clone());
                }
            }
            Ok((group_id, vec![]))
        }
    }

    async fn send_group_invite(&self, group: &GroupId, member: &ServiceId) -> SignalResult<()> {
        if let Some(manager) = &self.manager {
            let master_key = {
                let keys = self.group_keys.lock().await;
                *keys.get(group).ok_or_else(|| {
                    SignalError::GroupNotFound(format!("Group not found: {}", group))
                })?
            };

            let uuid: uuid::Uuid = member.0.parse().map_err(|e| {
                SignalError::InvalidMessage(format!("Invalid UUID '{}': {}", member.0, e))
            })?;
            let aci: presage::libsignal_service::protocol::Aci = uuid.into();

            let mut mgr = manager.lock().await;
            mgr.send_group_invite_dm(&master_key, aci)
                .await
                .map_err(|e| {
                    SignalError::Network(format!("send_group_invite_dm failed: {:?}", e))
                })?;

            Ok(())
        } else {
            // Stub: no-op for tests without Manager
            Ok(())
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

    async fn create_poll(
        &self,
        group: &GroupId,
        question: &str,
        options: Vec<String>,
        allow_multiple: bool,
    ) -> SignalResult<u64> {
        let manager = self.manager.as_ref().ok_or_else(|| {
            SignalError::NotImplemented("create_poll: no Manager configured".to_string())
        })?;

        let master_key = {
            let keys = self.group_keys.lock().await;
            *keys.get(group).ok_or_else(|| {
                SignalError::GroupNotFound(format!("No master key for group: {}", group))
            })?
        };

        let mut mgr = manager.lock().await;
        Ok(mgr
            .send_poll(&master_key, question, options, allow_multiple)
            .await
            .map_err(|e| SignalError::Network(format!("create_poll failed: {:?}", e)))?)
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

        let mut mgr = manager.lock().await;
        mgr.terminate_poll(&master_key, poll_timestamp)
            .await
            .map_err(|e| SignalError::Network(format!("terminate_poll failed: {:?}", e)))?;

        Ok(())
    }

    async fn get_group_info(&self, group: &GroupId) -> SignalResult<GroupInfo> {
        let manager = self.manager.as_ref().ok_or_else(|| {
            SignalError::NotImplemented("get_group_info: no Manager configured".to_string())
        })?;

        let master_key = {
            let keys = self.group_keys.lock().await;
            *keys.get(group).ok_or_else(|| {
                SignalError::GroupNotFound(format!("No master key for group: {}", group))
            })?
        };

        use presage::store::ContentsStore;

        let mgr = manager.lock().await;
        let presage_group: presage::model::groups::Group = mgr
            .store()
            .group(master_key)
            .await
            .map_err(|e| SignalError::Store(format!("Failed to get group: {:?}", e)))?
            .ok_or_else(|| SignalError::GroupNotFound("Group not in store".to_string()))?;

        Ok(GroupInfo {
            name: presage_group.title,
            description: presage_group.description,
            disappearing_messages_timer: presage_group
                .disappearing_messages_timer
                .map(|t| t.duration),
            // TODO: Update when presage Group struct includes announcements_only field
            announcements_only: false,
        })
    }

    async fn set_group_name(&self, group: &GroupId, name: &str) -> SignalResult<()> {
        let manager = self.manager.as_ref().ok_or_else(|| {
            SignalError::NotImplemented("set_group_name: no Manager configured".to_string())
        })?;

        let master_key = {
            let keys = self.group_keys.lock().await;
            *keys.get(group).ok_or_else(|| {
                SignalError::GroupNotFound(format!("No master key for group: {}", group))
            })?
        };

        let mut mgr = manager.lock().await;
        mgr.update_group_title(&master_key, name)
            .await
            .map_err(|e| SignalError::Network(format!("set_group_name failed: {:?}", e)))?;

        Ok(())
    }

    async fn set_group_description(
        &self,
        _group: &GroupId,
        _description: &str,
    ) -> SignalResult<()> {
        // TODO: Wire to presage Manager when set_group_description is available
        // Method exists in latest presage but not yet in our dependency version
        Err(SignalError::NotImplemented(
            "set_group_description: presage API not yet available".to_string(),
        ))
    }

    async fn set_disappearing_messages(&self, _group: &GroupId, _seconds: u32) -> SignalResult<()> {
        // TODO: Wire to presage Manager when set_disappearing_messages_timer is available
        // Method exists in latest presage but not yet in our dependency version
        Err(SignalError::NotImplemented(
            "set_disappearing_messages: presage API not yet available".to_string(),
        ))
    }

    async fn set_announcements_only(&self, _group: &GroupId, _enabled: bool) -> SignalResult<()> {
        // TODO: Wire to presage Manager when set_group_announcements_only is available
        // Method exists in latest presage but not yet in our dependency version
        Err(SignalError::NotImplemented(
            "set_announcements_only: presage API not yet available".to_string(),
        ))
    }

    async fn resolve_identifier(&self, identifier: &str) -> SignalResult<ServiceId> {
        use crate::signal::pm::{parse_identifier, Identifier};

        let parsed = parse_identifier(identifier);

        match parsed {
            Identifier::Uuid(uuid_str) => {
                // Validate ServiceId format (supports both plain UUIDs and prefixed forms like "PNI:..." or "ACI:...")
                presage::libsignal_service::protocol::ServiceId::parse_from_service_id_string(
                    &uuid_str,
                )
                .ok_or_else(|| {
                    SignalError::InvalidMessage(format!("Invalid ServiceId '{}'", uuid_str))
                })?;
                Ok(ServiceId(uuid_str))
            }
            Identifier::Username(username) => {
                let manager = self.manager.as_ref().ok_or_else(|| {
                    SignalError::NotImplemented(
                        "resolve_identifier (username): no Manager configured".to_string(),
                    )
                })?;

                let mut mgr = manager.lock().await;
                let aci = mgr.resolve_username(&username).await.map_err(|e| {
                    SignalError::Network(format!("resolve_username failed: {:?}", e))
                })?;

                match aci {
                    Some(aci) => Ok(ServiceId(aci.service_id_string())),
                    None => Err(SignalError::InvalidMessage(format!(
                        "Username '{}' not found on Signal",
                        username
                    ))),
                }
            }
            Identifier::Phone(phone) => {
                let manager = self.manager.as_ref().ok_or_else(|| {
                    SignalError::NotImplemented(
                        "resolve_identifier (phone): no Manager configured".to_string(),
                    )
                })?;

                let mut mgr = manager.lock().await;
                let service_id = mgr.resolve_phone_number(&phone).await.map_err(|e| {
                    SignalError::Network(format!("resolve_phone_number failed: {:?}", e))
                })?;

                match service_id {
                    Some(id) => Ok(ServiceId(id.service_id_string())),
                    None => Err(SignalError::InvalidMessage(format!(
                        "Phone number '{}' not found on Signal",
                        phone
                    ))),
                }
            }
        }
    }

    async fn receive_messages(&self) -> SignalResult<Vec<Message>> {
        let rx = self.message_rx.as_ref().ok_or_else(|| {
            SignalError::NotImplemented(
                "receive_messages: receive loop not started (call start_receive_loop first)"
                    .to_string(),
            )
        })?;

        let mut messages = Vec::new();
        let mut rx_guard = rx.lock().unwrap();
        while let Ok(msg) = rx_guard.try_recv() {
            messages.push(msg);
        }
        Ok(messages)
    }

    fn service_id(&self) -> &ServiceId {
        &self.service_id
    }

    async fn list_groups(&self) -> SignalResult<Vec<(GroupId, usize)>> {
        let keys = self.group_keys.lock().await;
        let members = self.group_members.lock().await;

        let mut groups = Vec::new();
        for (group_id, _master_key) in keys.iter() {
            let member_count = members.get(group_id).map(|m| m.len()).unwrap_or(0);
            groups.push((group_id.clone(), member_count));
        }

        Ok(groups)
    }

    async fn leave_group(&self, group: &GroupId) -> SignalResult<()> {
        use tracing::info;

        let manager = self.manager.as_ref().ok_or_else(|| {
            SignalError::NotImplemented("leave_group: no Manager configured".to_string())
        })?;

        let master_key = {
            let keys = self.group_keys.lock().await;
            *keys.get(group).ok_or_else(|| {
                SignalError::GroupNotFound(format!("Cannot leave unknown group: {}", group))
            })?
        };

        // Call presage's leave_group (removes bot from group in Signal)
        let mut mgr = manager.lock().await;
        mgr.leave_group(&master_key)
            .await
            .map_err(|e| SignalError::Network(format!("leave_group failed: {:?}", e)))?;

        // Remove from in-memory tracking
        {
            let mut keys = self.group_keys.lock().await;
            keys.remove(group);
        }
        {
            let mut members = self.group_members.lock().await;
            members.remove(group);
        }

        info!("✅ Successfully left group: {}", group);

        Ok(())
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
                    // Determine message source (DM vs group)
                    // For group messages, extract and include the actual GroupId
                    let source = if let Some(group_v2) = &dm.group_v2 {
                        // Message is from a group context
                        if let Some(master_key_bytes) = &group_v2.master_key {
                            MessageSource::Group(GroupId(master_key_bytes.clone()))
                        } else {
                            // Group message without master key - treat as DM (shouldn't happen)
                            MessageSource::DirectMessage
                        }
                    } else {
                        // Message is a direct message (1-on-1 PM)
                        MessageSource::DirectMessage
                    };

                    return Some(Message {
                        sender: ServiceId(sender_id),
                        source,
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
        let db_dir = format!("/tmp/test_libsignal_client_{}_{}", test_name, timestamp);
        let service_id = ServiceId("test".to_string());
        let store = StromaStore::open(&db_dir, "passphrase".to_string())
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

        let result = client.create_group(group_name, &[]).await;

        assert!(result.is_ok(), "create_group should succeed");
        let (group_id, _pending) = result.unwrap();
        assert!(!group_id.0.is_empty(), "GroupId should not be empty");
    }

    #[tokio::test]
    async fn test_add_group_member() {
        let client = create_test_client("add_group_member").await;
        let group_name = "Test Group";
        let (group_id, _pending) = client.create_group(group_name, &[]).await.unwrap();
        let member_id = ServiceId("member-aci".to_string());

        let result = client.add_group_member(&group_id, &member_id).await;

        assert!(result.is_ok(), "add_group_member should succeed");
    }

    #[tokio::test]
    async fn test_remove_group_member() {
        let client = create_test_client("remove_group_member").await;
        let group_name = "Test Group";
        let (group_id, _pending) = client.create_group(group_name, &[]).await.unwrap();
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

        let result = client.create_group(group_name, &[]).await;

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
        let (group_id, _pending) = client.create_group(group_name, &[]).await.unwrap();
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
        let (group_id, _pending) = client.create_group("Test Group", &[]).await.unwrap();

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
        let (group_id, _pending) = client.create_group("Test Group", &[]).await.unwrap();

        let result = client
            .create_poll(
                &group_id,
                "Test?",
                vec!["Yes".to_string(), "No".to_string()],
                false,
            )
            .await;

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
        let (group_id, _pending) = client.create_group("Test Group", &[]).await.unwrap();

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

        let (group_id, _pending) = client.create_group("Shared Group", &[]).await.unwrap();

        let member = ServiceId("new-member".to_string());
        let result = cloned.add_group_member(&group_id, &member).await;
        assert!(
            result.is_ok(),
            "Clone should share group state with original"
        );
    }
}
