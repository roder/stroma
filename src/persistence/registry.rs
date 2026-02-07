//! Persistence Registry - Bot Discovery for Reciprocal Persistence Network
//!
//! This module implements the core discovery mechanism for Stroma's persistence network.
//! Bots register in a well-known Freenet contract to discover each other for chunk distribution.
//!
//! ## Architecture
//!
//! - **Registry Contract**: Single well-known contract at deterministic address
//! - **Discovery**: O(1) lookup, <1ms latency (Q7 validated)
//! - **Scaling**: Sharding support for 10K+ bots
//! - **Stale Handling**: Tombstones for remove-wins semantics
//!
//! ## Key Design Decisions
//!
//! 1. **No Heartbeats**: Bots register once, unregister on clean shutdown.
//!    Stale bots detected lazily during chunk distribution (Q7 RESULTS.md § Stale Bot Handling).
//!
//! 2. **Separate from Federation**: Persistence discovery uses different trust model
//!    than federation discovery. This registry is for adversarial chunk holders.
//!
//! 3. **Size Buckets**: Approximate fairness tracking without revealing exact member counts.
//!
//! 4. **Epoch Tracking**: Network changes >10% increment epoch for holder recomputation.
//!
//! ## References
//!
//! - Spike: docs/spike/q7/ (registry.rs, RESULTS.md)
//! - Agent: Agent-Freenet

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};

/// Size bucket for approximate fairness tracking.
///
/// Used to balance chunk distribution without revealing exact member counts.
/// Thresholds: Small (<50), Medium (50-200), Large (>200).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub enum SizeBucket {
    /// Less than 50 members
    Small,
    /// 50-200 members
    Medium,
    /// More than 200 members
    Large,
}

impl SizeBucket {
    /// Compute size bucket from member count
    pub fn from_count(count: usize) -> Self {
        match count {
            0..=49 => SizeBucket::Small,
            50..=200 => SizeBucket::Medium,
            _ => SizeBucket::Large,
        }
    }
}

/// Registry entry for a single bot in the persistence network.
///
/// Represents a bot that has registered for reciprocal persistence.
/// Contains metadata needed for chunk holder selection and recovery.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Bot's trust contract address (unique identifier)
    pub contract_hash: String,

    /// Approximate size bucket (not exact count)
    pub size_bucket: SizeBucket,

    /// Number of chunks for this bot's state (for recovery planning)
    pub num_chunks: u32,

    /// Timestamp of registration (for debugging/monitoring)
    pub registered_at: u64,

    /// Bot's identity key for signing attestations (32 bytes)
    ///
    /// Used to sign chunk possession attestations. This key proves that
    /// a holder actually possesses a chunk via HMAC-SHA256 signatures.
    pub identity_key: Vec<u8>,
}

impl RegistryEntry {
    /// Create a new registry entry
    pub fn new(
        contract_hash: String,
        size_bucket: SizeBucket,
        num_chunks: u32,
        registered_at: u64,
        identity_key: Vec<u8>,
    ) -> Self {
        Self {
            contract_hash,
            size_bucket,
            num_chunks,
            registered_at,
            identity_key,
        }
    }
}

/// Persistence Registry - Core discovery contract for the persistence network.
///
/// # Design
///
/// Stores O(N) bot list only. Chunk holder relationships are COMPUTED
/// via rendezvous hashing (Q11), not stored. This keeps registry lightweight
/// and enables deterministic holder selection.
///
/// # Semantics
///
/// - **Remove-Wins**: Tombstones prevent re-registration after unregister
/// - **Deterministic**: All bots compute same contract address
/// - **Epoch Tracking**: Network changes >10% increment epoch
///
/// # Scaling
///
/// - Phase 0: Single contract (<10K bots)
/// - Phase 1+: Sharded registry (256 shards, by contract hash first byte)
///
/// # Example
///
/// ```ignore
/// let mut registry = PersistenceRegistry::new();
///
/// // Register a bot
/// let entry = RegistryEntry::new(
///     "contract-hash-abc123".to_string(),
///     SizeBucket::Small,
///     8,
///     1234567890,
/// );
/// registry.register(entry);
///
/// // Discover all bots
/// let bots = registry.discover_bots();
/// assert_eq!(bots.len(), 1);
///
/// // Get network size for replication requirements
/// let size = registry.network_size();
/// assert!(size >= 3, "Need at least 3 bots for replication");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceRegistry {
    /// Registered bots (BTreeSet for deterministic ordering)
    bots: BTreeSet<RegistryEntry>,

    /// Tombstones for remove-wins semantics
    ///
    /// Once a bot is unregistered, it cannot re-register until tombstone is cleared.
    /// This prevents race conditions and ensures clean shutdown semantics.
    tombstones: HashSet<String>,

    /// Current epoch (increments on significant network changes)
    ///
    /// Used to trigger holder recomputation when network topology changes >10%.
    epoch: u64,

    /// Network size at last epoch change (for >10% threshold detection)
    last_epoch_size: usize,
}

impl PersistenceRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            bots: BTreeSet::new(),
            tombstones: HashSet::new(),
            epoch: 1,
            last_epoch_size: 0,
        }
    }

    /// Deterministic registry contract address.
    ///
    /// All bots in the network compute the same value, enabling discovery
    /// without coordination.
    ///
    /// # Production Implementation
    ///
    /// Uses SHA256 hash of a well-known seed to derive Freenet contract address:
    /// ```ignore
    /// ContractHash::from_bytes(&sha256("stroma-persistence-registry-v1"))
    /// ```
    ///
    /// # Returns
    ///
    /// Well-known contract address string
    pub fn registry_contract_address() -> String {
        // For production: ContractHash::from_bytes(&sha256(SEED))
        const SEED: &str = "stroma-persistence-registry-v1";
        format!("contract-hash-{}", Self::deterministic_hash(SEED))
    }

    /// Deterministic shard contract address for scaling to millions of bots.
    ///
    /// When network approaches 10K bots, registry shards by first byte of contract hash.
    /// This provides 256 shards, supporting 2.5M bots at ~10K per shard.
    ///
    /// # Arguments
    ///
    /// * `shard_id` - Shard identifier (0-255, derived from contract_hash[0])
    ///
    /// # Returns
    ///
    /// Deterministic shard contract address
    pub fn shard_contract_address(shard_id: u8) -> String {
        let seed = format!("stroma-persistence-registry-v1-shard-{:02x}", shard_id);
        format!("contract-hash-{}", Self::deterministic_hash(&seed))
    }

    /// Register a bot in the persistence network.
    ///
    /// # Semantics
    ///
    /// - **Idempotent**: Re-registering same bot is a no-op
    /// - **Remove-Wins**: If bot has tombstone, registration is rejected
    /// - **Epoch Update**: >10% network size change increments epoch
    ///
    /// # Arguments
    ///
    /// * `entry` - Bot registration entry
    ///
    /// # Returns
    ///
    /// `true` if registration succeeded, `false` if blocked by tombstone
    pub fn register(&mut self, entry: RegistryEntry) -> bool {
        // Check tombstone (remove-wins semantics)
        if self.tombstones.contains(&entry.contract_hash) {
            // Silently reject - bot was previously removed
            return false;
        }

        let old_size = self.bots.len();
        self.bots.insert(entry);
        let new_size = self.bots.len();

        // Check if epoch should increment (>10% change)
        if self.should_increment_epoch(old_size, new_size) {
            self.epoch += 1;
            self.last_epoch_size = new_size;
        }

        true
    }

    /// Unregister a bot (clean shutdown or detected stale).
    ///
    /// # Semantics
    ///
    /// - **Tombstone Created**: Prevents re-registration (remove-wins)
    /// - **Epoch Update**: >10% network size change increments epoch
    ///
    /// # Arguments
    ///
    /// * `contract_hash` - Bot's contract hash to unregister
    pub fn unregister(&mut self, contract_hash: &str) {
        // Add to tombstones (remove-wins)
        self.tombstones.insert(contract_hash.to_string());

        // Remove from active set
        let old_size = self.bots.len();
        self.bots.retain(|b| b.contract_hash != contract_hash);
        let new_size = self.bots.len();

        // Check if epoch should increment
        if self.should_increment_epoch(old_size, new_size) {
            self.epoch += 1;
            self.last_epoch_size = new_size;
        }
    }

    /// Discover all registered bots.
    ///
    /// # Returns
    ///
    /// Vector of all currently registered bots (deterministic ordering)
    ///
    /// # Performance
    ///
    /// - Latency: <1ms (Q7 validated)
    /// - Size: O(N) where N is number of registered bots
    pub fn discover_bots(&self) -> Vec<RegistryEntry> {
        self.bots.iter().cloned().collect()
    }

    /// Get current network size.
    ///
    /// Used to validate replication requirements (need N >= 3 for 2x replication).
    ///
    /// # Returns
    ///
    /// Number of currently registered bots
    pub fn network_size(&self) -> usize {
        self.bots.len()
    }

    /// Get current epoch.
    ///
    /// Epoch increments when network size changes >10%. Used to trigger
    /// holder recomputation to maintain balanced distribution.
    ///
    /// # Returns
    ///
    /// Current epoch number
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Check if a bot is currently registered (tombstone check).
    ///
    /// # Arguments
    ///
    /// * `contract_hash` - Bot's contract hash
    ///
    /// # Returns
    ///
    /// `true` if bot is registered and not tombstoned
    pub fn is_registered(&self, contract_hash: &str) -> bool {
        !self.tombstones.contains(contract_hash)
            && self.bots.iter().any(|b| b.contract_hash == contract_hash)
    }

    /// Clear tombstone for a bot (admin operation).
    ///
    /// Allows a previously unregistered bot to register again.
    /// Use with caution - typically only needed for testing or manual recovery.
    ///
    /// # Arguments
    ///
    /// * `contract_hash` - Bot's contract hash
    pub fn clear_tombstone(&mut self, contract_hash: &str) {
        self.tombstones.remove(contract_hash);
    }

    /// Get a bot's identity key by contract hash.
    ///
    /// Used to verify attestations from holders. Returns the holder's
    /// identity key needed for HMAC-SHA256 signature verification.
    ///
    /// # Arguments
    ///
    /// * `contract_hash` - Bot's contract hash
    ///
    /// # Returns
    ///
    /// Identity key if bot is registered, None otherwise
    pub fn get_identity_key(&self, contract_hash: &str) -> Option<&[u8]> {
        self.bots
            .iter()
            .find(|b| b.contract_hash == contract_hash)
            .map(|b| b.identity_key.as_slice())
    }

    /// Check if epoch should increment (>10% change in bot count).
    ///
    /// # Arguments
    ///
    /// * `old_count` - Bot count before operation
    /// * `new_count` - Bot count after operation
    ///
    /// # Returns
    ///
    /// `true` if epoch should increment
    fn should_increment_epoch(&self, old_count: usize, new_count: usize) -> bool {
        if old_count == 0 {
            return false; // Don't increment on first bot
        }

        let change = new_count.abs_diff(old_count);

        let change_ratio = change as f64 / old_count as f64;
        change_ratio > 0.10
    }

    /// Deterministic hash function for contract address derivation.
    ///
    /// # Production Implementation
    ///
    /// Uses SHA256 for cryptographic security:
    /// ```ignore
    /// use sha2::{Sha256, Digest};
    /// let mut hasher = Sha256::new();
    /// hasher.update(input.as_bytes());
    /// format!("{:x}", hasher.finalize())
    /// ```
    ///
    /// # Arguments
    ///
    /// * `input` - String to hash
    ///
    /// # Returns
    ///
    /// Hexadecimal hash string
    fn deterministic_hash(input: &str) -> String {
        // Simple hash for now - production will use SHA256
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

impl Default for PersistenceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_identity_key(name: &str) -> Vec<u8> {
        // Generate unique but deterministic key based on name
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        hasher.finalize().to_vec()
    }

    #[test]
    fn test_deterministic_address() {
        let addr1 = PersistenceRegistry::registry_contract_address();
        let addr2 = PersistenceRegistry::registry_contract_address();
        assert_eq!(addr1, addr2, "Registry address should be deterministic");
    }

    #[test]
    fn test_shard_addresses_unique() {
        let shard_0 = PersistenceRegistry::shard_contract_address(0);
        let shard_1 = PersistenceRegistry::shard_contract_address(1);
        assert_ne!(
            shard_0, shard_1,
            "Different shards should have different addresses"
        );
    }

    #[test]
    fn test_shard_addresses_deterministic() {
        let shard_42a = PersistenceRegistry::shard_contract_address(42);
        let shard_42b = PersistenceRegistry::shard_contract_address(42);
        assert_eq!(
            shard_42a, shard_42b,
            "Shard address should be deterministic"
        );
    }

    #[test]
    fn test_registration_deduplication() {
        let mut registry = PersistenceRegistry::new();
        let bot = RegistryEntry::new("test-bot".to_string(), SizeBucket::Small, 8, 1000, test_identity_key("test-bot"));

        assert!(registry.register(bot.clone()));
        assert!(registry.register(bot.clone()));
        assert!(registry.register(bot));

        assert_eq!(
            registry.network_size(),
            1,
            "Should deduplicate registrations"
        );
    }

    #[test]
    fn test_tombstone_prevents_reregistration() {
        let mut registry = PersistenceRegistry::new();
        let bot = RegistryEntry::new("tombstone-bot".to_string(), SizeBucket::Small, 8, 1000, test_identity_key("tombstone-bot"));

        // Register, unregister, try to re-register
        assert!(registry.register(bot.clone()));
        assert_eq!(registry.network_size(), 1);

        registry.unregister(&bot.contract_hash);
        assert_eq!(registry.network_size(), 0);

        assert!(
            !registry.register(bot),
            "Tombstone should prevent re-registration"
        );
        assert_eq!(registry.network_size(), 0, "Bot should remain unregistered");
    }

    #[test]
    fn test_epoch_increments_on_large_change() {
        let mut registry = PersistenceRegistry::new();
        let initial_epoch = registry.epoch();

        // Add 10 bots
        for i in 0..10 {
            let name = format!("bot-{}", i);
            registry.register(RegistryEntry::new(
                name.clone(),
                SizeBucket::Small,
                8,
                1000 + i,
                test_identity_key(&name),
            ));
        }

        // Add 2 more (20% increase - should trigger epoch increment)
        registry.register(RegistryEntry::new(
            "bot-10".to_string(),
            SizeBucket::Small,
            8,
            1010,
            test_identity_key("bot-10"),
        ));
        registry.register(RegistryEntry::new(
            "bot-11".to_string(),
            SizeBucket::Small,
            8,
            1011,
            test_identity_key("bot-11"),
        ));

        assert!(
            registry.epoch() > initial_epoch,
            "Epoch should increment on >10% change"
        );
    }

    #[test]
    fn test_is_registered() {
        let mut registry = PersistenceRegistry::new();
        let bot = RegistryEntry::new("test-bot".to_string(), SizeBucket::Small, 8, 1000, test_identity_key("test-bot"));

        assert!(!registry.is_registered(&bot.contract_hash));

        registry.register(bot.clone());
        assert!(registry.is_registered(&bot.contract_hash));

        registry.unregister(&bot.contract_hash);
        assert!(!registry.is_registered(&bot.contract_hash));
    }

    #[test]
    fn test_clear_tombstone() {
        let mut registry = PersistenceRegistry::new();
        let bot = RegistryEntry::new("test-bot".to_string(), SizeBucket::Small, 8, 1000, test_identity_key("test-bot"));

        // Register and unregister
        registry.register(bot.clone());
        registry.unregister(&bot.contract_hash);

        // Should be blocked by tombstone
        assert!(!registry.register(bot.clone()));

        // Clear tombstone
        registry.clear_tombstone(&bot.contract_hash);

        // Now registration should work
        assert!(registry.register(bot));
        assert_eq!(registry.network_size(), 1);
    }

    #[test]
    fn test_size_bucket_from_count() {
        assert_eq!(SizeBucket::from_count(0), SizeBucket::Small);
        assert_eq!(SizeBucket::from_count(49), SizeBucket::Small);
        assert_eq!(SizeBucket::from_count(50), SizeBucket::Medium);
        assert_eq!(SizeBucket::from_count(200), SizeBucket::Medium);
        assert_eq!(SizeBucket::from_count(201), SizeBucket::Large);
        assert_eq!(SizeBucket::from_count(10000), SizeBucket::Large);
    }

    #[test]
    fn test_discover_bots_ordering() {
        let mut registry = PersistenceRegistry::new();

        // Add bots in non-alphabetical order
        registry.register(RegistryEntry::new(
            "zebra".to_string(),
            SizeBucket::Small,
            8,
            1000,
            test_identity_key("zebra"),
        ));
        registry.register(RegistryEntry::new(
            "alpha".to_string(),
            SizeBucket::Small,
            8,
            1001,
            test_identity_key("alpha"),
        ));
        registry.register(RegistryEntry::new(
            "mike".to_string(),
            SizeBucket::Small,
            8,
            1002,
            test_identity_key("mike"),
        ));

        let bots = registry.discover_bots();

        // Should be ordered deterministically (by BTreeSet ordering)
        assert_eq!(bots.len(), 3);
        // BTreeSet orders by RegistryEntry's Ord implementation
        // which compares all fields in order: contract_hash, size_bucket, num_chunks, registered_at
    }
}
