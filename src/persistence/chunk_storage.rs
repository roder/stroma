//! Contract-Based Chunk Storage
//!
//! This module provides an abstraction layer for storing and retrieving chunks
//! via Freenet contracts or other P2P storage mechanisms.
//!
//! ## Storage Model
//!
//! - **Local Storage**: Chunks owned by this bot (on-disk cache)
//! - **Remote Storage**: Chunks held for other bots (Freenet contracts)
//! - **Contract-Based**: Each chunk maps to a deterministic contract address
//!
//! ## Contract Address Derivation
//!
//! ```text
//! contract_addr = SHA256(owner_hash || chunk_index || holder_hash || epoch)
//! ```
//!
//! This ensures:
//! - Deterministic addressing (all parties compute same address)
//! - Collision resistance (different chunks → different contracts)
//! - Epoch-based invalidation (holder reassignment triggers new contracts)
//!
//! ## Security Properties
//!
//! - **Encrypted**: All chunks are encrypted before storage
//! - **Signed**: HMAC signatures prevent tampering
//! - **Untrusted Holders**: Holders cannot read or modify chunks
//! - **Verification**: Retrievers validate signatures on fetch
//!
//! ## References
//!
//! - Design: docs/PERSISTENCE.md § Chunk Storage
//! - Agent: Agent-Freenet

use super::chunks::Chunk;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during chunk storage operations
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Chunk not found: owner={owner}, index={chunk_index}")]
    ChunkNotFound { owner: String, chunk_index: u32 },

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Storage full: {0}")]
    StorageFull(String),

    #[error("Contract operation failed: {0}")]
    ContractError(String),

    #[error("Invalid chunk data: {0}")]
    InvalidChunk(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Chunk storage trait for abstracting storage backends.
///
/// This trait allows different implementations:
/// - **Freenet**: Production P2P storage via Freenet contracts
/// - **Mock**: In-memory storage for testing
/// - **Hybrid**: Local cache + remote fallback
#[async_trait]
pub trait ChunkStorage: Send + Sync {
    /// Store a chunk locally (owner's own chunks).
    ///
    /// Local chunks are stored on-disk for fast access during recovery.
    ///
    /// # Arguments
    ///
    /// * `owner` - Owner's contract hash
    /// * `chunk_index` - Index of this chunk (0-based)
    /// * `chunk` - The chunk to store
    ///
    /// # Returns
    ///
    /// `Ok(())` if successful
    async fn store_local(
        &self,
        owner: &str,
        chunk_index: u32,
        chunk: &Chunk,
    ) -> Result<(), StorageError>;

    /// Retrieve a locally stored chunk.
    ///
    /// # Arguments
    ///
    /// * `owner` - Owner's contract hash
    /// * `chunk_index` - Index of the chunk to retrieve
    ///
    /// # Returns
    ///
    /// The chunk if found
    async fn retrieve_local(&self, owner: &str, chunk_index: u32) -> Result<Chunk, StorageError>;

    /// Store a chunk remotely (holding for another bot).
    ///
    /// Remote chunks are stored via Freenet contracts at deterministic addresses.
    /// Returns a cryptographic attestation proving the holder possesses the chunk.
    ///
    /// # Arguments
    ///
    /// * `holder` - This bot's contract hash (holder identity)
    /// * `owner` - Owner's contract hash (whose chunk this is)
    /// * `chunk_index` - Index of this chunk
    /// * `chunk` - The chunk to store
    ///
    /// # Returns
    ///
    /// `Ok(Attestation)` with signed proof of chunk possession if successful
    async fn store_remote(
        &self,
        holder: &str,
        owner: &str,
        chunk_index: u32,
        chunk: &Chunk,
    ) -> Result<super::attestation::Attestation, StorageError>;

    /// Retrieve a chunk from a remote holder.
    ///
    /// # Arguments
    ///
    /// * `holder` - Holder's contract hash
    /// * `owner` - Owner's contract hash
    /// * `chunk_index` - Index of the chunk to retrieve
    ///
    /// # Returns
    ///
    /// The chunk if available
    async fn retrieve_remote(
        &self,
        holder: &str,
        owner: &str,
        chunk_index: u32,
    ) -> Result<Chunk, StorageError>;

    /// Delete a locally stored chunk.
    ///
    /// Called when state is updated and old chunks are no longer needed.
    ///
    /// # Arguments
    ///
    /// * `owner` - Owner's contract hash
    /// * `chunk_index` - Index of the chunk to delete
    ///
    /// # Returns
    ///
    /// `Ok(())` if successful or chunk doesn't exist
    async fn delete_local(&self, owner: &str, chunk_index: u32) -> Result<(), StorageError> {
        // Default implementation: no-op
        // Subclasses can override for actual deletion
        let _ = (owner, chunk_index);
        Ok(())
    }

    /// Delete a remotely held chunk.
    ///
    /// Called when holder is reassigned or chunk is no longer needed.
    ///
    /// # Arguments
    ///
    /// * `holder` - Holder's contract hash
    /// * `owner` - Owner's contract hash
    /// * `chunk_index` - Index of the chunk to delete
    ///
    /// # Returns
    ///
    /// `Ok(())` if successful or chunk doesn't exist
    async fn delete_remote(
        &self,
        holder: &str,
        owner: &str,
        chunk_index: u32,
    ) -> Result<(), StorageError> {
        // Default implementation: no-op
        let _ = (holder, owner, chunk_index);
        Ok(())
    }

    /// List all locally stored chunks for an owner.
    ///
    /// Used for inventory and cleanup operations.
    ///
    /// # Arguments
    ///
    /// * `owner` - Owner's contract hash
    ///
    /// # Returns
    ///
    /// Vector of chunk indices
    async fn list_local(&self, owner: &str) -> Result<Vec<u32>, StorageError> {
        // Default implementation: empty list
        let _ = owner;
        Ok(Vec::new())
    }

    /// Get storage statistics.
    ///
    /// # Returns
    ///
    /// Storage statistics (optional)
    async fn stats(&self) -> Option<StorageStats> {
        None
    }
}

/// Storage statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Number of locally stored chunks
    pub local_chunks: usize,

    /// Number of remotely held chunks
    pub remote_chunks: usize,

    /// Total storage used in bytes
    pub bytes_used: u64,

    /// Available storage in bytes
    pub bytes_available: u64,
}

/// Contract address derivation for chunk storage.
///
/// Computes deterministic Freenet contract addresses for chunks.
///
/// # Algorithm
///
/// ```text
/// contract_addr = SHA256(owner || chunk_index || holder || epoch)
/// ```
///
/// # Arguments
///
/// * `owner` - Owner's contract hash
/// * `chunk_index` - Chunk index
/// * `holder` - Holder's contract hash
/// * `epoch` - Current epoch
///
/// # Returns
///
/// Deterministic contract address string
pub fn derive_chunk_contract_address(
    owner: &str,
    chunk_index: u32,
    holder: &str,
    epoch: u64,
) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(owner.as_bytes());
    hasher.update(chunk_index.to_le_bytes());
    hasher.update(holder.as_bytes());
    hasher.update(epoch.to_le_bytes());

    format!("chunk-contract-{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_address_deterministic() {
        let addr1 = derive_chunk_contract_address("owner-1", 0, "holder-a", 1);
        let addr2 = derive_chunk_contract_address("owner-1", 0, "holder-a", 1);

        assert_eq!(addr1, addr2, "Contract addresses should be deterministic");
    }

    #[test]
    fn test_contract_address_unique_per_chunk() {
        let addr_chunk0 = derive_chunk_contract_address("owner-1", 0, "holder-a", 1);
        let addr_chunk1 = derive_chunk_contract_address("owner-1", 1, "holder-a", 1);

        assert_ne!(
            addr_chunk0, addr_chunk1,
            "Different chunks should have different addresses"
        );
    }

    #[test]
    fn test_contract_address_unique_per_holder() {
        let addr_holder_a = derive_chunk_contract_address("owner-1", 0, "holder-a", 1);
        let addr_holder_b = derive_chunk_contract_address("owner-1", 0, "holder-b", 1);

        assert_ne!(
            addr_holder_a, addr_holder_b,
            "Different holders should have different addresses"
        );
    }

    #[test]
    fn test_contract_address_unique_per_epoch() {
        let addr_epoch1 = derive_chunk_contract_address("owner-1", 0, "holder-a", 1);
        let addr_epoch2 = derive_chunk_contract_address("owner-1", 0, "holder-a", 2);

        assert_ne!(
            addr_epoch1, addr_epoch2,
            "Different epochs should have different addresses"
        );
    }

    #[test]
    fn test_contract_address_unique_per_owner() {
        let addr_owner1 = derive_chunk_contract_address("owner-1", 0, "holder-a", 1);
        let addr_owner2 = derive_chunk_contract_address("owner-2", 0, "holder-a", 1);

        assert_ne!(
            addr_owner1, addr_owner2,
            "Different owners should have different addresses"
        );
    }
}
