//! Recovery orchestration for Stroma's Reciprocal Persistence Network.
//!
//! This module implements the full crash recovery flow:
//! 1. Fetch registry to discover network state
//! 2. Compute chunk holders using rendezvous hashing
//! 3. Fetch chunks from holders (with fallback to alternates)
//! 4. Reassemble and decrypt state
//! 5. Verify signature and return recovered state
//!
//! ## Design
//!
//! - **Fallback**: If primary holder unavailable, try other replicas
//! - **Resilience**: Need any 1 of 3 copies per chunk (1 local + 2 remote)
//! - **Security**: All chunks encrypted, signed, require ACI key
//! - **Verification**: Signature verification prevents tampering
//!
//! ## Recovery Requirements
//!
//! To recover state, you need:
//! 1. Signal protocol store backup (contains ACI key)
//! 2. At least 1 copy of each chunk available
//! 3. Network access to registry and holders
//!
//! ## References
//!
//! - Architecture: docs/PERSISTENCE.md § Recovery
//! - Security: .beads/security-constraints.bead

use super::chunks::{decrypt_and_reassemble, Chunk, ChunkError};
use super::registry::{PersistenceRegistry, RegistryEntry};
use super::rendezvous::compute_chunk_holders;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during recovery
#[derive(Debug, Error)]
pub enum RecoveryError {
    #[error("Registry fetch failed: {0}")]
    RegistryFetchFailed(String),

    #[error("Chunk fetch failed for chunk {chunk_index}: {reason}")]
    ChunkFetchFailed { chunk_index: u32, reason: String },

    #[error("Missing chunks: {0}")]
    MissingChunks(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(#[from] ChunkError),

    #[error("Owner not found in registry")]
    OwnerNotInRegistry,

    #[error("Insufficient replicas available")]
    InsufficientReplicas,

    #[error("Network error: {0}")]
    NetworkError(String),
}

/// Registry fetcher trait for abstracting registry access.
///
/// This allows testing with mock registries and production use with
/// actual Freenet contract queries.
#[async_trait]
pub trait RegistryFetcher: Send + Sync {
    /// Fetch the current registry state.
    ///
    /// # Returns
    ///
    /// Complete registry with all registered bots
    async fn fetch_registry(&self) -> Result<PersistenceRegistry, RecoveryError>;

    /// Get registry entry for a specific bot.
    ///
    /// # Arguments
    ///
    /// * `contract_hash` - Bot's contract hash
    ///
    /// # Returns
    ///
    /// Registry entry if bot is registered
    async fn get_bot_entry(
        &self,
        contract_hash: &str,
    ) -> Result<Option<RegistryEntry>, RecoveryError>;
}

/// Chunk fetcher trait for abstracting chunk retrieval.
///
/// This allows testing with mock storage and production use with
/// actual Freenet contract queries or P2P fetches.
#[async_trait]
pub trait ChunkFetcher: Send + Sync {
    /// Fetch a specific chunk from a holder.
    ///
    /// # Arguments
    ///
    /// * `holder` - Holder's contract hash
    /// * `owner` - Owner's contract hash (whose chunk this is)
    /// * `chunk_index` - Index of the chunk to fetch
    ///
    /// # Returns
    ///
    /// The requested chunk if available
    async fn fetch_chunk(
        &self,
        holder: &str,
        owner: &str,
        chunk_index: u32,
    ) -> Result<Chunk, RecoveryError>;
}

/// Recovery configuration and options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryConfig {
    /// Number of replicas per chunk (default: 2 remote + 1 local = 3 total)
    pub num_replicas: usize,

    /// Maximum retries per chunk fetch (default: 3)
    pub max_retries: usize,

    /// Timeout per chunk fetch in milliseconds (default: 5000ms = 5s)
    pub fetch_timeout_ms: u64,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            num_replicas: 2, // 2 remote replicas (local copy assumed lost)
            max_retries: 3,
            fetch_timeout_ms: 5000,
        }
    }
}

/// Recovered state with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveredState {
    /// Decrypted state bytes
    pub state: Vec<u8>,

    /// Owner's contract hash
    pub owner: String,

    /// Number of chunks recovered
    pub num_chunks: u32,

    /// Recovery statistics
    pub stats: RecoveryStats,
}

/// Recovery statistics for monitoring and debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStats {
    /// Total chunks to recover
    pub total_chunks: u32,

    /// Chunks recovered successfully
    pub chunks_recovered: u32,

    /// Chunks that required fallback to alternate holders
    pub chunks_with_fallback: u32,

    /// Total fetch attempts
    pub total_fetch_attempts: u32,

    /// Failed fetch attempts
    pub failed_fetch_attempts: u32,

    /// Total recovery time in milliseconds
    pub recovery_time_ms: u64,
}

/// Recovery orchestrator - coordinates the full recovery process.
///
/// # Example
///
/// ```ignore
/// let registry_fetcher = FreenodRegistryFetcher::new();
/// let chunk_fetcher = FreenodChunkFetcher::new();
/// let config = RecoveryConfig::default();
///
/// let recovered = recover_state(
///     "my-contract-hash",
///     &aci_key,
///     &registry_fetcher,
///     &chunk_fetcher,
///     &config,
/// ).await?;
///
/// println!("Recovered {} bytes", recovered.state.len());
/// ```
pub async fn recover_state<R, C>(
    owner_contract: &str,
    aci_key: &[u8],
    registry_fetcher: &R,
    chunk_fetcher: &C,
    config: &RecoveryConfig,
) -> Result<RecoveredState, RecoveryError>
where
    R: RegistryFetcher,
    C: ChunkFetcher,
{
    let start_time = std::time::Instant::now();

    // 1. Fetch registry to get bot list and epoch
    let registry = registry_fetcher.fetch_registry().await?;
    let epoch = registry.epoch();
    let all_bots: Vec<String> = registry
        .discover_bots()
        .into_iter()
        .map(|entry| entry.contract_hash)
        .collect();

    // 2. Get owner's entry to find num_chunks
    let owner_entry = registry_fetcher
        .get_bot_entry(owner_contract)
        .await?
        .ok_or(RecoveryError::OwnerNotInRegistry)?;

    let num_chunks = owner_entry.num_chunks;

    // 3. Fetch all chunks with fallback
    let mut chunks = Vec::with_capacity(num_chunks as usize);
    let mut stats = RecoveryStats {
        total_chunks: num_chunks,
        chunks_recovered: 0,
        chunks_with_fallback: 0,
        total_fetch_attempts: 0,
        failed_fetch_attempts: 0,
        recovery_time_ms: 0,
    };

    for chunk_index in 0..num_chunks {
        let chunk = fetch_chunk_with_fallback(
            owner_contract,
            chunk_index,
            &all_bots,
            epoch,
            chunk_fetcher,
            config,
            &mut stats,
        )
        .await?;

        chunks.push(chunk);
        stats.chunks_recovered += 1;
    }

    // 4. Decrypt and reassemble
    let state = decrypt_and_reassemble(&chunks, aci_key)?;

    let recovery_time_ms = start_time.elapsed().as_millis() as u64;
    stats.recovery_time_ms = recovery_time_ms;

    Ok(RecoveredState {
        state,
        owner: owner_contract.to_string(),
        num_chunks,
        stats,
    })
}

/// Fetch a single chunk with fallback to alternate holders.
///
/// Tries all available replicas until one succeeds or all fail.
///
/// # Algorithm
///
/// 1. Compute holders using rendezvous hashing
/// 2. Try each holder in order
/// 3. If fetch fails, try next holder
/// 4. Return first successful fetch
/// 5. If all fail, return error
///
/// # Arguments
///
/// * `owner` - Owner's contract hash
/// * `chunk_index` - Index of chunk to fetch
/// * `all_bots` - All registered bots
/// * `epoch` - Current epoch
/// * `chunk_fetcher` - Chunk fetcher implementation
/// * `config` - Recovery configuration
/// * `stats` - Mutable stats for tracking
///
/// # Returns
///
/// The chunk if any replica was available
async fn fetch_chunk_with_fallback<C>(
    owner: &str,
    chunk_index: u32,
    all_bots: &[String],
    epoch: u64,
    chunk_fetcher: &C,
    config: &RecoveryConfig,
    stats: &mut RecoveryStats,
) -> Result<Chunk, RecoveryError>
where
    C: ChunkFetcher,
{
    // Compute holders for this chunk
    let holders = compute_chunk_holders(owner, chunk_index, all_bots, epoch, config.num_replicas);

    let mut last_error = None;
    let mut used_fallback = false;

    // Try each holder in order
    for (attempt, holder) in holders.iter().enumerate() {
        stats.total_fetch_attempts += 1;

        if attempt > 0 {
            used_fallback = true;
        }

        match chunk_fetcher.fetch_chunk(holder, owner, chunk_index).await {
            Ok(chunk) => {
                if used_fallback {
                    stats.chunks_with_fallback += 1;
                }
                return Ok(chunk);
            }
            Err(e) => {
                stats.failed_fetch_attempts += 1;
                last_error = Some(e);
                // Continue to next holder
            }
        }
    }

    // All holders failed
    Err(
        last_error.unwrap_or_else(|| RecoveryError::ChunkFetchFailed {
            chunk_index,
            reason: "All holders unavailable".to_string(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::chunks::encrypt_and_chunk;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock registry fetcher for testing
    struct MockRegistryFetcher {
        registry: PersistenceRegistry,
    }

    impl MockRegistryFetcher {
        fn new(registry: PersistenceRegistry) -> Self {
            Self { registry }
        }
    }

    #[async_trait]
    impl RegistryFetcher for MockRegistryFetcher {
        async fn fetch_registry(&self) -> Result<PersistenceRegistry, RecoveryError> {
            Ok(self.registry.clone())
        }

        async fn get_bot_entry(
            &self,
            contract_hash: &str,
        ) -> Result<Option<RegistryEntry>, RecoveryError> {
            Ok(self
                .registry
                .discover_bots()
                .into_iter()
                .find(|entry| entry.contract_hash == contract_hash))
        }
    }

    // Mock chunk fetcher for testing
    struct MockChunkFetcher {
        chunks: Arc<Mutex<HashMap<(String, u32), Chunk>>>,
        failures: Arc<Mutex<HashMap<String, bool>>>, // holder -> should_fail
    }

    impl MockChunkFetcher {
        fn new() -> Self {
            Self {
                chunks: Arc::new(Mutex::new(HashMap::new())),
                failures: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        async fn store_chunk(&self, holder: &str, chunk: Chunk) {
            let mut chunks = self.chunks.lock().await;
            chunks.insert((holder.to_string(), chunk.index), chunk);
        }

        async fn set_holder_failure(&self, holder: &str, should_fail: bool) {
            let mut failures = self.failures.lock().await;
            failures.insert(holder.to_string(), should_fail);
        }
    }

    #[async_trait]
    impl ChunkFetcher for MockChunkFetcher {
        async fn fetch_chunk(
            &self,
            holder: &str,
            _owner: &str,
            chunk_index: u32,
        ) -> Result<Chunk, RecoveryError> {
            // Check if this holder should fail
            let failures = self.failures.lock().await;
            if failures.get(holder).copied().unwrap_or(false) {
                return Err(RecoveryError::NetworkError(
                    "Simulated network error".to_string(),
                ));
            }

            // Fetch chunk
            let chunks = self.chunks.lock().await;
            chunks
                .get(&(holder.to_string(), chunk_index))
                .cloned()
                .ok_or_else(|| RecoveryError::ChunkFetchFailed {
                    chunk_index,
                    reason: "Chunk not found".to_string(),
                })
        }
    }

    fn test_aci_key() -> Vec<u8> {
        vec![42u8; 32]
    }

    fn test_identity_key(name: &str) -> Vec<u8> {
        // Generate unique but deterministic key based on name
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        hasher.finalize().to_vec()
    }

    #[tokio::test]
    async fn test_successful_recovery() {
        let owner = "owner-bot";
        let aci_key = test_aci_key();
        let original_state = b"test state to recover";

        // Create registry
        let mut registry = PersistenceRegistry::new();
        registry.register(RegistryEntry::new(
            owner.to_string(),
            crate::persistence::SizeBucket::Small,
            1, // Will be updated after chunking
            1000,
            test_identity_key(owner),
        ));
        registry.register(RegistryEntry::new(
            "holder-a".to_string(),
            crate::persistence::SizeBucket::Small,
            0,
            1001,
            test_identity_key("holder-a"),
        ));
        registry.register(RegistryEntry::new(
            "holder-b".to_string(),
            crate::persistence::SizeBucket::Small,
            0,
            1002,
            test_identity_key("holder-b"),
        ));

        // Encrypt and chunk
        let chunks = encrypt_and_chunk(owner, original_state, &aci_key).unwrap();

        // Update registry with actual num_chunks
        let mut registry = PersistenceRegistry::new();
        registry.register(RegistryEntry::new(
            owner.to_string(),
            crate::persistence::SizeBucket::Small,
            chunks.len() as u32,
            1000,
            test_identity_key(owner),
        ));
        registry.register(RegistryEntry::new(
            "holder-a".to_string(),
            crate::persistence::SizeBucket::Small,
            0,
            1001,
            test_identity_key("holder-a"),
        ));
        registry.register(RegistryEntry::new(
            "holder-b".to_string(),
            crate::persistence::SizeBucket::Small,
            0,
            1002,
            test_identity_key("holder-b"),
        ));

        // Store chunks with holders
        let chunk_fetcher = MockChunkFetcher::new();
        for chunk in &chunks {
            let all_bots = registry
                .discover_bots()
                .into_iter()
                .map(|e| e.contract_hash)
                .collect::<Vec<_>>();
            let holders = compute_chunk_holders(owner, chunk.index, &all_bots, registry.epoch(), 2);
            for holder in holders {
                chunk_fetcher.store_chunk(&holder, chunk.clone()).await;
            }
        }

        // Recover
        let registry_fetcher = MockRegistryFetcher::new(registry);
        let config = RecoveryConfig::default();

        let recovered = recover_state(owner, &aci_key, &registry_fetcher, &chunk_fetcher, &config)
            .await
            .unwrap();

        assert_eq!(recovered.state, original_state);
        assert_eq!(recovered.owner, owner);
        assert_eq!(recovered.stats.chunks_recovered, chunks.len() as u32);
        assert_eq!(recovered.stats.failed_fetch_attempts, 0);
    }

    #[tokio::test]
    async fn test_recovery_with_fallback() {
        let owner = "owner-bot";
        let aci_key = test_aci_key();
        let original_state = b"test state needing fallback";

        // Create registry
        let mut registry = PersistenceRegistry::new();
        registry.register(RegistryEntry::new(
            owner.to_string(),
            crate::persistence::SizeBucket::Small,
            1,
            1000,
            test_identity_key(owner),
        ));
        registry.register(RegistryEntry::new(
            "holder-a".to_string(),
            crate::persistence::SizeBucket::Small,
            0,
            1001,
            test_identity_key("holder-a"),
        ));
        registry.register(RegistryEntry::new(
            "holder-b".to_string(),
            crate::persistence::SizeBucket::Small,
            0,
            1002,
            test_identity_key("holder-b"),
        ));
        registry.register(RegistryEntry::new(
            "holder-c".to_string(),
            crate::persistence::SizeBucket::Small,
            0,
            1003,
            test_identity_key("holder-c"),
        ));

        // Encrypt and chunk
        let chunks = encrypt_and_chunk(owner, original_state, &aci_key).unwrap();

        // Update registry with actual num_chunks
        let mut registry = PersistenceRegistry::new();
        registry.register(RegistryEntry::new(
            owner.to_string(),
            crate::persistence::SizeBucket::Small,
            chunks.len() as u32,
            1000,
            test_identity_key(owner),
        ));
        registry.register(RegistryEntry::new(
            "holder-a".to_string(),
            crate::persistence::SizeBucket::Small,
            0,
            1001,
            test_identity_key("holder-a"),
        ));
        registry.register(RegistryEntry::new(
            "holder-b".to_string(),
            crate::persistence::SizeBucket::Small,
            0,
            1002,
            test_identity_key("holder-b"),
        ));
        registry.register(RegistryEntry::new(
            "holder-c".to_string(),
            crate::persistence::SizeBucket::Small,
            0,
            1003,
            test_identity_key("holder-c"),
        ));

        // Store chunks with holders
        let chunk_fetcher = MockChunkFetcher::new();
        for chunk in &chunks {
            let all_bots = registry
                .discover_bots()
                .into_iter()
                .map(|e| e.contract_hash)
                .collect::<Vec<_>>();
            let holders = compute_chunk_holders(owner, chunk.index, &all_bots, registry.epoch(), 2);

            // Store with all holders
            for holder in &holders {
                chunk_fetcher.store_chunk(holder, chunk.clone()).await;
            }

            // Make first holder fail (should fallback to second)
            if !holders.is_empty() {
                chunk_fetcher.set_holder_failure(&holders[0], true).await;
            }
        }

        // Recover (should succeed via fallback)
        let registry_fetcher = MockRegistryFetcher::new(registry);
        let config = RecoveryConfig::default();

        let recovered = recover_state(owner, &aci_key, &registry_fetcher, &chunk_fetcher, &config)
            .await
            .unwrap();

        assert_eq!(recovered.state, original_state);
        assert!(
            recovered.stats.chunks_with_fallback > 0,
            "Should use fallback"
        );
        assert!(
            recovered.stats.failed_fetch_attempts > 0,
            "Should have some failures"
        );
    }

    #[tokio::test]
    async fn test_recovery_fails_when_all_holders_unavailable() {
        let owner = "owner-bot";
        let aci_key = test_aci_key();
        let original_state = b"test state";

        // Create registry
        let mut registry = PersistenceRegistry::new();
        registry.register(RegistryEntry::new(
            owner.to_string(),
            crate::persistence::SizeBucket::Small,
            1,
            1000,
            test_identity_key(owner),
        ));
        registry.register(RegistryEntry::new(
            "holder-a".to_string(),
            crate::persistence::SizeBucket::Small,
            0,
            1001,
            test_identity_key("holder-a"),
        ));
        registry.register(RegistryEntry::new(
            "holder-b".to_string(),
            crate::persistence::SizeBucket::Small,
            0,
            1002,
            test_identity_key("holder-b"),
        ));

        // Encrypt and chunk
        let chunks = encrypt_and_chunk(owner, original_state, &aci_key).unwrap();

        // Update registry
        let mut registry = PersistenceRegistry::new();
        registry.register(RegistryEntry::new(
            owner.to_string(),
            crate::persistence::SizeBucket::Small,
            chunks.len() as u32,
            1000,
            test_identity_key(owner),
        ));
        registry.register(RegistryEntry::new(
            "holder-a".to_string(),
            crate::persistence::SizeBucket::Small,
            0,
            1001,
            test_identity_key("holder-a"),
        ));
        registry.register(RegistryEntry::new(
            "holder-b".to_string(),
            crate::persistence::SizeBucket::Small,
            0,
            1002,
            test_identity_key("holder-b"),
        ));

        // Don't store chunks - all holders will be unavailable
        let chunk_fetcher = MockChunkFetcher::new();

        // Recover should fail
        let registry_fetcher = MockRegistryFetcher::new(registry);
        let config = RecoveryConfig::default();

        let result =
            recover_state(owner, &aci_key, &registry_fetcher, &chunk_fetcher, &config).await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(RecoveryError::ChunkFetchFailed { .. })
        ));
    }
}
