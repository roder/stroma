//! Stroma Store Wrapper
//!
//! Newtype wrapper around presage SqliteStore that:
//! - Delegates protocol state management (groups, contacts, profiles)
//! - No-ops message storage methods (server seizure protection)
//! - Encrypts with operator passphrase via SQLCipher
//!
//! See: .beads/security-constraints.bead § 10

use presage::libsignal_service::prelude::ProfileKey;
use presage::libsignal_service::prelude::Uuid;
use presage::libsignal_service::protocol::SenderCertificate;
use presage::libsignal_service::zkgroup::GroupMasterKeyBytes;
use presage::manager::RegistrationData;
use presage::model::contacts::Contact;
use presage::model::groups::Group;
use presage::store::{ContentsStore, StateStore, StickerPack, Store, Thread};
use presage::AvatarBytes;
use presage_store_sqlite::{OnNewIdentity, SqliteStore, SqliteStoreError};
use std::collections::HashMap;
use std::ops::RangeBounds;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

// TEMPORARY: Simple in-memory storage for poll state (testing only)
// TODO: Replace with actual SQLite table
static POLL_STATE_CACHE: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();

/// Stroma protocol store wrapper
///
/// CRITICAL: This wrapper delegates to SQLite for:
/// - Account configuration (ACI/PNI identity)
/// - Group membership state
/// - Protocol state (sessions, pre-keys, sender keys)
///
/// But explicitly NO-OPs message storage (server seizure protection).
/// See: security-constraints.bead §10
#[derive(Clone)]
pub struct StromaStore(SqliteStore);

impl StromaStore {
    /// Open StromaStore
    ///
    /// # Arguments
    /// * `path` - Path to SQLCipher database
    /// * `passphrase` - Encryption passphrase for SQLCipher
    pub async fn open(path: impl AsRef<Path>, passphrase: String) -> Result<Self, StoreError> {
        let path_str = path.as_ref().to_str().ok_or_else(|| {
            StoreError::Sqlite("Invalid path: contains non-UTF8 characters".to_string())
        })?;

        let inner = SqliteStore::open_with_passphrase(
            path_str,
            Some(&passphrase),
            OnNewIdentity::Trust, // Trust new identities by default
        )
        .await
        .map_err(|e| StoreError::Sqlite(format!("{:?}", e)))?;

        Ok(Self(inner))
    }

    /// Store arbitrary data (for poll state persistence).
    ///
    /// TEMPORARY: Uses in-memory cache for testing.
    /// TODO: Implement actual SQLite table for generic key-value storage.
    pub async fn store_data(&self, key: &str, value: &[u8]) -> Result<(), StoreError> {
        let cache = POLL_STATE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let mut cache_guard = cache.lock().unwrap();
        cache_guard.insert(key.to_string(), value.to_vec());
        Ok(())
    }

    /// Retrieve arbitrary data (for poll state restoration).
    ///
    /// TEMPORARY: Uses in-memory cache for testing.
    /// TODO: Implement actual SQLite table for generic key-value storage.
    pub async fn retrieve_data(&self, key: &str) -> Result<Option<Vec<u8>>, StoreError> {
        let cache = POLL_STATE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let cache_guard = cache.lock().unwrap();
        Ok(cache_guard.get(key).cloned())
    }
}

// Delegate StateStore trait (protocol state management)
impl StateStore for StromaStore {
    type StateStoreError = SqliteStoreError;

    async fn load_registration_data(
        &self,
    ) -> Result<Option<RegistrationData>, Self::StateStoreError> {
        self.0.load_registration_data().await
    }

    async fn save_registration_data(
        &mut self,
        state: &RegistrationData,
    ) -> Result<(), Self::StateStoreError> {
        self.0.save_registration_data(state).await
    }

    async fn is_registered(&self) -> bool {
        self.0.is_registered().await
    }

    async fn clear_registration(&mut self) -> Result<(), Self::StateStoreError> {
        self.0.clear_registration().await
    }

    async fn set_aci_identity_key_pair(
        &self,
        key_pair: presage::libsignal_service::protocol::IdentityKeyPair,
    ) -> Result<(), Self::StateStoreError> {
        self.0.set_aci_identity_key_pair(key_pair).await
    }

    async fn set_pni_identity_key_pair(
        &self,
        key_pair: presage::libsignal_service::protocol::IdentityKeyPair,
    ) -> Result<(), Self::StateStoreError> {
        self.0.set_pni_identity_key_pair(key_pair).await
    }

    async fn sender_certificate(&self) -> Result<Option<SenderCertificate>, Self::StateStoreError> {
        self.0.sender_certificate().await
    }

    async fn save_sender_certificate(
        &self,
        certificate: &SenderCertificate,
    ) -> Result<(), Self::StateStoreError> {
        self.0.save_sender_certificate(certificate).await
    }

    async fn fetch_master_key(
        &self,
    ) -> Result<Option<presage::libsignal_service::prelude::MasterKey>, Self::StateStoreError> {
        self.0.fetch_master_key().await
    }

    async fn store_master_key(
        &self,
        master_key: Option<&presage::libsignal_service::prelude::MasterKey>,
    ) -> Result<(), Self::StateStoreError> {
        self.0.store_master_key(master_key).await
    }
}

// Delegate ContentsStore trait BUT no-op message methods
impl ContentsStore for StromaStore {
    type ContentsStoreError = SqliteStoreError;

    type ContactsIter =
        Box<dyn Iterator<Item = Result<Contact, Self::ContentsStoreError>> + Send + Sync>;

    type GroupsIter = Box<
        dyn Iterator<Item = Result<(GroupMasterKeyBytes, Group), Self::ContentsStoreError>>
            + Send
            + Sync,
    >;

    type MessagesIter = Box<
        dyn Iterator<
                Item = Result<
                    presage::libsignal_service::content::Content,
                    Self::ContentsStoreError,
                >,
            > + Send
            + Sync,
    >;

    type StickerPacksIter =
        Box<dyn Iterator<Item = Result<StickerPack, Self::ContentsStoreError>> + Send + Sync>;

    // Delegate profile methods
    async fn clear_profiles(&mut self) -> Result<(), Self::ContentsStoreError> {
        self.0.clear_profiles().await
    }

    async fn upsert_profile_key(
        &mut self,
        uuid: &Uuid,
        key: ProfileKey,
    ) -> Result<bool, Self::ContentsStoreError> {
        self.0.upsert_profile_key(uuid, key).await
    }

    async fn profile_key(
        &self,
        service_id: &presage::libsignal_service::protocol::ServiceId,
    ) -> Result<Option<ProfileKey>, Self::ContentsStoreError> {
        self.0.profile_key(service_id).await
    }

    async fn save_profile(
        &mut self,
        uuid: Uuid,
        key: ProfileKey,
        profile: presage::libsignal_service::Profile,
    ) -> Result<(), Self::ContentsStoreError> {
        self.0.save_profile(uuid, key, profile).await
    }

    async fn profile(
        &self,
        uuid: Uuid,
        key: ProfileKey,
    ) -> Result<Option<presage::libsignal_service::Profile>, Self::ContentsStoreError> {
        self.0.profile(uuid, key).await
    }

    async fn save_profile_avatar(
        &mut self,
        uuid: Uuid,
        key: ProfileKey,
        avatar: &AvatarBytes,
    ) -> Result<(), Self::ContentsStoreError> {
        self.0.save_profile_avatar(uuid, key, avatar).await
    }

    async fn profile_avatar(
        &self,
        uuid: Uuid,
        key: ProfileKey,
    ) -> Result<Option<AvatarBytes>, Self::ContentsStoreError> {
        self.0.profile_avatar(uuid, key).await
    }

    // Delegate contact methods
    async fn clear_contacts(&mut self) -> Result<(), Self::ContentsStoreError> {
        self.0.clear_contacts().await
    }

    async fn save_contact(&mut self, contact: &Contact) -> Result<(), Self::ContentsStoreError> {
        self.0.save_contact(contact).await
    }

    async fn contacts(&self) -> Result<Self::ContactsIter, Self::ContentsStoreError> {
        self.0.contacts().await
    }

    async fn contact_by_id(&self, id: &Uuid) -> Result<Option<Contact>, Self::ContentsStoreError> {
        self.0.contact_by_id(id).await
    }

    // Delegate group methods
    async fn clear_groups(&mut self) -> Result<(), Self::ContentsStoreError> {
        self.0.clear_groups().await
    }

    async fn save_group(
        &self,
        master_key_bytes: GroupMasterKeyBytes,
        group: impl Into<Group>,
    ) -> Result<(), Self::ContentsStoreError> {
        self.0.save_group(master_key_bytes, group).await
    }

    async fn groups(&self) -> Result<Self::GroupsIter, Self::ContentsStoreError> {
        self.0.groups().await
    }

    async fn group(
        &self,
        master_key_bytes: GroupMasterKeyBytes,
    ) -> Result<Option<Group>, Self::ContentsStoreError> {
        self.0.group(master_key_bytes).await
    }

    async fn save_group_avatar(
        &self,
        master_key_bytes: GroupMasterKeyBytes,
        avatar: &AvatarBytes,
    ) -> Result<(), Self::ContentsStoreError> {
        self.0.save_group_avatar(master_key_bytes, avatar).await
    }

    async fn group_avatar(
        &self,
        master_key_bytes: GroupMasterKeyBytes,
    ) -> Result<Option<AvatarBytes>, Self::ContentsStoreError> {
        self.0.group_avatar(master_key_bytes).await
    }

    // Delegate general contents methods
    async fn clear_contents(&mut self) -> Result<(), Self::ContentsStoreError> {
        self.0.clear_contents().await
    }

    // NO-OP message methods (server seizure protection)
    async fn clear_messages(&mut self) -> Result<(), Self::ContentsStoreError> {
        // NO-OP: No messages stored
        Ok(())
    }

    async fn clear_thread(&mut self, _thread: &Thread) -> Result<(), Self::ContentsStoreError> {
        // NO-OP: No messages stored
        Ok(())
    }

    async fn save_message(
        &self,
        _thread: &Thread,
        _message: presage::libsignal_service::content::Content,
    ) -> Result<(), Self::ContentsStoreError> {
        // NO-OP: Do not persist message history (server seizure protection)
        Ok(())
    }

    async fn delete_message(
        &mut self,
        _thread: &Thread,
        _timestamp: u64,
    ) -> Result<bool, Self::ContentsStoreError> {
        // NO-OP: No messages stored, so nothing to delete
        Ok(false)
    }

    async fn message(
        &self,
        _thread: &Thread,
        _timestamp: u64,
    ) -> Result<Option<presage::libsignal_service::content::Content>, Self::ContentsStoreError>
    {
        // NO-OP: No messages stored
        Ok(None)
    }

    async fn messages(
        &self,
        _thread: &Thread,
        _range: impl RangeBounds<u64>,
    ) -> Result<Self::MessagesIter, Self::ContentsStoreError> {
        // NO-OP: No messages stored, return empty iterator
        Ok(Box::new(std::iter::empty()))
    }

    // Delegate sticker pack methods
    async fn add_sticker_pack(
        &mut self,
        pack: &StickerPack,
    ) -> Result<(), Self::ContentsStoreError> {
        self.0.add_sticker_pack(pack).await
    }

    async fn remove_sticker_pack(
        &mut self,
        pack_id: &[u8],
    ) -> Result<bool, Self::ContentsStoreError> {
        self.0.remove_sticker_pack(pack_id).await
    }

    async fn sticker_pack(
        &self,
        pack_id: &[u8],
    ) -> Result<Option<StickerPack>, Self::ContentsStoreError> {
        self.0.sticker_pack(pack_id).await
    }

    async fn sticker_packs(&self) -> Result<Self::StickerPacksIter, Self::ContentsStoreError> {
        self.0.sticker_packs().await
    }
}

// Delegate Store trait (high-level store operations)
// Note: We delegate to the inner SqliteStore's Store implementation
impl Store for StromaStore {
    type Error = SqliteStoreError;

    // The protocol stores are private in presage-store-sqlite, so we use the inner store's types
    type AciStore = <SqliteStore as Store>::AciStore;
    type PniStore = <SqliteStore as Store>::PniStore;

    async fn clear(&mut self) -> Result<(), Self::Error> {
        self.0.clear().await
    }

    fn aci_protocol_store(&self) -> Self::AciStore {
        self.0.aci_protocol_store()
    }

    fn pni_protocol_store(&self) -> Self::PniStore {
        self.0.pni_protocol_store()
    }
}

/// Store errors
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("SQLite error: {0}")]
    Sqlite(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_stroma_store_open() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let result = StromaStore::open(&db_path, "test_passphrase".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_message_storage_noops() {
        use presage::libsignal_service::content::{Content, ContentBody, Metadata};
        use presage::libsignal_service::prelude::Uuid;
        use presage::libsignal_service::protocol::ServiceId;
        use presage::store::Thread;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let store = StromaStore::open(&db_path, "test_passphrase".to_string())
            .await
            .unwrap();

        // Create dummy thread and message
        let uuid = Uuid::nil();
        let thread = Thread::Contact(uuid);

        // Create a minimal Content for testing
        use presage::libsignal_service::push_service::DEFAULT_DEVICE_ID;
        use presage::proto::NullMessage;

        let service_id =
            ServiceId::parse_from_service_id_string("00000000-0000-0000-0000-000000000000")
                .unwrap();
        let metadata = Metadata {
            sender: service_id,
            destination: service_id,
            sender_device: *DEFAULT_DEVICE_ID,
            timestamp: 123456,
            needs_receipt: false,
            unidentified_sender: false,
            server_guid: None,
            was_plaintext: false,
        };
        let content = Content {
            metadata,
            body: ContentBody::NullMessage(NullMessage::default()),
        };

        // save_message should succeed but not store
        let result = store.save_message(&thread, content.clone()).await;
        assert!(result.is_ok());

        // message should return None (since we no-op message storage)
        let result = store.message(&thread, 12345).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
