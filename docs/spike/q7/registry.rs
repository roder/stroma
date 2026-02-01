//! Minimal Persistence Registry Implementation
//!
//! This is a simplified registry for spike validation.
//! Production implementation will use Freenet ComposableState.

use std::collections::{BTreeSet, HashSet};

/// Size bucket for approximate fairness tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SizeBucket {
    Small,   // <50 members
    Medium,  // 50-200 members
    Large,   // >200 members
}

/// Registry entry for a single bot
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegistryEntry {
    pub contract_hash: String,  // Bot's trust contract address
    pub size_bucket: SizeBucket, // Approximate size (not exact count)
    pub num_chunks: u32,         // Number of chunks (for recovery)
    pub registered_at: u64,      // Timestamp of registration
}

/// Minimal persistence registry
///
/// Stores O(N) bot list only. Chunk holder relationships are COMPUTED
/// via rendezvous hashing, not stored.
#[derive(Debug, Clone)]
pub struct PersistenceRegistry {
    /// Registered bots (BTreeSet for deterministic ordering)
    bots: BTreeSet<RegistryEntry>,

    /// Tombstones (remove-wins semantics)
    tombstones: HashSet<String>,

    /// Epoch (increments on significant network changes)
    epoch: u64,

    /// Network size at last epoch change (for >10% threshold)
    last_epoch_size: usize,
}

impl PersistenceRegistry {
    /// Create new empty registry
    pub fn new() -> Self {
        Self {
            bots: BTreeSet::new(),
            tombstones: HashSet::new(),
            epoch: 1,
            last_epoch_size: 0,
        }
    }

    /// Deterministic registry contract address (any bot computes the same value)
    pub fn registry_contract_address() -> String {
        // In production: ContractHash::from_bytes(&sha256(STROMA_REGISTRY_SEED))
        // For spike: simple deterministic string
        const SEED: &str = "stroma-persistence-registry-v1";
        format!("contract-hash-{}", simple_hash(SEED))
    }

    /// Deterministic shard contract address (for scaling to millions of bots)
    pub fn shard_contract_address(shard_id: u8) -> String {
        // In production: hash(format!("stroma-persistence-registry-v1-shard-{:02x}", shard_id))
        format!(
            "contract-hash-{}",
            simple_hash(&format!("stroma-persistence-registry-v1-shard-{:02x}", shard_id))
        )
    }

    /// Register a bot in the persistence network
    pub fn register(&mut self, entry: RegistryEntry) {
        // Check tombstone (remove-wins semantics)
        if self.tombstones.contains(&entry.contract_hash) {
            // Silently reject - bot was previously removed
            return;
        }

        let old_size = self.bots.len();
        self.bots.insert(entry);
        let new_size = self.bots.len();

        // Check if epoch should increment (>10% change)
        if self.should_increment_epoch(old_size, new_size) {
            self.epoch += 1;
            self.last_epoch_size = new_size;
        }
    }

    /// Unregister a bot (clean shutdown or detected stale)
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

    /// Discover all registered bots
    pub fn discover_bots(&self) -> Vec<RegistryEntry> {
        self.bots.iter().cloned().collect()
    }

    /// Get current network size
    pub fn network_size(&self) -> usize {
        self.bots.len()
    }

    /// Get current epoch
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Check if epoch should increment (>10% change in bot count)
    fn should_increment_epoch(&self, old_count: usize, new_count: usize) -> bool {
        if old_count == 0 {
            return false; // Don't increment on first bot
        }

        let change = if new_count > old_count {
            new_count - old_count
        } else {
            old_count - new_count
        };

        let change_ratio = change as f64 / old_count as f64;
        change_ratio > 0.10
    }
}

/// Simple hash function for spike (production uses SHA256)
fn simple_hash(input: &str) -> String {
    // For spike: simple deterministic hash
    // Production: sha256(input).to_hex()
    let mut hash = 0u64;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    format!("{:016x}", hash)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_ne!(shard_0, shard_1, "Different shards should have different addresses");
    }

    #[test]
    fn test_shard_addresses_deterministic() {
        let shard_42a = PersistenceRegistry::shard_contract_address(42);
        let shard_42b = PersistenceRegistry::shard_contract_address(42);
        assert_eq!(shard_42a, shard_42b, "Shard address should be deterministic");
    }

    #[test]
    fn test_registration_deduplication() {
        let mut registry = PersistenceRegistry::new();
        let bot = RegistryEntry {
            contract_hash: "test-bot".to_string(),
            size_bucket: SizeBucket::Small,
            num_chunks: 8,
            registered_at: 1000,
        };

        registry.register(bot.clone());
        registry.register(bot.clone());
        registry.register(bot);

        assert_eq!(registry.network_size(), 1, "Should deduplicate registrations");
    }

    #[test]
    fn test_tombstone_prevents_reregistration() {
        let mut registry = PersistenceRegistry::new();
        let bot = RegistryEntry {
            contract_hash: "tombstone-bot".to_string(),
            size_bucket: SizeBucket::Small,
            num_chunks: 8,
            registered_at: 1000,
        };

        // Register, unregister, try to re-register
        registry.register(bot.clone());
        assert_eq!(registry.network_size(), 1);

        registry.unregister(&bot.contract_hash);
        assert_eq!(registry.network_size(), 0);

        registry.register(bot);
        assert_eq!(
            registry.network_size(),
            0,
            "Tombstone should prevent re-registration"
        );
    }

    #[test]
    fn test_epoch_increments_on_large_change() {
        let mut registry = PersistenceRegistry::new();
        let initial_epoch = registry.epoch();

        // Add 10 bots
        for i in 0..10 {
            registry.register(RegistryEntry {
                contract_hash: format!("bot-{}", i),
                size_bucket: SizeBucket::Small,
                num_chunks: 8,
                registered_at: 1000 + i,
            });
        }

        // Add 2 more (20% increase - should trigger epoch increment)
        registry.register(RegistryEntry {
            contract_hash: "bot-10".to_string(),
            size_bucket: SizeBucket::Small,
            num_chunks: 8,
            registered_at: 1010,
        });
        registry.register(RegistryEntry {
            contract_hash: "bot-11".to_string(),
            size_bucket: SizeBucket::Small,
            num_chunks: 8,
            registered_at: 1011,
        });

        assert!(
            registry.epoch() > initial_epoch,
            "Epoch should increment on >10% change"
        );
    }
}
