//! Write-Blocking State Machine for Stroma Persistence
//!
//! This module implements the write-blocking state machine that ensures
//! trust map changes are only persisted when replication is sufficient.
//!
//! ## Design Philosophy
//!
//! **Availability-based, NOT TTL-based.** Bot never penalized for network scarcity.
//!
//! ## States
//!
//! | State | Condition | Writes | Replication Health |
//! |-------|-----------|--------|-------------------|
//! | **PROVISIONAL** | No suitable peers available | ALLOWED | 🔵 Initializing |
//! | **ACTIVE** | All chunks have 2+ replicas confirmed | ALLOWED | 🟢 Replicated or 🟡 Partial |
//! | **DEGRADED** | Any chunk has ≤1 replica, peers available | **BLOCKED** | 🔴 At Risk |
//! | **ISOLATED** | N=1 network | ALLOWED (warned) | 🔵 Initializing |
//!
//! ## Key Principle
//!
//! If peers are available but distribution failed, you MUST succeed before making changes.
//! This prevents accumulating state that can't be backed up.
//!
//! ## References
//!
//! - Spec: .beads/persistence-model.bead § Write-Blocking States
//! - Agent: Agent-Freenet

use serde::{Deserialize, Serialize};

/// Write-blocking state determines whether writes are allowed.
///
/// State transitions are based on chunk replication health and network availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WriteBlockingState {
    /// No suitable peers available for replication.
    ///
    /// **Writes**: ALLOWED (with warning)
    /// **Replication**: 🔵 Initializing
    /// **Reason**: Bot not penalized for network scarcity
    Provisional,

    /// All chunks have 2+ replicas confirmed.
    ///
    /// **Writes**: ALLOWED
    /// **Replication**: 🟢 Replicated or 🟡 Partial
    /// **Reason**: State is sufficiently backed up
    Active,

    /// Any chunk has ≤1 replica, but peers ARE available.
    ///
    /// **Writes**: **BLOCKED**
    /// **Replication**: 🔴 At Risk
    /// **Reason**: Must fix replication before accumulating more unbackable changes
    Degraded,

    /// Network size is 1 (single bot).
    ///
    /// **Writes**: ALLOWED (with warning)
    /// **Replication**: 🔵 Initializing
    /// **Reason**: Testing scenario, no persistence guarantee
    Isolated,
}

impl WriteBlockingState {
    /// Check if writes are currently allowed in this state.
    ///
    /// # Returns
    ///
    /// `true` if writes are allowed, `false` if blocked
    pub fn allows_writes(&self) -> bool {
        match self {
            WriteBlockingState::Active => true,
            WriteBlockingState::Provisional => true,
            WriteBlockingState::Isolated => true,
            WriteBlockingState::Degraded => false,
        }
    }

    /// Check if this state requires a warning to the user.
    ///
    /// # Returns
    ///
    /// `true` if operator should be warned about degraded persistence
    pub fn requires_warning(&self) -> bool {
        match self {
            WriteBlockingState::Provisional => true,
            WriteBlockingState::Isolated => true,
            WriteBlockingState::Degraded => true,
            WriteBlockingState::Active => false,
        }
    }

    /// Get a human-readable description of this state.
    ///
    /// # Returns
    ///
    /// String describing what this state means for the operator
    pub fn description(&self) -> &'static str {
        match self {
            WriteBlockingState::Provisional => {
                "No suitable peers available - writes allowed but persistence not guaranteed"
            }
            WriteBlockingState::Active => {
                "All chunks replicated - writes allowed, state is recoverable"
            }
            WriteBlockingState::Degraded => {
                "Chunks at risk - writes blocked until replication succeeds"
            }
            WriteBlockingState::Isolated => {
                "Single bot network - writes allowed but no replication possible"
            }
        }
    }
}

/// Chunk replication status for a single chunk.
///
/// Tracks how many confirmed replicas exist for this chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkReplicationStatus {
    /// Chunk index (0-based)
    pub chunk_index: u32,

    /// Number of confirmed replicas (includes local copy)
    ///
    /// Target: 3 (1 local + 2 remote)
    /// Minimum acceptable: 2 (for recovery)
    pub confirmed_replicas: u8,

    /// Total replicas expected (always 3 for current spec)
    pub expected_replicas: u8,
}

impl ChunkReplicationStatus {
    /// Create a new chunk status.
    ///
    /// # Arguments
    ///
    /// * `chunk_index` - Chunk index (0-based)
    /// * `confirmed_replicas` - Number of confirmed replicas (1-3)
    pub fn new(chunk_index: u32, confirmed_replicas: u8) -> Self {
        Self {
            chunk_index,
            confirmed_replicas,
            expected_replicas: 3, // Fixed by spec: 1 local + 2 remote
        }
    }

    /// Check if this chunk is at risk (≤1 replica).
    ///
    /// # Returns
    ///
    /// `true` if chunk has only 0-1 replicas (cannot recover)
    pub fn is_at_risk(&self) -> bool {
        self.confirmed_replicas <= 1
    }

    /// Check if this chunk is fully replicated (all 3 copies).
    ///
    /// # Returns
    ///
    /// `true` if chunk has all expected replicas
    pub fn is_fully_replicated(&self) -> bool {
        self.confirmed_replicas >= self.expected_replicas
    }

    /// Check if this chunk is recoverable (2+ replicas).
    ///
    /// # Returns
    ///
    /// `true` if chunk has at least 2 replicas (minimum for recovery)
    pub fn is_recoverable(&self) -> bool {
        self.confirmed_replicas >= 2
    }
}

/// Replication health metric for user-facing display.
///
/// Answers the fundamental question: "Is my trust network data resilient?"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicationHealth {
    /// All chunks fully replicated (3/3).
    ///
    /// **Recovery**: ✅ Guaranteed
    /// **Display**: "Fully resilient"
    Replicated,

    /// Some chunks degraded (2/3) but all recoverable.
    ///
    /// **Recovery**: ✅ Possible
    /// **Display**: "Recoverable but degraded"
    Partial,

    /// Any chunk at risk (≤1/3).
    ///
    /// **Recovery**: ❌ Not possible
    /// **Display**: "Cannot recover if crash"
    AtRisk,

    /// Establishing replication.
    ///
    /// **Recovery**: —
    /// **Display**: "Setting up persistence"
    Initializing,
}

impl ReplicationHealth {
    /// Convert replication health to emoji for display.
    ///
    /// # Returns
    ///
    /// Emoji representing health status
    pub fn emoji(&self) -> &'static str {
        match self {
            ReplicationHealth::Replicated => "🟢",
            ReplicationHealth::Partial => "🟡",
            ReplicationHealth::AtRisk => "🔴",
            ReplicationHealth::Initializing => "🔵",
        }
    }

    /// Get a human-readable description.
    ///
    /// # Returns
    ///
    /// String describing replication health
    pub fn description(&self) -> &'static str {
        match self {
            ReplicationHealth::Replicated => "Fully resilient",
            ReplicationHealth::Partial => "Recoverable but degraded",
            ReplicationHealth::AtRisk => "Cannot recover if crash",
            ReplicationHealth::Initializing => "Setting up persistence",
        }
    }
}

/// Write-blocking manager tracks replication health and enforces write rules.
///
/// This manager:
/// - Tracks per-chunk replication status
/// - Computes current write-blocking state
/// - Maps state to replication health for display
///
/// # Example
///
/// ```ignore
/// let mut manager = WriteBlockingManager::new();
///
/// // Initial state: no chunks, PROVISIONAL
/// assert_eq!(manager.current_state(), WriteBlockingState::Provisional);
///
/// // Add chunk status after distribution
/// manager.update_chunk_status(0, 3); // Chunk 0: fully replicated
/// manager.update_chunk_status(1, 3); // Chunk 1: fully replicated
///
/// // Network has peers available
/// manager.set_network_size(5);
///
/// // State should be ACTIVE (all chunks good, peers available)
/// assert_eq!(manager.current_state(), WriteBlockingState::Active);
/// assert!(manager.allows_writes());
/// ```
#[derive(Debug, Clone)]
pub struct WriteBlockingManager {
    /// Per-chunk replication status
    chunk_status: Vec<ChunkReplicationStatus>,

    /// Current network size (number of registered bots)
    network_size: usize,

    /// Last computed state (cached for efficiency)
    cached_state: WriteBlockingState,
}

impl WriteBlockingManager {
    /// Create a new write-blocking manager.
    ///
    /// Initial state is PROVISIONAL (no chunks, no network info).
    pub fn new() -> Self {
        Self {
            chunk_status: Vec::new(),
            network_size: 0,
            cached_state: WriteBlockingState::Provisional,
        }
    }

    /// Update network size from registry.
    ///
    /// This affects state computation (ISOLATED vs PROVISIONAL vs DEGRADED).
    ///
    /// # Arguments
    ///
    /// * `size` - Number of registered bots in persistence network
    pub fn set_network_size(&mut self, size: usize) {
        self.network_size = size;
        self.recompute_state();
    }

    /// Update replication status for a specific chunk.
    ///
    /// Called after chunk distribution to record confirmed replicas.
    ///
    /// # Arguments
    ///
    /// * `chunk_index` - Chunk index (0-based)
    /// * `confirmed_replicas` - Number of confirmed replicas (1-3)
    pub fn update_chunk_status(&mut self, chunk_index: u32, confirmed_replicas: u8) {
        // Find existing entry or create new one
        if let Some(status) = self
            .chunk_status
            .iter_mut()
            .find(|s| s.chunk_index == chunk_index)
        {
            status.confirmed_replicas = confirmed_replicas;
        } else {
            self.chunk_status
                .push(ChunkReplicationStatus::new(chunk_index, confirmed_replicas));
        }

        self.recompute_state();
    }

    /// Initialize chunk tracking for a new state distribution.
    ///
    /// Clears previous chunk status and prepares for new distribution.
    ///
    /// # Arguments
    ///
    /// * `num_chunks` - Number of chunks in new state
    pub fn initialize_chunks(&mut self, num_chunks: u32) {
        self.chunk_status.clear();
        for i in 0..num_chunks {
            self.chunk_status
                .push(ChunkReplicationStatus::new(i, 0));
        }
        self.recompute_state();
    }

    /// Get current write-blocking state.
    ///
    /// # Returns
    ///
    /// Current state (cached, recomputed on updates)
    pub fn current_state(&self) -> WriteBlockingState {
        self.cached_state
    }

    /// Check if writes are currently allowed.
    ///
    /// # Returns
    ///
    /// `true` if writes allowed, `false` if blocked
    pub fn allows_writes(&self) -> bool {
        self.cached_state.allows_writes()
    }

    /// Compute replication health for user display.
    ///
    /// # Returns
    ///
    /// Health metric based on current chunk status
    pub fn replication_health(&self) -> ReplicationHealth {
        // Map state to health
        match self.cached_state {
            WriteBlockingState::Active => {
                // Check if fully replicated or partial
                if self.chunk_status.iter().all(|s| s.is_fully_replicated()) {
                    ReplicationHealth::Replicated
                } else {
                    ReplicationHealth::Partial
                }
            }
            WriteBlockingState::Degraded => ReplicationHealth::AtRisk,
            WriteBlockingState::Provisional | WriteBlockingState::Isolated => {
                ReplicationHealth::Initializing
            }
        }
    }

    /// Get chunks that are at risk (≤1 replica).
    ///
    /// # Returns
    ///
    /// Vector of chunk indices that are at risk
    pub fn at_risk_chunks(&self) -> Vec<u32> {
        self.chunk_status
            .iter()
            .filter(|s| s.is_at_risk())
            .map(|s| s.chunk_index)
            .collect()
    }

    /// Get replication statistics for display.
    ///
    /// # Returns
    ///
    /// (total_chunks, fully_replicated, recoverable, at_risk)
    pub fn replication_stats(&self) -> (usize, usize, usize, usize) {
        let total = self.chunk_status.len();
        let fully_replicated = self
            .chunk_status
            .iter()
            .filter(|s| s.is_fully_replicated())
            .count();
        let recoverable = self
            .chunk_status
            .iter()
            .filter(|s| s.is_recoverable())
            .count();
        let at_risk = self.chunk_status.iter().filter(|s| s.is_at_risk()).count();

        (total, fully_replicated, recoverable, at_risk)
    }

    /// Recompute current state based on chunk status and network size.
    ///
    /// State transition logic:
    /// 1. If N=1: ISOLATED (single bot)
    /// 2. If N=0 or no chunks: PROVISIONAL (initializing)
    /// 3. If any chunk at risk AND peers available: DEGRADED (blocked)
    /// 4. If all chunks recoverable (2+ replicas): ACTIVE (allowed)
    /// 5. Otherwise: PROVISIONAL (insufficient replication but no peers)
    fn recompute_state(&mut self) {
        // No chunks yet
        if self.chunk_status.is_empty() {
            self.cached_state = WriteBlockingState::Provisional;
            return;
        }

        // Network size 1 = ISOLATED
        if self.network_size == 1 {
            self.cached_state = WriteBlockingState::Isolated;
            return;
        }

        // Network size 0 = PROVISIONAL (no peers available)
        if self.network_size == 0 {
            self.cached_state = WriteBlockingState::Provisional;
            return;
        }

        // Check if any chunks are at risk
        let has_at_risk_chunks = self.chunk_status.iter().any(|s| s.is_at_risk());

        if has_at_risk_chunks {
            // Peers available but chunks at risk = DEGRADED (blocked)
            self.cached_state = WriteBlockingState::Degraded;
        } else {
            // All chunks recoverable = ACTIVE (allowed)
            self.cached_state = WriteBlockingState::Active;
        }
    }
}

impl Default for WriteBlockingManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_provisional() {
        let manager = WriteBlockingManager::new();
        assert_eq!(manager.current_state(), WriteBlockingState::Provisional);
        assert!(manager.allows_writes());
    }

    #[test]
    fn test_isolated_state_with_n1() {
        let mut manager = WriteBlockingManager::new();
        manager.set_network_size(1);
        manager.initialize_chunks(3);

        assert_eq!(manager.current_state(), WriteBlockingState::Isolated);
        assert!(manager.allows_writes());
    }

    #[test]
    fn test_active_state_with_good_replication() {
        let mut manager = WriteBlockingManager::new();
        manager.set_network_size(5);
        manager.initialize_chunks(2);

        // All chunks have 2+ replicas
        manager.update_chunk_status(0, 3);
        manager.update_chunk_status(1, 2);

        assert_eq!(manager.current_state(), WriteBlockingState::Active);
        assert!(manager.allows_writes());
    }

    #[test]
    fn test_degraded_state_blocks_writes() {
        let mut manager = WriteBlockingManager::new();
        manager.set_network_size(5); // Peers available
        manager.initialize_chunks(2);

        // One chunk at risk, one good
        manager.update_chunk_status(0, 3);
        manager.update_chunk_status(1, 1); // At risk!

        assert_eq!(manager.current_state(), WriteBlockingState::Degraded);
        assert!(!manager.allows_writes());
    }

    #[test]
    fn test_provisional_allows_writes_when_no_peers() {
        let mut manager = WriteBlockingManager::new();
        manager.set_network_size(0); // No peers
        manager.initialize_chunks(2);

        // Chunks at risk but no peers available
        manager.update_chunk_status(0, 1);
        manager.update_chunk_status(1, 1);

        assert_eq!(manager.current_state(), WriteBlockingState::Provisional);
        assert!(manager.allows_writes()); // Not penalized for network scarcity
    }

    #[test]
    fn test_replication_health_replicated() {
        let mut manager = WriteBlockingManager::new();
        manager.set_network_size(5);
        manager.initialize_chunks(2);

        manager.update_chunk_status(0, 3);
        manager.update_chunk_status(1, 3);

        assert_eq!(manager.replication_health(), ReplicationHealth::Replicated);
    }

    #[test]
    fn test_replication_health_partial() {
        let mut manager = WriteBlockingManager::new();
        manager.set_network_size(5);
        manager.initialize_chunks(2);

        manager.update_chunk_status(0, 3); // Full
        manager.update_chunk_status(1, 2); // Degraded but recoverable

        assert_eq!(manager.replication_health(), ReplicationHealth::Partial);
    }

    #[test]
    fn test_replication_health_at_risk() {
        let mut manager = WriteBlockingManager::new();
        manager.set_network_size(5);
        manager.initialize_chunks(2);

        manager.update_chunk_status(0, 3);
        manager.update_chunk_status(1, 1); // At risk!

        assert_eq!(manager.replication_health(), ReplicationHealth::AtRisk);
    }

    #[test]
    fn test_at_risk_chunks_detection() {
        let mut manager = WriteBlockingManager::new();
        manager.set_network_size(5);
        manager.initialize_chunks(4);

        manager.update_chunk_status(0, 3);
        manager.update_chunk_status(1, 1); // At risk
        manager.update_chunk_status(2, 2);
        manager.update_chunk_status(3, 0); // At risk

        let at_risk = manager.at_risk_chunks();
        assert_eq!(at_risk.len(), 2);
        assert!(at_risk.contains(&1));
        assert!(at_risk.contains(&3));
    }

    #[test]
    fn test_replication_stats() {
        let mut manager = WriteBlockingManager::new();
        manager.set_network_size(5);
        manager.initialize_chunks(4);

        manager.update_chunk_status(0, 3); // Fully replicated
        manager.update_chunk_status(1, 2); // Recoverable
        manager.update_chunk_status(2, 3); // Fully replicated
        manager.update_chunk_status(3, 1); // At risk

        let (total, fully, recoverable, at_risk) = manager.replication_stats();
        assert_eq!(total, 4);
        assert_eq!(fully, 2);
        assert_eq!(recoverable, 3); // 0, 1, 2
        assert_eq!(at_risk, 1); // 3
    }

    #[test]
    fn test_state_requires_warning() {
        assert!(!WriteBlockingState::Active.requires_warning());
        assert!(WriteBlockingState::Provisional.requires_warning());
        assert!(WriteBlockingState::Degraded.requires_warning());
        assert!(WriteBlockingState::Isolated.requires_warning());
    }

    #[test]
    fn test_chunk_status_predicates() {
        let at_risk = ChunkReplicationStatus::new(0, 1);
        let recoverable = ChunkReplicationStatus::new(1, 2);
        let full = ChunkReplicationStatus::new(2, 3);

        assert!(at_risk.is_at_risk());
        assert!(!at_risk.is_recoverable());
        assert!(!at_risk.is_fully_replicated());

        assert!(!recoverable.is_at_risk());
        assert!(recoverable.is_recoverable());
        assert!(!recoverable.is_fully_replicated());

        assert!(!full.is_at_risk());
        assert!(full.is_recoverable());
        assert!(full.is_fully_replicated());
    }
}
