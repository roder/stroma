//! Integration tests for crash recovery in the Reciprocal Persistence Network.
//!
//! These tests validate the complete crash recovery flow:
//! - Bot stores state → crashes → restarts → recovers state
//! - Primary holder unavailable → fallback to secondary holder
//! - Missing chunk → recovery fails with clear error
//! - Wrong ACI key → decryption fails
//! - Signature mismatch → verification fails
//!
//! ## Test Strategy
//!
//! Uses in-memory mock implementations for:
//! - Registry (simulates Freenet contract state)
//! - Chunk storage (simulates distributed chunk holders)
//! - Network failures (simulates holder unavailability)
//!
//! ## References
//!
//! - Architecture: docs/PERSISTENCE.md § Recovery
//! - Agent: Agent-Freenet

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use stroma::persistence::{
    compute_chunk_holders, encrypt_and_chunk, recover_state, Chunk, ChunkFetcher,
    PersistenceRegistry, RecoveredState, RecoveryConfig, RecoveryError, RegistryEntry,
    RegistryFetcher, SizeBucket,
};
use tokio::sync::Mutex;

// === Test Fixtures ===

fn test_aci_key() -> Vec<u8> {
    vec![42u8; 32]
}

fn different_aci_key() -> Vec<u8> {
    vec![99u8; 32]
}

fn test_identity_key(name: &str) -> Vec<u8> {
    // Generate unique but deterministic key based on name
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hasher.finalize().to_vec()
}

/// Create a test registry with multiple bots
fn create_test_registry(owner: &str, num_chunks: u32) -> PersistenceRegistry {
    let mut registry = PersistenceRegistry::new();

    registry.register(RegistryEntry::new(
        owner.to_string(),
        SizeBucket::Small,
        num_chunks,
        1000,
        test_identity_key(owner),
    ));

    // Add 5 holder bots (more than needed for 2 replicas)
    for i in 0..5 {
        let name = format!("holder-{}", i);
        registry.register(RegistryEntry::new(
            name.clone(),
            SizeBucket::Small,
            0,
            1000 + i,
            test_identity_key(&name),
        ));
    }

    registry
}

// === Mock Implementations ===

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

struct MockChunkStorage {
    chunks: Arc<Mutex<HashMap<(String, u32), Chunk>>>,
    unavailable_holders: Arc<Mutex<HashMap<String, bool>>>,
}

impl MockChunkStorage {
    fn new() -> Self {
        Self {
            chunks: Arc::new(Mutex::new(HashMap::new())),
            unavailable_holders: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn store_chunk(&self, holder: &str, chunk: Chunk) {
        let mut chunks = self.chunks.lock().await;
        chunks.insert((holder.to_string(), chunk.index), chunk);
    }

    async fn mark_holder_unavailable(&self, holder: &str) {
        let mut unavailable = self.unavailable_holders.lock().await;
        unavailable.insert(holder.to_string(), true);
    }
}

#[async_trait]
impl ChunkFetcher for MockChunkStorage {
    async fn fetch_chunk(
        &self,
        holder: &str,
        _owner: &str,
        chunk_index: u32,
    ) -> Result<Chunk, RecoveryError> {
        // Check if holder is unavailable
        let unavailable = self.unavailable_holders.lock().await;
        if unavailable.get(holder).copied().unwrap_or(false) {
            return Err(RecoveryError::NetworkError(format!(
                "Holder {} unavailable",
                holder
            )));
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

/// Simulate full bot lifecycle: store → crash → recover
async fn bot_lifecycle_with_crash(
    owner: &str,
    initial_state: &[u8],
    aci_key: &[u8],
) -> Result<RecoveredState, RecoveryError> {
    // === PHASE 1: Bot stores state ===

    // Encrypt and chunk the state
    let chunks = encrypt_and_chunk(owner, initial_state, aci_key).unwrap();

    // Create registry
    let registry = create_test_registry(owner, chunks.len() as u32);

    // Distribute chunks to holders
    let storage = MockChunkStorage::new();
    let all_bots: Vec<String> = registry
        .discover_bots()
        .into_iter()
        .map(|e| e.contract_hash)
        .collect();

    for chunk in &chunks {
        let holders = compute_chunk_holders(owner, chunk.index, &all_bots, registry.epoch(), 2);
        for holder in holders {
            storage.store_chunk(&holder, chunk.clone()).await;
        }
    }

    // === PHASE 2: Bot crashes (lose all local state) ===
    // (Simulated by not keeping the chunks or state in memory)

    // === PHASE 3: Bot restarts and recovers ===

    let registry_fetcher = MockRegistryFetcher::new(registry);
    let config = RecoveryConfig::default();

    recover_state(owner, aci_key, &registry_fetcher, &storage, &config).await
}

// === Integration Tests ===

#[tokio::test]
async fn test_bot_crash_and_full_recovery() {
    let owner = "test-bot-123";
    let aci_key = test_aci_key();
    let original_state = b"This is important bot state that must survive crashes!";

    let recovered = bot_lifecycle_with_crash(owner, original_state, &aci_key)
        .await
        .expect("Recovery should succeed");

    assert_eq!(
        recovered.state, original_state,
        "Recovered state should match original"
    );
    assert_eq!(recovered.owner, owner);
    assert!(recovered.stats.chunks_recovered > 0);
    assert_eq!(
        recovered.stats.chunks_recovered, recovered.stats.total_chunks,
        "All chunks should be recovered"
    );
}

#[tokio::test]
async fn test_large_state_recovery() {
    let owner = "test-bot-large-state";
    let aci_key = test_aci_key();
    // 200KB state (will create multiple chunks)
    let original_state = vec![7u8; 200 * 1024];

    let recovered = bot_lifecycle_with_crash(owner, &original_state, &aci_key)
        .await
        .expect("Large state recovery should succeed");

    assert_eq!(recovered.state, original_state);
    assert!(
        recovered.stats.total_chunks > 1,
        "Large state should produce multiple chunks"
    );
}

#[tokio::test]
async fn test_recovery_with_primary_holder_unavailable() {
    let owner = "test-bot-fallback";
    let aci_key = test_aci_key();
    let original_state = b"State requiring fallback recovery";

    // Store state
    let chunks = encrypt_and_chunk(owner, original_state, &aci_key).unwrap();
    let registry = create_test_registry(owner, chunks.len() as u32);

    let storage = MockChunkStorage::new();
    let all_bots: Vec<String> = registry
        .discover_bots()
        .into_iter()
        .map(|e| e.contract_hash)
        .collect();

    // Distribute chunks and mark primary holders unavailable
    for chunk in &chunks {
        let holders = compute_chunk_holders(owner, chunk.index, &all_bots, registry.epoch(), 2);

        // Store with all holders
        for holder in &holders {
            storage.store_chunk(holder, chunk.clone()).await;
        }

        // Mark first holder (primary) as unavailable
        if !holders.is_empty() {
            storage.mark_holder_unavailable(&holders[0]).await;
        }
    }

    // Recover (should succeed via fallback to secondary holders)
    let registry_fetcher = MockRegistryFetcher::new(registry);
    let config = RecoveryConfig::default();

    let recovered = recover_state(owner, &aci_key, &registry_fetcher, &storage, &config)
        .await
        .expect("Recovery should succeed via fallback");

    assert_eq!(recovered.state, original_state);
    assert!(
        recovered.stats.chunks_with_fallback > 0,
        "Should have used fallback holders"
    );
    assert!(
        recovered.stats.failed_fetch_attempts > 0,
        "Should have some failed fetch attempts"
    );
}

#[tokio::test]
async fn test_recovery_fails_with_all_holders_unavailable() {
    let owner = "test-bot-no-holders";
    let aci_key = test_aci_key();
    let original_state = b"State with no available holders";

    // Store state
    let chunks = encrypt_and_chunk(owner, original_state, &aci_key).unwrap();
    let registry = create_test_registry(owner, chunks.len() as u32);

    let storage = MockChunkStorage::new();
    let all_bots: Vec<String> = registry
        .discover_bots()
        .into_iter()
        .map(|e| e.contract_hash)
        .collect();

    // Distribute chunks
    for chunk in &chunks {
        let holders = compute_chunk_holders(owner, chunk.index, &all_bots, registry.epoch(), 2);
        for holder in &holders {
            storage.store_chunk(holder, chunk.clone()).await;
        }
    }

    // Mark ALL holders unavailable
    for bot in &all_bots {
        if bot != owner {
            storage.mark_holder_unavailable(bot).await;
        }
    }

    // Recover should fail
    let registry_fetcher = MockRegistryFetcher::new(registry);
    let config = RecoveryConfig::default();

    let result = recover_state(owner, &aci_key, &registry_fetcher, &storage, &config).await;

    assert!(
        result.is_err(),
        "Recovery should fail when no holders available"
    );
    // Should fail with either ChunkFetchFailed or NetworkError
    assert!(matches!(
        result,
        Err(RecoveryError::ChunkFetchFailed { .. }) | Err(RecoveryError::NetworkError(_))
    ));
}

#[tokio::test]
async fn test_recovery_fails_with_wrong_aci_key() {
    let owner = "test-bot-wrong-key";
    let correct_aci_key = test_aci_key();
    let wrong_aci_key = different_aci_key();
    let original_state = b"State encrypted with correct key";

    // Store state with correct key
    let chunks = encrypt_and_chunk(owner, original_state, &correct_aci_key).unwrap();
    let registry = create_test_registry(owner, chunks.len() as u32);

    let storage = MockChunkStorage::new();
    let all_bots: Vec<String> = registry
        .discover_bots()
        .into_iter()
        .map(|e| e.contract_hash)
        .collect();

    for chunk in &chunks {
        let holders = compute_chunk_holders(owner, chunk.index, &all_bots, registry.epoch(), 2);
        for holder in holders {
            storage.store_chunk(&holder, chunk.clone()).await;
        }
    }

    // Try to recover with wrong key
    let registry_fetcher = MockRegistryFetcher::new(registry);
    let config = RecoveryConfig::default();

    let result = recover_state(owner, &wrong_aci_key, &registry_fetcher, &storage, &config).await;

    assert!(result.is_err(), "Recovery should fail with wrong ACI key");
    // Should fail during signature verification or decryption
    assert!(matches!(result, Err(RecoveryError::DecryptionFailed(_))));
}

#[tokio::test]
async fn test_recovery_fails_with_tampered_chunk() {
    let owner = "test-bot-tampered";
    let aci_key = test_aci_key();
    let original_state = b"State that will be tampered with";

    // Store state
    let mut chunks = encrypt_and_chunk(owner, original_state, &aci_key).unwrap();
    let registry = create_test_registry(owner, chunks.len() as u32);

    // Tamper with first chunk's data (but keep signature)
    if !chunks.is_empty() {
        chunks[0].data[0] ^= 0xFF; // Flip bits in first byte
    }

    let storage = MockChunkStorage::new();
    let all_bots: Vec<String> = registry
        .discover_bots()
        .into_iter()
        .map(|e| e.contract_hash)
        .collect();

    // Store tampered chunks
    for chunk in &chunks {
        let holders = compute_chunk_holders(owner, chunk.index, &all_bots, registry.epoch(), 2);
        for holder in holders {
            storage.store_chunk(&holder, chunk.clone()).await;
        }
    }

    // Recovery should fail signature verification
    let registry_fetcher = MockRegistryFetcher::new(registry);
    let config = RecoveryConfig::default();

    let result = recover_state(owner, &aci_key, &registry_fetcher, &storage, &config).await;

    assert!(result.is_err(), "Recovery should fail with tampered chunk");
    assert!(matches!(result, Err(RecoveryError::DecryptionFailed(_))));
}

#[tokio::test]
async fn test_recovery_with_missing_chunk() {
    let owner = "test-bot-missing-chunk";
    let aci_key = test_aci_key();
    let original_state = vec![1u8; 200 * 1024]; // Large enough for multiple chunks

    // Store state
    let chunks = encrypt_and_chunk(owner, &original_state, &aci_key).unwrap();
    let registry = create_test_registry(owner, chunks.len() as u32);

    let storage = MockChunkStorage::new();
    let all_bots: Vec<String> = registry
        .discover_bots()
        .into_iter()
        .map(|e| e.contract_hash)
        .collect();

    // Store all chunks except the last one
    for chunk in chunks.iter().take(chunks.len() - 1) {
        let holders = compute_chunk_holders(owner, chunk.index, &all_bots, registry.epoch(), 2);
        for holder in holders {
            storage.store_chunk(&holder, chunk.clone()).await;
        }
    }

    // Recovery should fail due to missing chunk
    let registry_fetcher = MockRegistryFetcher::new(registry);
    let config = RecoveryConfig::default();

    let result = recover_state(owner, &aci_key, &registry_fetcher, &storage, &config).await;

    assert!(result.is_err(), "Recovery should fail with missing chunk");
    assert!(matches!(
        result,
        Err(RecoveryError::ChunkFetchFailed { .. })
    ));
}

#[tokio::test]
async fn test_recovery_stats_accuracy() {
    let owner = "test-bot-stats";
    let aci_key = test_aci_key();
    let original_state = vec![42u8; 150 * 1024]; // Multiple chunks

    let recovered = bot_lifecycle_with_crash(owner, &original_state, &aci_key)
        .await
        .expect("Recovery should succeed");

    // Verify stats
    assert_eq!(
        recovered.stats.chunks_recovered, recovered.stats.total_chunks,
        "All chunks should be reported as recovered"
    );
    assert!(
        recovered.stats.total_fetch_attempts >= recovered.stats.chunks_recovered as u32,
        "Fetch attempts should be at least equal to chunks recovered"
    );
    assert!(
        recovered.stats.recovery_time_ms > 0,
        "Recovery time should be recorded"
    );
    assert_eq!(
        recovered.stats.failed_fetch_attempts, 0,
        "No failures in clean recovery"
    );
}

#[tokio::test]
async fn test_deterministic_holder_selection() {
    let owner = "test-bot-deterministic";
    let aci_key = test_aci_key();
    let original_state = b"State for deterministic holder test";

    // Run recovery twice
    let recovered1 = bot_lifecycle_with_crash(owner, original_state, &aci_key)
        .await
        .expect("First recovery should succeed");

    let recovered2 = bot_lifecycle_with_crash(owner, original_state, &aci_key)
        .await
        .expect("Second recovery should succeed");

    // Both should succeed and produce same result
    assert_eq!(recovered1.state, recovered2.state);
    assert_eq!(recovered1.num_chunks, recovered2.num_chunks);
}
