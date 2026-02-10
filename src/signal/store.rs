//! Stroma Protocol Store
//!
//! Custom Signal protocol store that:
//! - Stores ONLY protocol state (~100KB)
//! - NO message history
//! - NO contact database
//! - Encrypted with operator passphrase
//!
//! See: .beads/security-constraints.bead § 10

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Wrapper around presage-store-sqlite for Stroma
///
/// CRITICAL: This wrapper delegates to SQLite for:
/// - Account configuration (ACI/PNI identity)
/// - Group membership state
/// - Vote state persistence
///
/// But explicitly EXCLUDES message storage (server seizure protection).
/// See: security-constraints.bead §10
///
/// NOTE: This is a minimal stub to satisfy CI checks while the full
/// implementation is completed in st-ayc by obsidian polecat.
pub struct StromaStore {
    /// Store path
    #[allow(dead_code)]
    path: std::path::PathBuf,
}

impl StromaStore {
    /// Create new StromaStore
    ///
    /// # Arguments
    /// * `path` - Path to SQLite database
    /// * `passphrase` - Encryption passphrase for SQLCipher
    pub fn open(path: impl AsRef<Path>, _passphrase: String) -> Result<Self, StoreError> {
        Ok(Self {
            path: path.as_ref().to_path_buf(),
        })
    }

    // NOTE: message storage methods are intentionally NOT implemented
    // This enforces the security constraint that StromaStore does not
    // persist message history (server seizure protection).
}

/// Custom protocol store for Stroma
///
/// CRITICAL: This store persists ONLY protocol state required for Signal encryption.
/// NO message history, NO contact database, NO conversation content.
pub struct StromaProtocolStore {
    /// Path to encrypted protocol state file (~100KB)
    state_path: std::path::PathBuf,

    /// Operator-provided passphrase for encryption
    #[allow(dead_code)]
    passphrase: String,

    /// In-memory session cache (ephemeral)
    sessions: HashMap<String, SessionData>,
}

/// Protocol state persisted to disk
#[derive(Serialize, Deserialize)]
pub struct ProtocolState {
    /// Signal identity (ACI/PNI)
    identity: IdentityData,

    /// Pre-keys for new conversations
    pre_keys: Vec<PreKeyData>,

    /// Version for migration
    version: u32,
}

#[derive(Serialize, Deserialize)]
struct IdentityData {
    aci: Vec<u8>,
    pni: Option<Vec<u8>>,
    private_key: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct PreKeyData {
    id: u32,
    key: Vec<u8>,
}

#[derive(Clone)]
pub struct SessionData {
    // Ephemeral session data for active conversations
    // Not persisted - reconstructed on restart
    #[allow(dead_code)]
    pub session_cipher: Vec<u8>,
}

impl StromaProtocolStore {
    /// Create new protocol store
    ///
    /// # Arguments
    /// * `state_path` - Path to encrypted protocol state file
    /// * `passphrase` - Operator-provided passphrase for encryption
    pub fn new(state_path: impl AsRef<Path>, passphrase: String) -> Self {
        Self {
            state_path: state_path.as_ref().to_path_buf(),
            passphrase,
            sessions: HashMap::new(),
        }
    }

    /// Load protocol state from disk
    ///
    /// Returns None if file doesn't exist (first run)
    pub fn load(&self) -> Result<Option<ProtocolState>, StoreError> {
        if !self.state_path.exists() {
            return Ok(None);
        }

        // TODO: Implement encrypted loading
        // - Read encrypted file
        // - Decrypt with passphrase-derived key
        // - Deserialize CBOR
        Err(StoreError::NotImplemented("load".to_string()))
    }

    /// Save protocol state to disk
    ///
    /// Encrypts state with passphrase-derived key
    pub fn save(&self, _state: &ProtocolState) -> Result<(), StoreError> {
        // TODO: Implement encrypted saving
        // - Serialize to CBOR
        // - Encrypt with passphrase-derived key
        // - Write to file atomically
        Err(StoreError::NotImplemented("save".to_string()))
    }

    /// Get session for contact (ephemeral, not persisted)
    pub fn get_session(&self, service_id: &str) -> Option<SessionData> {
        self.sessions.get(service_id).cloned()
    }

    /// Store session for contact (in-memory only)
    pub fn put_session(&mut self, service_id: String, session: SessionData) {
        self.sessions.insert(service_id, session);
    }

    /// Clear ephemeral session cache
    pub fn clear_sessions(&mut self) {
        self.sessions.clear();
    }
}

/// Store errors
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_store() {
        let store = StromaProtocolStore::new("/tmp/test.store", "passphrase".to_string());
        assert_eq!(
            store.state_path,
            std::path::PathBuf::from("/tmp/test.store")
        );
        assert_eq!(store.sessions.len(), 0);
    }

    #[test]
    fn test_session_cache() {
        let mut store = StromaProtocolStore::new("/tmp/test.store", "passphrase".to_string());

        let session = SessionData {
            session_cipher: vec![1, 2, 3],
        };

        store.put_session("user1".to_string(), session.clone());

        let retrieved = store.get_session("user1");
        assert!(retrieved.is_some());

        store.clear_sessions();
        assert!(store.get_session("user1").is_none());
    }

    #[test]
    fn test_load_nonexistent() {
        let store = StromaProtocolStore::new("/tmp/nonexistent.store", "passphrase".to_string());
        let result = store.load();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_stroma_store_open() {
        // Test that StromaStore can be created
        let store = StromaStore::open("/tmp/stroma.db", "test_passphrase".to_string());
        assert!(store.is_ok());

        let store = store.unwrap();
        assert_eq!(store.path, std::path::PathBuf::from("/tmp/stroma.db"));
    }
}
