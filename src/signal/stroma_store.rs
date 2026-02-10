//! Stroma Protocol Store
//!
//! Wrapper around presage SqliteStore that:
//! - Delegates protocol state, groups, profiles (encrypted with passphrase)
//! - NO-OP for message storage (seizure protection)
//! - Uses SQLCipher encryption via BIP-39 passphrase
//!
//! See: .beads/signal-integration.bead, .beads/security-constraints.bead § 10

use std::future::Future;
use std::ops::RangeBounds;

use presage::libsignal_service::{
    prelude::{Content, ProfileKey, Uuid},
    protocol::{IdentityKeyPair, SenderCertificate, ServiceId},
    zkgroup::GroupMasterKeyBytes,
    Profile,
};
use presage::manager::RegistrationData;
use presage::model::{contacts::Contact, groups::Group};
use presage::store::{ContentsStore, StateStore, StickerPack, Store, Thread};
use presage::AvatarBytes;

// Re-export SqliteStore types
pub use presage_store_sqlite::{
    OnNewIdentity, SqliteConnectOptions, SqliteStore, SqliteStoreError,
};

/// Stroma wrapper around SqliteStore
///
/// Delegates all persistence EXCEPT message storage (no-op for seizure protection).
/// Uses SQLCipher encryption with 24-word BIP-39 passphrase.
#[derive(Debug, Clone)]
pub struct StromaStore {
    inner: SqliteStore,
}

impl StromaStore {
    /// Open encrypted store with passphrase
    ///
    /// Creates new encrypted SQLite database if it doesn't exist.
    /// Uses SQLCipher with PBKDF2-HMAC-SHA512 (256K iterations).
    ///
    /// # Arguments
    /// * `path` - Path to SQLite database file
    /// * `passphrase` - BIP-39 24-word recovery phrase (or raw passphrase)
    /// * `trust_new_identities` - How to handle new identity keys
    pub async fn open(
        path: &str,
        passphrase: Option<&str>,
        trust_new_identities: OnNewIdentity,
    ) -> Result<Self, SqliteStoreError> {
        let inner =
            SqliteStore::open_with_passphrase(path, passphrase, trust_new_identities).await?;
        Ok(Self { inner })
    }

    /// Access inner SqliteStore (for advanced use cases)
    pub fn inner(&self) -> &SqliteStore {
        &self.inner
    }
}

// StateStore: Delegate all methods to inner SqliteStore
impl StateStore for StromaStore {
    type StateStoreError = SqliteStoreError;

    fn load_registration_data(
        &self,
    ) -> impl Future<Output = Result<Option<RegistrationData>, Self::StateStoreError>> {
        self.inner.load_registration_data()
    }

    fn set_aci_identity_key_pair(
        &self,
        key_pair: IdentityKeyPair,
    ) -> impl Future<Output = Result<(), Self::StateStoreError>> {
        self.inner.set_aci_identity_key_pair(key_pair)
    }

    fn set_pni_identity_key_pair(
        &self,
        key_pair: IdentityKeyPair,
    ) -> impl Future<Output = Result<(), Self::StateStoreError>> {
        self.inner.set_pni_identity_key_pair(key_pair)
    }

    fn save_registration_data(
        &mut self,
        state: &RegistrationData,
    ) -> impl Future<Output = Result<(), Self::StateStoreError>> + Send {
        self.inner.save_registration_data(state)
    }

    fn sender_certificate(
        &self,
    ) -> impl Future<Output = Result<Option<SenderCertificate>, Self::StateStoreError>> {
        self.inner.sender_certificate()
    }

    fn save_sender_certificate(
        &self,
        certificate: &SenderCertificate,
    ) -> impl Future<Output = Result<(), Self::StateStoreError>> {
        self.inner.save_sender_certificate(certificate)
    }

    fn is_registered(&self) -> impl Future<Output = bool> {
        self.inner.is_registered()
    }

    fn clear_registration(&mut self) -> impl Future<Output = Result<(), Self::StateStoreError>> {
        self.inner.clear_registration()
    }
}

// ContentsStore: Delegate groups/contacts/profiles, NO-OP for messages/stickers
impl ContentsStore for StromaStore {
    type ContentsStoreError = SqliteStoreError;
    type ContactsIter = <SqliteStore as ContentsStore>::ContactsIter;
    type GroupsIter = <SqliteStore as ContentsStore>::GroupsIter;
    type MessagesIter = Box<dyn Iterator<Item = Result<Content, SqliteStoreError>> + Send + Sync>;
    type StickerPacksIter =
        Box<dyn Iterator<Item = Result<StickerPack, SqliteStoreError>> + Send + Sync>;

    fn clear_profiles(&mut self) -> impl Future<Output = Result<(), Self::ContentsStoreError>> {
        self.inner.clear_profiles()
    }

    fn clear_contents(&mut self) -> impl Future<Output = Result<(), Self::ContentsStoreError>> {
        self.inner.clear_contents()
    }

    // Messages: NO-OP (seizure protection)
    async fn clear_messages(&mut self) -> Result<(), Self::ContentsStoreError> {
        Ok(())
    }

    async fn clear_thread(&mut self, _thread: &Thread) -> Result<(), Self::ContentsStoreError> {
        Ok(())
    }

    async fn save_message(
        &self,
        _thread: &Thread,
        _message: Content,
    ) -> Result<(), Self::ContentsStoreError> {
        Ok(())
    }

    async fn delete_message(
        &mut self,
        _thread: &Thread,
        _timestamp: u64,
    ) -> Result<bool, Self::ContentsStoreError> {
        Ok(false)
    }

    async fn message(
        &self,
        _thread: &Thread,
        _timestamp: u64,
    ) -> Result<Option<Content>, Self::ContentsStoreError> {
        Ok(None)
    }

    async fn messages(
        &self,
        _thread: &Thread,
        _range: impl RangeBounds<u64>,
    ) -> Result<Self::MessagesIter, Self::ContentsStoreError> {
        Ok(
            Box::new(std::iter::empty::<Result<Content, SqliteStoreError>>())
                as Box<dyn Iterator<Item = Result<Content, SqliteStoreError>> + Send + Sync>,
        )
    }

    // Contacts: Delegate to inner store
    fn clear_contacts(&mut self) -> impl Future<Output = Result<(), Self::ContentsStoreError>> {
        self.inner.clear_contacts()
    }

    fn save_contact(
        &mut self,
        contact: &Contact,
    ) -> impl Future<Output = Result<(), Self::ContentsStoreError>> + Send {
        self.inner.save_contact(contact)
    }

    fn contacts(
        &self,
    ) -> impl Future<Output = Result<Self::ContactsIter, Self::ContentsStoreError>> {
        self.inner.contacts()
    }

    fn contact_by_id(
        &self,
        id: &Uuid,
    ) -> impl Future<Output = Result<Option<Contact>, Self::ContentsStoreError>> + Send {
        self.inner.contact_by_id(id)
    }

    // Groups: Delegate to inner store
    fn clear_groups(&mut self) -> impl Future<Output = Result<(), Self::ContentsStoreError>> {
        self.inner.clear_groups()
    }

    fn save_group(
        &self,
        master_key: GroupMasterKeyBytes,
        group: impl Into<Group>,
    ) -> impl Future<Output = Result<(), Self::ContentsStoreError>> {
        self.inner.save_group(master_key, group)
    }

    fn groups(&self) -> impl Future<Output = Result<Self::GroupsIter, Self::ContentsStoreError>> {
        self.inner.groups()
    }

    fn group(
        &self,
        master_key: GroupMasterKeyBytes,
    ) -> impl Future<Output = Result<Option<Group>, Self::ContentsStoreError>> {
        self.inner.group(master_key)
    }

    fn save_group_avatar(
        &self,
        master_key: GroupMasterKeyBytes,
        avatar: &AvatarBytes,
    ) -> impl Future<Output = Result<(), Self::ContentsStoreError>> {
        self.inner.save_group_avatar(master_key, avatar)
    }

    fn group_avatar(
        &self,
        master_key: GroupMasterKeyBytes,
    ) -> impl Future<Output = Result<Option<AvatarBytes>, Self::ContentsStoreError>> {
        self.inner.group_avatar(master_key)
    }

    // Profiles: Delegate to inner store
    fn upsert_profile_key(
        &mut self,
        uuid: &Uuid,
        key: ProfileKey,
    ) -> impl Future<Output = Result<bool, Self::ContentsStoreError>> + Send {
        self.inner.upsert_profile_key(uuid, key)
    }

    fn profile_key(
        &self,
        service_id: &ServiceId,
    ) -> impl Future<Output = Result<Option<ProfileKey>, Self::ContentsStoreError>> + Send {
        self.inner.profile_key(service_id)
    }

    fn save_profile(
        &mut self,
        uuid: Uuid,
        key: ProfileKey,
        profile: Profile,
    ) -> impl Future<Output = Result<(), Self::ContentsStoreError>> {
        self.inner.save_profile(uuid, key, profile)
    }

    fn profile(
        &self,
        uuid: Uuid,
        key: ProfileKey,
    ) -> impl Future<Output = Result<Option<Profile>, Self::ContentsStoreError>> {
        self.inner.profile(uuid, key)
    }

    fn save_profile_avatar(
        &mut self,
        uuid: Uuid,
        key: ProfileKey,
        avatar: &AvatarBytes,
    ) -> impl Future<Output = Result<(), Self::ContentsStoreError>> {
        self.inner.save_profile_avatar(uuid, key, avatar)
    }

    fn profile_avatar(
        &self,
        uuid: Uuid,
        key: ProfileKey,
    ) -> impl Future<Output = Result<Option<AvatarBytes>, Self::ContentsStoreError>> {
        self.inner.profile_avatar(uuid, key)
    }

    // Stickers: NO-OP (not used by Stroma)
    async fn add_sticker_pack(
        &mut self,
        _pack: &StickerPack,
    ) -> Result<(), Self::ContentsStoreError> {
        Ok(())
    }

    async fn sticker_pack(
        &self,
        _id: &[u8],
    ) -> Result<Option<StickerPack>, Self::ContentsStoreError> {
        Ok(None)
    }

    async fn remove_sticker_pack(&mut self, _id: &[u8]) -> Result<bool, Self::ContentsStoreError> {
        Ok(false)
    }

    async fn sticker_packs(&self) -> Result<Self::StickerPacksIter, Self::ContentsStoreError> {
        Ok(
            Box::new(std::iter::empty::<Result<StickerPack, SqliteStoreError>>())
                as Box<dyn Iterator<Item = Result<StickerPack, SqliteStoreError>> + Send + Sync>,
        )
    }
}

// Store: Combine StateStore + ContentsStore + protocol stores
impl Store for StromaStore {
    type Error = SqliteStoreError;
    type AciStore = <SqliteStore as Store>::AciStore;
    type PniStore = <SqliteStore as Store>::PniStore;

    fn clear(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.inner.clear()
    }

    fn aci_protocol_store(&self) -> Self::AciStore {
        self.inner.aci_protocol_store()
    }

    fn pni_protocol_store(&self) -> Self::PniStore {
        self.inner.pni_protocol_store()
    }
}

// Note: SqliteStoreError already implements StoreError in presage-store-sqlite

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stroma_store_creation() {
        // Create in-memory store for testing
        let store = StromaStore::open(":memory:", None, OnNewIdentity::Trust).await;
        assert!(store.is_ok());
    }

    #[tokio::test]
    async fn test_message_storage_noop() {
        // Verify that message retrieval is always None (no persistence)
        let store = StromaStore::open(":memory:", None, OnNewIdentity::Trust)
            .await
            .unwrap();

        let thread = Thread::Contact(Uuid::nil());

        // Verify message retrieval returns None
        let retrieved = store.message(&thread, 0).await.unwrap();
        assert!(retrieved.is_none());

        // Verify messages iterator is empty
        let messages = store.messages(&thread, 0..100).await.unwrap();
        assert_eq!(messages.count(), 0);
    }
}
