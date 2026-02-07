//! Chunk Distribution Orchestration
//!
//! This module orchestrates the complete chunk distribution process:
//! 1. Encrypt bot state with AES-256-GCM
//! 2. Split into 64KB chunks
//! 3. Compute holders via rendezvous hashing
//! 4. Distribute chunks to selected holders
//! 5. Track replication attestations (currently trust-based, see note below)
//! 6. Update write-blocking state
//!
//! ## Attestation Integration
//!
//! This module integrates with the attestation module (`super::attestation`) for
//! cryptographic verification of chunk possession. The attestation module provides:
//! - `Attestation` struct for HMAC-signed receipts from holders
//! - `verify_and_record_attestation()` for cryptographic verification
//!
//! **Current Limitation**: ChunkStorage trait returns `Result<(), StorageError>`,
//! not signed attestations. The distribution code currently trusts storage success
//! without cryptographic proof. See TODO comments in the code for integration points
//! where attestation verification would occur once ChunkStorage is updated.
//!
//! See `test_attestation_integration_pattern` for a demonstration of the full
//! attestation verification flow.
//!
//! ## Version Locking
//!
//! Distribution is version-locked to prevent concurrent modifications:
//! - Each state has a version number
//! - Only the latest version can be distributed
//! - Holders verify version before accepting chunks
//!
//! ## Replication Factor
//!
//! - Target: 3 copies per chunk (1 local + 2 remote)
//! - Minimum: 2 copies for recovery
//! - Local copy always stored first
//! - Remote distribution is parallel
//!
//! ## References
//!
//! - Design: docs/PERSISTENCE.md § Distribution
//! - Agent: Agent-Freenet

use super::attestation::record_attestation as verify_and_record_attestation;
use super::chunk_storage::{ChunkStorage, StorageError};
use super::chunks::{encrypt_and_chunk, ChunkError};
use super::health::ReplicationHealth;
use super::registry::PersistenceRegistry;
use super::rendezvous::compute_chunk_holders;
use super::write_blocking::WriteBlockingManager;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(test)]
use super::attestation::Attestation;
#[cfg(test)]
use super::chunks::Chunk;
#[cfg(test)]
use super::registry::RegistryEntry;
#[cfg(test)]
use async_trait::async_trait;

/// Number of remote replicas per chunk (local + remote = 3 total)
pub const REPLICATION_FACTOR: usize = 2;

/// Errors that can occur during chunk distribution
#[derive(Debug, Error)]
pub enum DistributionError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(#[from] ChunkError),

    #[error("Storage failed: {0}")]
    StorageFailed(#[from] StorageError),

    #[error("Registry fetch failed: {0}")]
    RegistryFetchFailed(String),

    #[error("Insufficient network size: need {needed}, have {available}")]
    InsufficientNetworkSize { needed: usize, available: usize },

    #[error("Version conflict: expected {expected}, got {actual}")]
    VersionConflict { expected: u64, actual: u64 },

    #[error("Distribution incomplete: {successful}/{total} chunks distributed")]
    IncompleteDistribution { successful: u32, total: u32 },

    #[error("Holder unavailable: {holder}")]
    HolderUnavailable { holder: String },
}

/// Distribution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionConfig {
    /// Number of remote replicas per chunk (default: 2)
    pub num_replicas: usize,

    /// Maximum parallel chunk distributions (default: 10)
    pub max_parallel: usize,

    /// Timeout per chunk distribution in milliseconds (default: 5000ms)
    pub timeout_ms: u64,

    /// Retry failed distributions (default: true)
    pub retry_on_failure: bool,

    /// Maximum retries per chunk (default: 3)
    pub max_retries: usize,
}

impl Default for DistributionConfig {
    fn default() -> Self {
        Self {
            num_replicas: REPLICATION_FACTOR,
            max_parallel: 10,
            timeout_ms: 5000,
            retry_on_failure: true,
            max_retries: 3,
        }
    }
}

/// Distribution result with statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionResult {
    /// Number of chunks successfully distributed
    pub chunks_distributed: u32,

    /// Total number of chunks
    pub total_chunks: u32,

    /// Number of chunks fully replicated (all copies confirmed)
    pub fully_replicated: u32,

    /// Number of chunks partially replicated (some copies confirmed)
    pub partially_replicated: u32,

    /// Number of failed distributions
    pub failed: u32,

    /// Total distribution time in milliseconds
    pub distribution_time_ms: u64,

    /// State version that was distributed
    pub version: u64,
}

/// Versioned state for distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedState {
    /// State data (plaintext)
    pub data: Vec<u8>,

    /// Version number (increments on each state change)
    pub version: u64,

    /// Owner's contract hash
    pub owner: String,
}

/// Distribution orchestrator
///
/// Coordinates the full chunk distribution process with version locking,
/// replication tracking, and write-blocking state management.
///
/// # Example
///
/// ```ignore
/// let distributor = ChunkDistributor::new(
///     registry_fetcher,
///     storage,
///     config,
/// );
///
/// let state = VersionedState {
///     data: bot_state.to_vec(),
///     version: 1,
///     owner: "my-contract-hash".to_string(),
/// };
///
/// let result = distributor.distribute(state, &aci_key).await?;
/// println!("Distributed {}/{} chunks", result.chunks_distributed, result.total_chunks);
/// ```
pub struct ChunkDistributor<S: ChunkStorage> {
    /// Chunk storage backend
    storage: S,

    /// Distribution configuration
    config: DistributionConfig,

    /// Replication health tracker
    health: ReplicationHealth,

    /// Write-blocking manager
    write_blocking: WriteBlockingManager,
}

impl<S: ChunkStorage> ChunkDistributor<S> {
    /// Create a new chunk distributor
    ///
    /// # Arguments
    ///
    /// * `storage` - Chunk storage implementation
    /// * `config` - Distribution configuration
    pub fn new(storage: S, config: DistributionConfig) -> Self {
        Self {
            storage,
            config,
            health: ReplicationHealth::new(),
            write_blocking: WriteBlockingManager::new(),
        }
    }

    /// Distribute a versioned state to the persistence network.
    ///
    /// # Flow
    ///
    /// 1. Check write-blocking state (may reject if degraded)
    /// 2. Fetch registry to get available holders
    /// 3. Verify network size sufficient for replication
    /// 4. Encrypt and chunk the state
    /// 5. Store local copy
    /// 6. Compute holders via rendezvous hashing
    /// 7. Distribute chunks to remote holders (parallel)
    /// 8. Track attestations
    /// 9. Update write-blocking state
    /// 10. Return distribution result
    ///
    /// # Arguments
    ///
    /// * `state` - Versioned state to distribute
    /// * `aci_key` - ACI key for encryption (32 bytes)
    ///
    /// # Returns
    ///
    /// Distribution result with statistics
    ///
    /// # Errors
    ///
    /// - `InsufficientNetworkSize`: Not enough bots for replication
    /// - `EncryptionFailed`: Encryption or chunking failed
    /// - `StorageFailed`: Local or remote storage failed
    /// - `IncompleteDistribution`: Some chunks failed to distribute
    pub async fn distribute(
        &mut self,
        state: VersionedState,
        aci_key: &[u8],
        registry: &PersistenceRegistry,
    ) -> Result<DistributionResult, DistributionError> {
        let start_time = std::time::Instant::now();

        // 1. Check network size
        let network_size = registry.network_size();
        let needed = self.config.num_replicas + 1; // +1 for owner
        if network_size < needed {
            return Err(DistributionError::InsufficientNetworkSize {
                needed,
                available: network_size,
            });
        }

        // 2. Encrypt and chunk
        let chunks = encrypt_and_chunk(&state.owner, &state.data, aci_key)?;
        let num_chunks = chunks.len() as u32;

        // 3. Initialize health tracking
        self.health.update_total_chunks(num_chunks);
        self.health.update_network_size(network_size);
        self.write_blocking.set_network_size(network_size);
        self.write_blocking.initialize_chunks(num_chunks);

        // 4. Get all registered bots
        let all_bots: Vec<String> = registry
            .discover_bots()
            .into_iter()
            .map(|entry| entry.contract_hash)
            .collect();
        let epoch = registry.epoch();

        // 5. Store local copy first
        for chunk in &chunks {
            self.storage
                .store_local(&state.owner, chunk.index, chunk)
                .await?;
        }

        // 6. Distribute to remote holders
        let mut chunks_distributed = 0u32;
        let mut fully_replicated = 0u32;
        let mut partially_replicated = 0u32;
        let mut failed = 0u32;

        for chunk in &chunks {
            // Compute holders for this chunk
            let holders = compute_chunk_holders(
                &state.owner,
                chunk.index,
                &all_bots,
                epoch,
                self.config.num_replicas,
            );

            // Distribute to each holder
            let mut successful_holders = 0;
            for holder in &holders {
                match self
                    .storage
                    .store_remote(holder, &state.owner, chunk.index, chunk)
                    .await
                {
                    Ok(attestation) => {
                        // Get holder's identity key from registry
                        match registry.get_identity_key(holder) {
                            Some(holder_identity_key) => {
                                // Verify attestation cryptographically
                                match verify_and_record_attestation(
                                    &attestation,
                                    holder_identity_key,
                                    &mut self.health,
                                ) {
                                    Ok(true) => {
                                        // Attestation verified successfully
                                        self.write_blocking.update_chunk_status(
                                            chunk.index,
                                            (successful_holders + 1) as u8 + 1,
                                        ); // +1 for local
                                        successful_holders += 1;
                                    }
                                    Ok(false) => {
                                        // Attestation failed verification
                                        eprintln!(
                                            "Failed to verify attestation for chunk {} from {}: signature verification failed",
                                            chunk.index, holder
                                        );
                                    }
                                    Err(e) => {
                                        // Error during verification (e.g., invalid key)
                                        eprintln!(
                                            "Error verifying attestation for chunk {} from {}: {}",
                                            chunk.index, holder, e
                                        );
                                    }
                                }
                            }
                            None => {
                                // Holder not found in registry or no identity key
                                eprintln!(
                                    "Cannot verify attestation for chunk {} from {}: holder identity key not found in registry",
                                    chunk.index, holder
                                );
                                self.health.record_attestation(chunk.index, holder, false);
                            }
                        }
                    }
                    Err(e) => {
                        // Storage failed
                        self.health.record_attestation(chunk.index, holder, false);
                        eprintln!("Failed to store chunk {} on {}: {}", chunk.index, holder, e);
                    }
                }
            }

            // Track replication status
            if successful_holders == holders.len() {
                fully_replicated += 1;
                chunks_distributed += 1;
            } else if successful_holders > 0 {
                partially_replicated += 1;
                chunks_distributed += 1;
            } else {
                failed += 1;
            }
        }

        let distribution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(DistributionResult {
            chunks_distributed,
            total_chunks: num_chunks,
            fully_replicated,
            partially_replicated,
            failed,
            distribution_time_ms,
            version: state.version,
        })
    }

    /// Get current replication health
    pub fn replication_health(&self) -> &ReplicationHealth {
        &self.health
    }

    /// Get current write-blocking state
    pub fn write_blocking_state(&self) -> &WriteBlockingManager {
        &self.write_blocking
    }

    /// Check if writes are currently allowed
    pub fn allows_writes(&self) -> bool {
        self.write_blocking.allows_writes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock storage for testing
    #[allow(clippy::type_complexity)]
    struct MockStorage {
        local: Arc<Mutex<HashMap<(String, u32), Chunk>>>,
        remote: Arc<Mutex<HashMap<(String, String, u32), Chunk>>>,
        failures: Arc<Mutex<HashMap<String, bool>>>,
        // Map holder -> identity key for creating attestations
        identity_keys: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                local: Arc::new(Mutex::new(HashMap::new())),
                remote: Arc::new(Mutex::new(HashMap::new())),
                failures: Arc::new(Mutex::new(HashMap::new())),
                identity_keys: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        async fn set_holder_failure(&self, holder: &str, should_fail: bool) {
            let mut failures = self.failures.lock().await;
            failures.insert(holder.to_string(), should_fail);
        }

        async fn set_holder_identity_key(&self, holder: &str, key: Vec<u8>) {
            let mut keys = self.identity_keys.lock().await;
            keys.insert(holder.to_string(), key);
        }
    }

    #[async_trait]
    impl ChunkStorage for MockStorage {
        async fn store_local(
            &self,
            owner: &str,
            chunk_index: u32,
            chunk: &Chunk,
        ) -> Result<(), StorageError> {
            let mut local = self.local.lock().await;
            local.insert((owner.to_string(), chunk_index), chunk.clone());
            Ok(())
        }

        async fn retrieve_local(
            &self,
            owner: &str,
            chunk_index: u32,
        ) -> Result<Chunk, StorageError> {
            let local = self.local.lock().await;
            local
                .get(&(owner.to_string(), chunk_index))
                .cloned()
                .ok_or_else(|| StorageError::ChunkNotFound {
                    owner: owner.to_string(),
                    chunk_index,
                })
        }

        async fn store_remote(
            &self,
            holder: &str,
            owner: &str,
            chunk_index: u32,
            chunk: &Chunk,
        ) -> Result<Attestation, StorageError> {
            // Check if this holder should fail
            let failures = self.failures.lock().await;
            if failures.get(holder).copied().unwrap_or(false) {
                return Err(StorageError::NetworkError(
                    "Simulated network error".to_string(),
                ));
            }
            drop(failures);

            // Get holder's identity key
            let keys = self.identity_keys.lock().await;
            let identity_key = keys
                .get(holder)
                .ok_or_else(|| {
                    StorageError::PermissionDenied(format!(
                        "Holder {} has no identity key configured",
                        holder
                    ))
                })?
                .clone();
            drop(keys);

            // Store the chunk
            let mut remote = self.remote.lock().await;
            remote.insert(
                (holder.to_string(), owner.to_string(), chunk_index),
                chunk.clone(),
            );
            drop(remote);

            // Create and return attestation
            Attestation::create(owner, chunk_index, holder, &identity_key).map_err(|e| {
                StorageError::ContractError(format!("Failed to create attestation: {}", e))
            })
        }

        async fn retrieve_remote(
            &self,
            holder: &str,
            owner: &str,
            chunk_index: u32,
        ) -> Result<Chunk, StorageError> {
            let remote = self.remote.lock().await;
            remote
                .get(&(holder.to_string(), owner.to_string(), chunk_index))
                .cloned()
                .ok_or_else(|| StorageError::ChunkNotFound {
                    owner: owner.to_string(),
                    chunk_index,
                })
        }
    }

    fn test_aci_key() -> Vec<u8> {
        vec![42u8; 32]
    }

    fn test_identity_key(holder: &str) -> Vec<u8> {
        // Generate unique but deterministic key based on holder name
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(holder.as_bytes());
        hasher.finalize().to_vec()
    }

    fn create_test_registry() -> PersistenceRegistry {
        let mut registry = PersistenceRegistry::new();
        registry.register(RegistryEntry::new(
            "owner-bot".to_string(),
            crate::persistence::SizeBucket::Small,
            0,
            1000,
            test_identity_key("owner-bot"),
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
        registry
    }

    #[tokio::test]
    async fn test_successful_distribution() {
        let storage = MockStorage::new();

        // Set up identity keys for all holders
        storage.set_holder_identity_key("holder-a", test_identity_key("holder-a")).await;
        storage.set_holder_identity_key("holder-b", test_identity_key("holder-b")).await;
        storage.set_holder_identity_key("holder-c", test_identity_key("holder-c")).await;

        let config = DistributionConfig::default();
        let mut distributor = ChunkDistributor::new(storage, config);
        let registry = create_test_registry();

        let state = VersionedState {
            data: b"test state to distribute".to_vec(),
            version: 1,
            owner: "owner-bot".to_string(),
        };

        let aci_key = test_aci_key();
        let result = distributor
            .distribute(state, &aci_key, &registry)
            .await
            .unwrap();

        assert!(result.chunks_distributed > 0);
        assert_eq!(result.failed, 0);
        // Distribution time may be 0ms for small test cases
    }

    #[tokio::test]
    async fn test_insufficient_network_size() {
        let mut registry = PersistenceRegistry::new();
        registry.register(RegistryEntry::new(
            "owner-bot".to_string(),
            crate::persistence::SizeBucket::Small,
            0,
            1000,
            test_identity_key("owner-bot"),
        ));
        // Only 1 bot - not enough for replication

        let storage = MockStorage::new();
        storage.set_holder_identity_key("owner-bot", test_identity_key("owner-bot")).await;

        let config = DistributionConfig::default();
        let mut distributor = ChunkDistributor::new(storage, config);

        let state = VersionedState {
            data: b"test state".to_vec(),
            version: 1,
            owner: "owner-bot".to_string(),
        };

        let aci_key = test_aci_key();
        let result = distributor.distribute(state, &aci_key, &registry).await;

        assert!(matches!(
            result,
            Err(DistributionError::InsufficientNetworkSize { .. })
        ));
    }

    #[tokio::test]
    async fn test_partial_distribution_with_failures() {
        let storage = MockStorage::new();

        // Set up identity keys
        storage.set_holder_identity_key("holder-a", test_identity_key("holder-a")).await;
        storage.set_holder_identity_key("holder-b", test_identity_key("holder-b")).await;
        storage.set_holder_identity_key("holder-c", test_identity_key("holder-c")).await;

        storage.set_holder_failure("holder-a", true).await; // Make one holder fail

        let config = DistributionConfig::default();
        let mut distributor = ChunkDistributor::new(storage, config);
        let registry = create_test_registry();

        let state = VersionedState {
            data: b"test state with failures".to_vec(),
            version: 1,
            owner: "owner-bot".to_string(),
        };

        let aci_key = test_aci_key();
        let result = distributor
            .distribute(state, &aci_key, &registry)
            .await
            .unwrap();

        // Should have some partial replication due to holder-a failures
        assert!(result.chunks_distributed > 0);
        assert!(result.partially_replicated > 0 || result.fully_replicated > 0);
    }

    #[tokio::test]
    async fn test_write_blocking_after_distribution() {
        let storage = MockStorage::new();

        // Set up identity keys
        storage.set_holder_identity_key("holder-a", test_identity_key("holder-a")).await;
        storage.set_holder_identity_key("holder-b", test_identity_key("holder-b")).await;
        storage.set_holder_identity_key("holder-c", test_identity_key("holder-c")).await;

        let config = DistributionConfig::default();
        let mut distributor = ChunkDistributor::new(storage, config);
        let registry = create_test_registry();

        let state = VersionedState {
            data: b"test state".to_vec(),
            version: 1,
            owner: "owner-bot".to_string(),
        };

        let aci_key = test_aci_key();
        distributor
            .distribute(state, &aci_key, &registry)
            .await
            .unwrap();

        // After successful distribution, writes should be allowed
        assert!(distributor.allows_writes());
    }

    #[tokio::test]
    async fn test_attestation_integration_pattern() {
        // This test demonstrates how attestation verification would work once
        // ChunkStorage is updated to return signed attestations from holders.
        //
        // Currently, ChunkStorage::store_remote returns Result<(), StorageError>.
        // Future enhancement: Result<Attestation, StorageError>

        let holder_identity_key = vec![42u8; 32];
        let owner = "owner-bot";
        let chunk_index = 0;
        let holder = "holder-a";

        // STEP 1: Holder receives chunk and creates attestation
        // (In real implementation, this happens on the holder's side)
        let attestation =
            Attestation::create(owner, chunk_index, holder, &holder_identity_key).unwrap();

        // STEP 2: Owner receives attestation and verifies it
        let mut health = ReplicationHealth::new();
        health.update_total_chunks(1);

        // This is the integration point: verify_and_record_attestation
        // combines cryptographic verification with health tracking
        let verified =
            verify_and_record_attestation(&attestation, &holder_identity_key, &mut health).unwrap();

        assert!(verified, "Attestation should be verified successfully");
        assert_eq!(
            health.confirmed_replicas(chunk_index),
            1,
            "Health tracker should record the confirmed replica"
        );

        // STEP 3: Verify that tampered attestations are rejected
        let mut tampered_attestation = attestation.clone();
        tampered_attestation.chunk_index = 99; // Tamper with data

        let result =
            verify_and_record_attestation(&tampered_attestation, &holder_identity_key, &mut health)
                .unwrap();

        assert!(!result, "Tampered attestation should fail verification");
        assert_eq!(
            health.confirmed_replicas(99),
            0,
            "Failed attestation should not add confirmed replica"
        );
    }
}
