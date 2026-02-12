//! Stroma Store Wrapper
//!
//! Dual-database architecture:
//! - signal.db: presage SqliteStore (protocol state, groups, contacts, profiles)
//! - stroma.db: Stroma-specific data (poll state, ephemeral vote aggregates)
//!
//! Both databases use the SAME passphrase (24-word BIP-39 recovery phrase).
//! Message storage is NO-OPed for server seizure protection.
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
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::ops::RangeBounds;
use std::path::Path;
use std::str::FromStr;

/// Stroma protocol store wrapper
///
/// CRITICAL: This wrapper uses two separate databases:
/// 1. signal.db - presage SqliteStore (protocol state, groups, contacts)
/// 2. stroma.db - Stroma-specific data (poll state, ephemeral aggregates)
///
/// Both databases are encrypted with SQLCipher using the SAME passphrase.
/// Message storage is NO-OPed for server seizure protection.
///
/// See: security-constraints.bead §10
#[derive(Clone)]
pub struct StromaStore {
    signal_store: SqliteStore,
    stroma_db: Pool<Sqlite>,
}

impl StromaStore {
    /// Open both signal.db (presage) and stroma.db with the same passphrase.
    ///
    /// Both databases are created at the same time during initial setup and
    /// encrypted with SQLCipher using the SAME passphrase.
    ///
    /// # Arguments
    /// * `path` - Directory containing both databases (not a file path)
    /// * `passphrase` - Encryption passphrase for both databases (SQLCipher)
    ///
    /// # Returns
    /// * `Ok(StromaStore)` - Store with both databases opened
    /// * `Err(StoreError)` - If either database fails to open
    pub async fn open(path: impl AsRef<Path>, passphrase: String) -> Result<Self, StoreError> {
        let base_path = path.as_ref();

        // Ensure base path is a directory
        if base_path.exists() && !base_path.is_dir() {
            return Err(StoreError::Sqlite(
                "Path must be a directory, not a file".to_string(),
            ));
        }

        // Create directory if it doesn't exist
        if !base_path.exists() {
            std::fs::create_dir_all(base_path)
                .map_err(|e| StoreError::Sqlite(format!("Failed to create directory: {}", e)))?;
        }

        // Open presage's SqliteStore (signal.db)
        let signal_db_path = base_path.join("signal.db");
        let signal_db_str = signal_db_path.to_str().ok_or_else(|| {
            StoreError::Sqlite("Invalid path: contains non-UTF8 characters".to_string())
        })?;

        let signal_store = SqliteStore::open_with_passphrase(
            signal_db_str,
            Some(&passphrase),
            OnNewIdentity::Trust, // Trust new identities by default
        )
        .await
        .map_err(|e| StoreError::Sqlite(format!("Failed to open signal.db: {:?}", e)))?;

        // Open stroma.db with SQLCipher using SAME passphrase
        let stroma_db_path = base_path.join("stroma.db");
        let stroma_db_str = stroma_db_path
            .to_str()
            .ok_or_else(|| StoreError::Sqlite("Invalid stroma.db path".to_string()))?;

        // Create SQLCipher connection options
        // Note: SQLCipher requires the key to be quoted if it contains spaces
        let connect_options =
            SqliteConnectOptions::from_str(&format!("sqlite://{}", stroma_db_str))
                .map_err(|e| StoreError::Sqlite(format!("Invalid connection string: {}", e)))?
                .create_if_missing(true)
                .pragma("key", format!("'{}'", passphrase.replace("'", "''"))); // Same passphrase as signal.db, SQL-escaped

        // Create connection pool
        let stroma_db = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connect_options)
            .await
            .map_err(|e| StoreError::Sqlite(format!("Failed to open stroma.db: {}", e)))?;

        // Run migrations
        sqlx::migrate!("./src/signal/stroma_store_migrations")
            .run(&stroma_db)
            .await
            .map_err(|e| StoreError::Sqlite(format!("Migration failed: {}", e)))?;

        Ok(Self {
            signal_store,
            stroma_db,
        })
    }

    /// Store arbitrary key-value data in stroma.db (for poll state persistence).
    ///
    /// Uses stroma_kv table in the separate stroma.db database.
    /// Per security-constraints.bead: Poll state MUST be zeroized when outcome determined.
    pub async fn store_data(&self, key: &str, value: &[u8]) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT OR REPLACE INTO stroma_kv (key, value, updated_at) VALUES (?, ?, strftime('%s', 'now'))"
        )
        .bind(key)
        .bind(value)
        .execute(&self.stroma_db)
        .await
        .map_err(|e| StoreError::Sqlite(format!("Failed to store data: {}", e)))?;
        Ok(())
    }

    /// Retrieve arbitrary key-value data from stroma.db (for poll state restoration).
    ///
    /// Returns None if the key doesn't exist in the stroma_kv table.
    pub async fn retrieve_data(&self, key: &str) -> Result<Option<Vec<u8>>, StoreError> {
        let row: Option<(Vec<u8>,)> = sqlx::query_as("SELECT value FROM stroma_kv WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.stroma_db)
            .await
            .map_err(|e| StoreError::Sqlite(format!("Failed to retrieve data: {}", e)))?;
        Ok(row.map(|r| r.0))
    }

    /// Delete key-value data from stroma.db (for poll state cleanup).
    ///
    /// Per security-constraints.bead: Poll state MUST be zeroized when outcome determined.
    pub async fn delete_data(&self, key: &str) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM stroma_kv WHERE key = ?")
            .bind(key)
            .execute(&self.stroma_db)
            .await
            .map_err(|e| StoreError::Sqlite(format!("Failed to delete data: {}", e)))?;
        Ok(())
    }

    /// Clear all data from both databases (signal.db and stroma.db).
    ///
    /// This is used during unregistration to ensure no local data remains.
    /// For full account deletion, use the Manager's delete_account() method first
    /// to also remove the account from Signal servers.
    ///
    /// CAUTION: This is irreversible. All local data will be lost.
    pub async fn clear_all(&mut self) -> Result<(), StoreError> {
        // Clear signal.db via presage
        self.signal_store
            .clear()
            .await
            .map_err(|e| StoreError::Sqlite(format!("Failed to clear signal store: {:?}", e)))?;

        // Clear stroma.db
        sqlx::query("DELETE FROM stroma_kv")
            .execute(&self.stroma_db)
            .await
            .map_err(|e| StoreError::Sqlite(format!("Failed to clear stroma data: {}", e)))?;

        Ok(())
    }
}

// Delegate StateStore trait (protocol state management)
impl StateStore for StromaStore {
    type StateStoreError = SqliteStoreError;

    async fn load_registration_data(
        &self,
    ) -> Result<Option<RegistrationData>, Self::StateStoreError> {
        self.signal_store.load_registration_data().await
    }

    async fn save_registration_data(
        &mut self,
        state: &RegistrationData,
    ) -> Result<(), Self::StateStoreError> {
        self.signal_store.save_registration_data(state).await
    }

    async fn is_registered(&self) -> bool {
        self.signal_store.is_registered().await
    }

    async fn clear_registration(&mut self) -> Result<(), Self::StateStoreError> {
        self.signal_store.clear_registration().await
    }

    async fn set_aci_identity_key_pair(
        &self,
        key_pair: presage::libsignal_service::protocol::IdentityKeyPair,
    ) -> Result<(), Self::StateStoreError> {
        self.signal_store.set_aci_identity_key_pair(key_pair).await
    }

    async fn set_pni_identity_key_pair(
        &self,
        key_pair: presage::libsignal_service::protocol::IdentityKeyPair,
    ) -> Result<(), Self::StateStoreError> {
        self.signal_store.set_pni_identity_key_pair(key_pair).await
    }

    async fn sender_certificate(&self) -> Result<Option<SenderCertificate>, Self::StateStoreError> {
        self.signal_store.sender_certificate().await
    }

    async fn save_sender_certificate(
        &self,
        certificate: &SenderCertificate,
    ) -> Result<(), Self::StateStoreError> {
        self.signal_store.save_sender_certificate(certificate).await
    }

    async fn fetch_master_key(
        &self,
    ) -> Result<Option<presage::libsignal_service::prelude::MasterKey>, Self::StateStoreError> {
        self.signal_store.fetch_master_key().await
    }

    async fn store_master_key(
        &self,
        master_key: Option<&presage::libsignal_service::prelude::MasterKey>,
    ) -> Result<(), Self::StateStoreError> {
        self.signal_store.store_master_key(master_key).await
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
        self.signal_store.clear_profiles().await
    }

    async fn upsert_profile_key(
        &mut self,
        uuid: &Uuid,
        key: ProfileKey,
    ) -> Result<bool, Self::ContentsStoreError> {
        self.signal_store.upsert_profile_key(uuid, key).await
    }

    async fn profile_key(
        &self,
        service_id: &presage::libsignal_service::protocol::ServiceId,
    ) -> Result<Option<ProfileKey>, Self::ContentsStoreError> {
        self.signal_store.profile_key(service_id).await
    }

    async fn save_profile(
        &mut self,
        uuid: Uuid,
        key: ProfileKey,
        profile: presage::libsignal_service::Profile,
    ) -> Result<(), Self::ContentsStoreError> {
        self.signal_store.save_profile(uuid, key, profile).await
    }

    async fn profile(
        &self,
        uuid: Uuid,
        key: ProfileKey,
    ) -> Result<Option<presage::libsignal_service::Profile>, Self::ContentsStoreError> {
        self.signal_store.profile(uuid, key).await
    }

    async fn save_profile_avatar(
        &mut self,
        uuid: Uuid,
        key: ProfileKey,
        avatar: &AvatarBytes,
    ) -> Result<(), Self::ContentsStoreError> {
        self.signal_store
            .save_profile_avatar(uuid, key, avatar)
            .await
    }

    async fn profile_avatar(
        &self,
        uuid: Uuid,
        key: ProfileKey,
    ) -> Result<Option<AvatarBytes>, Self::ContentsStoreError> {
        self.signal_store.profile_avatar(uuid, key).await
    }

    // Delegate profile credential methods
    async fn save_profile_credential(
        &mut self,
        uuid: Uuid,
        credential_bytes: Vec<u8>,
        expiration_time: u64,
    ) -> Result<(), Self::ContentsStoreError> {
        self.signal_store
            .save_profile_credential(uuid, credential_bytes, expiration_time)
            .await
    }

    async fn profile_credential(
        &self,
        uuid: &Uuid,
    ) -> Result<Option<Vec<u8>>, Self::ContentsStoreError> {
        self.signal_store.profile_credential(uuid).await
    }

    async fn clear_expired_credentials(&mut self) -> Result<u64, Self::ContentsStoreError> {
        self.signal_store.clear_expired_credentials().await
    }

    // Delegate contact methods
    async fn clear_contacts(&mut self) -> Result<(), Self::ContentsStoreError> {
        self.signal_store.clear_contacts().await
    }

    async fn save_contact(&mut self, contact: &Contact) -> Result<(), Self::ContentsStoreError> {
        self.signal_store.save_contact(contact).await
    }

    async fn contacts(&self) -> Result<Self::ContactsIter, Self::ContentsStoreError> {
        self.signal_store.contacts().await
    }

    async fn contact_by_id(&self, id: &Uuid) -> Result<Option<Contact>, Self::ContentsStoreError> {
        self.signal_store.contact_by_id(id).await
    }

    // Delegate group methods
    async fn clear_groups(&mut self) -> Result<(), Self::ContentsStoreError> {
        self.signal_store.clear_groups().await
    }

    async fn save_group(
        &self,
        master_key_bytes: GroupMasterKeyBytes,
        group: impl Into<Group>,
    ) -> Result<(), Self::ContentsStoreError> {
        self.signal_store.save_group(master_key_bytes, group).await
    }

    async fn groups(&self) -> Result<Self::GroupsIter, Self::ContentsStoreError> {
        self.signal_store.groups().await
    }

    async fn group(
        &self,
        master_key_bytes: GroupMasterKeyBytes,
    ) -> Result<Option<Group>, Self::ContentsStoreError> {
        self.signal_store.group(master_key_bytes).await
    }

    async fn save_group_avatar(
        &self,
        master_key_bytes: GroupMasterKeyBytes,
        avatar: &AvatarBytes,
    ) -> Result<(), Self::ContentsStoreError> {
        self.signal_store
            .save_group_avatar(master_key_bytes, avatar)
            .await
    }

    async fn group_avatar(
        &self,
        master_key_bytes: GroupMasterKeyBytes,
    ) -> Result<Option<AvatarBytes>, Self::ContentsStoreError> {
        self.signal_store.group_avatar(master_key_bytes).await
    }

    // Delegate general contents methods
    async fn clear_contents(&mut self) -> Result<(), Self::ContentsStoreError> {
        self.signal_store.clear_contents().await
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
        self.signal_store.add_sticker_pack(pack).await
    }

    async fn remove_sticker_pack(
        &mut self,
        pack_id: &[u8],
    ) -> Result<bool, Self::ContentsStoreError> {
        self.signal_store.remove_sticker_pack(pack_id).await
    }

    async fn sticker_pack(
        &self,
        pack_id: &[u8],
    ) -> Result<Option<StickerPack>, Self::ContentsStoreError> {
        self.signal_store.sticker_pack(pack_id).await
    }

    async fn sticker_packs(&self) -> Result<Self::StickerPacksIter, Self::ContentsStoreError> {
        self.signal_store.sticker_packs().await
    }
}

// Delegate Store trait (high-level store operations)
// Note: We delegate to the signal_store's Store implementation
impl Store for StromaStore {
    type Error = SqliteStoreError;

    // The protocol stores are private in presage-store-sqlite, so we use the inner store's types
    type AciStore = <SqliteStore as Store>::AciStore;
    type PniStore = <SqliteStore as Store>::PniStore;

    async fn clear(&mut self) -> Result<(), Self::Error> {
        self.signal_store.clear().await
    }

    fn aci_protocol_store(&self) -> Self::AciStore {
        self.signal_store.aci_protocol_store()
    }

    fn pni_protocol_store(&self) -> Self::PniStore {
        self.signal_store.pni_protocol_store()
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
        // Pass directory path, not file path
        let result = StromaStore::open(temp_dir.path(), "test_passphrase".to_string()).await;
        assert!(result.is_ok());

        // Verify both databases were created
        assert!(temp_dir.path().join("signal.db").exists());
        assert!(temp_dir.path().join("stroma.db").exists());
    }

    #[tokio::test]
    async fn test_message_storage_noops() {
        use presage::libsignal_service::content::{Content, ContentBody, Metadata};
        use presage::libsignal_service::prelude::Uuid;
        use presage::libsignal_service::protocol::ServiceId;
        use presage::store::Thread;

        let temp_dir = TempDir::new().unwrap();

        let store = StromaStore::open(temp_dir.path(), "test_passphrase".to_string())
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

    #[tokio::test]
    async fn test_kv_storage() {
        let temp_dir = TempDir::new().unwrap();
        let store = StromaStore::open(temp_dir.path(), "test_passphrase".to_string())
            .await
            .unwrap();

        // Test store_data
        let key = "test_poll_state";
        let value = b"test_data";
        let result = store.store_data(key, value).await;
        assert!(result.is_ok());

        // Test retrieve_data
        let retrieved = store.retrieve_data(key).await.unwrap();
        assert_eq!(retrieved, Some(value.to_vec()));

        // Test delete_data
        let result = store.delete_data(key).await;
        assert!(result.is_ok());

        // Verify deletion
        let retrieved = store.retrieve_data(key).await.unwrap();
        assert_eq!(retrieved, None);
    }

    #[tokio::test]
    async fn test_clear_all() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = StromaStore::open(temp_dir.path(), "test_passphrase".to_string())
            .await
            .unwrap();

        // Store some data in stroma.db
        let key1 = "test_key_1";
        let key2 = "test_key_2";
        store.store_data(key1, b"value1").await.unwrap();
        store.store_data(key2, b"value2").await.unwrap();

        // Verify data exists
        assert!(store.retrieve_data(key1).await.unwrap().is_some());
        assert!(store.retrieve_data(key2).await.unwrap().is_some());

        // Clear all data
        let result = store.clear_all().await;
        assert!(result.is_ok());

        // Verify stroma.db is cleared
        assert!(store.retrieve_data(key1).await.unwrap().is_none());
        assert!(store.retrieve_data(key2).await.unwrap().is_none());
    }
}
