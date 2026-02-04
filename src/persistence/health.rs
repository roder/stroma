//! Replication Health Tracking
//!
//! This module tracks the replication health of persistence chunks across the network.
//! Health is measured at write time via attestations, NOT via heartbeat polling.
//!
//! ## Health States
//!
//! - **Replicated**: All chunks have 2+ remote replicas (🟢 Resilient)
//! - **Partial**: Some chunks degraded but recoverable (🟡 Degraded)
//! - **AtRisk**: Any chunk has 0 replicas (🔴 Cannot recover)
//! - **Initializing**: No suitable peers or N=1 network (🔵 Bootstrap)
//!
//! ## Write-Blocking
//!
//! Writes are **blocked** when:
//! - Health status is `AtRisk` AND suitable peers are available (DEGRADED state)
//!
//! Writes are **allowed** when:
//! - Health status is `Replicated` or `Partial` (ACTIVE state)
//! - No suitable peers available (PROVISIONAL state)
//! - N=1 network (ISOLATED state, warned)
//!
//! ## References
//!
//! - Persistence: docs/PERSISTENCE.md § Replication Health Metric
//! - Agent: Agent-Freenet

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Health status of the replication network.
///
/// Determines whether writes should be blocked and what risk level exists
/// for recovery after a crash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// All chunks have 2+ remote replicas confirmed (🟢)
    ///
    /// - Full resilience
    /// - Writes allowed
    /// - Recovery confidence: High
    Replicated,

    /// Some chunks have 1 replica, but all chunks have at least 1 (🟡)
    ///
    /// - Degraded resilience
    /// - Writes allowed
    /// - Recovery confidence: Medium
    Partial,

    /// Any chunk has 0 replicas (🔴)
    ///
    /// - Cannot recover if crash now
    /// - Writes **BLOCKED** if peers available
    /// - Recovery confidence: Low
    AtRisk,

    /// No suitable peers available or N=1 network (🔵)
    ///
    /// - Bootstrap or isolated state
    /// - Writes allowed (with warning for N=1)
    /// - Recovery confidence: None (expected during bootstrap)
    Initializing,
}

impl HealthStatus {
    /// Get emoji indicator for this status
    pub fn emoji(&self) -> &'static str {
        match self {
            HealthStatus::Replicated => "🟢",
            HealthStatus::Partial => "🟡",
            HealthStatus::AtRisk => "🔴",
            HealthStatus::Initializing => "🔵",
        }
    }

    /// Get human-readable status text
    pub fn as_str(&self) -> &'static str {
        match self {
            HealthStatus::Replicated => "Replicated",
            HealthStatus::Partial => "Partial",
            HealthStatus::AtRisk => "At Risk",
            HealthStatus::Initializing => "Initializing",
        }
    }
}

/// Replication health tracker for persistence chunks.
///
/// Tracks attestations (confirmations) from chunk holders that they possess
/// the chunks assigned to them. Health is measured at write time, not via
/// heartbeat polling.
///
/// # Example
///
/// ```ignore
/// let mut health = ReplicationHealth::new();
///
/// // Record chunk distribution attestations
/// health.record_attestation(0, "holder-bot-1", true);
/// health.record_attestation(0, "holder-bot-2", true);
///
/// // Check replication health
/// assert_eq!(health.status(), HealthStatus::Initializing);
/// assert!(health.can_write(1)); // N=1, writes allowed with warning
///
/// // After distributing all chunks
/// health.update_total_chunks(8);
/// for i in 0..8 {
///     health.record_attestation(i, "holder-1", true);
///     health.record_attestation(i, "holder-2", true);
/// }
///
/// assert_eq!(health.status(), HealthStatus::Replicated);
/// assert_eq!(health.ratio(), 1.0); // 100% fully replicated
/// assert!(health.can_write(5)); // Writes allowed, N>=5
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationHealth {
    /// Per-chunk attestations tracking.
    ///
    /// Maps chunk_index -> (holder_id -> attestation_status)
    /// attestation_status: true = confirmed, false = failed/missing
    chunk_attestations: HashMap<u32, HashMap<String, bool>>,

    /// Total number of chunks in current state.
    ///
    /// Used to compute replication ratio. Updated on each state change.
    total_chunks: u32,

    /// Cached health status (recomputed when attestations change).
    cached_status: HealthStatus,

    /// Network size at last health check (for write-blocking decisions).
    ///
    /// Used to distinguish between:
    /// - N=1 (ISOLATED - writes allowed)
    /// - N>=2 with degraded health (DEGRADED - writes blocked)
    last_network_size: usize,
}

impl ReplicationHealth {
    /// Create a new replication health tracker.
    pub fn new() -> Self {
        Self {
            chunk_attestations: HashMap::new(),
            total_chunks: 0,
            cached_status: HealthStatus::Initializing,
            last_network_size: 0,
        }
    }

    /// Record an attestation from a chunk holder.
    ///
    /// Called when a chunk distribution succeeds or a challenge-response
    /// verifies that a holder still possesses the chunk.
    ///
    /// # Arguments
    ///
    /// * `chunk_index` - Which chunk (0-based)
    /// * `holder_id` - Identity of the bot holding this chunk
    /// * `confirmed` - true if holder confirmed possession, false if failed
    pub fn record_attestation(&mut self, chunk_index: u32, holder_id: &str, confirmed: bool) {
        self.chunk_attestations
            .entry(chunk_index)
            .or_insert_with(HashMap::new)
            .insert(holder_id.to_string(), confirmed);

        // Recompute cached status
        self.cached_status = self.compute_status();
    }

    /// Remove attestation for a specific holder (e.g., bot unregistered).
    ///
    /// # Arguments
    ///
    /// * `chunk_index` - Which chunk
    /// * `holder_id` - Identity of the bot no longer holding this chunk
    pub fn remove_attestation(&mut self, chunk_index: u32, holder_id: &str) {
        if let Some(holders) = self.chunk_attestations.get_mut(&chunk_index) {
            holders.remove(holder_id);
        }

        // Recompute cached status
        self.cached_status = self.compute_status();
    }

    /// Clear all attestations for a chunk (e.g., chunk redistributed).
    ///
    /// # Arguments
    ///
    /// * `chunk_index` - Which chunk to clear
    pub fn clear_chunk(&mut self, chunk_index: u32) {
        self.chunk_attestations.remove(&chunk_index);
        self.cached_status = self.compute_status();
    }

    /// Clear all attestations (e.g., full state redistribution).
    pub fn clear_all(&mut self) {
        self.chunk_attestations.clear();
        self.cached_status = HealthStatus::Initializing;
    }

    /// Update total number of chunks (called on state change).
    ///
    /// # Arguments
    ///
    /// * `num_chunks` - New chunk count for current state
    pub fn update_total_chunks(&mut self, num_chunks: u32) {
        self.total_chunks = num_chunks;
        self.cached_status = self.compute_status();
    }

    /// Update network size (for write-blocking decisions).
    ///
    /// # Arguments
    ///
    /// * `network_size` - Current number of registered bots (from registry)
    pub fn update_network_size(&mut self, network_size: usize) {
        self.last_network_size = network_size;
    }

    /// Get current replication health status.
    ///
    /// # Returns
    ///
    /// Current health status (Replicated, Partial, AtRisk, or Initializing)
    pub fn status(&self) -> HealthStatus {
        self.cached_status
    }

    /// Check if writes should be allowed based on current health.
    ///
    /// # Write-Blocking Logic
    ///
    /// Writes are **blocked** when:
    /// - Health is `AtRisk` AND network size >= 2 (DEGRADED state)
    ///
    /// Writes are **allowed** when:
    /// - Health is `Replicated` or `Partial` (ACTIVE state)
    /// - Health is `Initializing` (PROVISIONAL state)
    /// - Network size = 1 (ISOLATED state, even if AtRisk)
    ///
    /// # Arguments
    ///
    /// * `network_size` - Current network size (from registry query)
    ///
    /// # Returns
    ///
    /// `true` if writes should be allowed, `false` if blocked
    pub fn can_write(&self, network_size: usize) -> bool {
        match self.cached_status {
            HealthStatus::Replicated | HealthStatus::Partial => true,
            HealthStatus::Initializing => true, // PROVISIONAL or bootstrap
            HealthStatus::AtRisk => {
                // DEGRADED state: block writes if peers available
                // ISOLATED state: allow writes with warning if N=1
                network_size <= 1
            }
        }
    }

    /// Compute replication ratio (chunks with 2+ replicas / total chunks).
    ///
    /// # Formula
    ///
    /// ```text
    /// ratio = chunks_with_2_plus_replicas / total_chunks
    /// ```
    ///
    /// # Returns
    ///
    /// Ratio from 0.0 (no chunks replicated) to 1.0 (all chunks replicated)
    pub fn ratio(&self) -> f64 {
        if self.total_chunks == 0 {
            return 0.0;
        }

        let chunks_with_2_plus = (0..self.total_chunks)
            .filter(|idx| self.confirmed_replicas(*idx) >= 2)
            .count();

        chunks_with_2_plus as f64 / self.total_chunks as f64
    }

    /// Get number of confirmed replicas for a specific chunk.
    ///
    /// # Arguments
    ///
    /// * `chunk_index` - Which chunk to query
    ///
    /// # Returns
    ///
    /// Number of confirmed replicas (holders with attestation = true)
    pub fn confirmed_replicas(&self, chunk_index: u32) -> usize {
        self.chunk_attestations
            .get(&chunk_index)
            .map(|holders| holders.values().filter(|confirmed| **confirmed).count())
            .unwrap_or(0)
    }

    /// Get detailed per-chunk replication status.
    ///
    /// # Returns
    ///
    /// Vector of (chunk_index, confirmed_replicas) tuples
    pub fn chunk_status(&self) -> Vec<(u32, usize)> {
        (0..self.total_chunks)
            .map(|idx| (idx, self.confirmed_replicas(idx)))
            .collect()
    }

    /// Compute health status from current attestations.
    ///
    /// # Logic
    ///
    /// - If no chunks tracked: `Initializing`
    /// - If any chunk has 0 replicas: `AtRisk`
    /// - If all chunks have 2+ replicas: `Replicated`
    /// - Otherwise (some chunks have 1 replica): `Partial`
    fn compute_status(&self) -> HealthStatus {
        if self.total_chunks == 0 {
            return HealthStatus::Initializing;
        }

        let mut all_replicated = true;
        let mut any_at_risk = false;

        for chunk_idx in 0..self.total_chunks {
            let replicas = self.confirmed_replicas(chunk_idx);

            if replicas == 0 {
                any_at_risk = true;
            }

            if replicas < 2 {
                all_replicated = false;
            }
        }

        if any_at_risk {
            HealthStatus::AtRisk
        } else if all_replicated {
            HealthStatus::Replicated
        } else {
            HealthStatus::Partial
        }
    }

    /// Get recovery confidence assessment.
    ///
    /// # Returns
    ///
    /// Human-readable string describing recovery confidence
    pub fn recovery_confidence(&self) -> &'static str {
        match self.cached_status {
            HealthStatus::Replicated => "✅ Yes — all chunks available from multiple holders",
            HealthStatus::Partial => {
                "⚠️ Degraded — some chunks have only 1 replica, but recoverable"
            }
            HealthStatus::AtRisk => "❌ No — some chunks missing, cannot guarantee recovery",
            HealthStatus::Initializing => {
                "🔵 Bootstrap — no replication yet (expected during initialization)"
            }
        }
    }

    /// Get total number of chunks being tracked.
    pub fn total_chunks(&self) -> u32 {
        self.total_chunks
    }

    /// Get last known network size.
    pub fn network_size(&self) -> usize {
        self.last_network_size
    }
}

impl Default for ReplicationHealth {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_health_is_initializing() {
        let health = ReplicationHealth::new();
        assert_eq!(health.status(), HealthStatus::Initializing);
        assert_eq!(health.ratio(), 0.0);
    }

    #[test]
    fn test_fully_replicated_status() {
        let mut health = ReplicationHealth::new();
        health.update_total_chunks(8);

        // Distribute all chunks with 2 replicas each
        for i in 0..8 {
            health.record_attestation(i, "holder-1", true);
            health.record_attestation(i, "holder-2", true);
        }

        assert_eq!(health.status(), HealthStatus::Replicated);
        assert_eq!(health.ratio(), 1.0);
    }

    #[test]
    fn test_partial_status() {
        let mut health = ReplicationHealth::new();
        health.update_total_chunks(8);

        // Some chunks fully replicated
        for i in 0..4 {
            health.record_attestation(i, "holder-1", true);
            health.record_attestation(i, "holder-2", true);
        }

        // Some chunks with only 1 replica
        for i in 4..8 {
            health.record_attestation(i, "holder-1", true);
        }

        assert_eq!(health.status(), HealthStatus::Partial);
        assert_eq!(health.ratio(), 0.5); // 4/8 chunks fully replicated
    }

    #[test]
    fn test_at_risk_status() {
        let mut health = ReplicationHealth::new();
        health.update_total_chunks(8);

        // Most chunks replicated
        for i in 0..7 {
            health.record_attestation(i, "holder-1", true);
            health.record_attestation(i, "holder-2", true);
        }

        // One chunk has NO replicas
        // (chunk 7 has no attestations)

        assert_eq!(health.status(), HealthStatus::AtRisk);
        assert_eq!(health.ratio(), 7.0 / 8.0);
    }

    #[test]
    fn test_write_blocking_degraded_state() {
        let mut health = ReplicationHealth::new();
        health.update_total_chunks(8);

        // At risk with 0 replicas for some chunks
        health.record_attestation(0, "holder-1", true);

        assert_eq!(health.status(), HealthStatus::AtRisk);

        // N >= 2: writes should be blocked (DEGRADED state)
        assert!(!health.can_write(2));
        assert!(!health.can_write(5));
    }

    #[test]
    fn test_write_blocking_isolated_state() {
        let mut health = ReplicationHealth::new();
        health.update_total_chunks(8);

        // At risk status
        assert_eq!(health.status(), HealthStatus::AtRisk);

        // N = 1: writes should be allowed (ISOLATED state)
        assert!(health.can_write(1));
    }

    #[test]
    fn test_write_allowed_active_state() {
        let mut health = ReplicationHealth::new();
        health.update_total_chunks(8);

        // Fully replicated
        for i in 0..8 {
            health.record_attestation(i, "holder-1", true);
            health.record_attestation(i, "holder-2", true);
        }

        assert_eq!(health.status(), HealthStatus::Replicated);
        assert!(health.can_write(5));
    }

    #[test]
    fn test_write_allowed_provisional_state() {
        let health = ReplicationHealth::new();

        // Initializing status
        assert_eq!(health.status(), HealthStatus::Initializing);
        assert!(health.can_write(0)); // PROVISIONAL: no peers available
    }

    #[test]
    fn test_confirmed_replicas() {
        let mut health = ReplicationHealth::new();

        health.record_attestation(0, "holder-1", true);
        health.record_attestation(0, "holder-2", true);
        health.record_attestation(0, "holder-3", false); // Failed

        assert_eq!(health.confirmed_replicas(0), 2);
    }

    #[test]
    fn test_remove_attestation() {
        let mut health = ReplicationHealth::new();
        health.update_total_chunks(1);

        health.record_attestation(0, "holder-1", true);
        health.record_attestation(0, "holder-2", true);
        assert_eq!(health.status(), HealthStatus::Replicated);

        // Remove one holder
        health.remove_attestation(0, "holder-2");
        assert_eq!(health.status(), HealthStatus::Partial);

        // Remove last holder
        health.remove_attestation(0, "holder-1");
        assert_eq!(health.status(), HealthStatus::AtRisk);
    }

    #[test]
    fn test_clear_chunk() {
        let mut health = ReplicationHealth::new();
        health.update_total_chunks(2);

        health.record_attestation(0, "holder-1", true);
        health.record_attestation(0, "holder-2", true);
        health.record_attestation(1, "holder-1", true);
        health.record_attestation(1, "holder-2", true);

        assert_eq!(health.status(), HealthStatus::Replicated);

        // Clear chunk 1
        health.clear_chunk(1);
        assert_eq!(health.confirmed_replicas(1), 0);
        assert_eq!(health.status(), HealthStatus::AtRisk);
    }

    #[test]
    fn test_clear_all() {
        let mut health = ReplicationHealth::new();
        health.update_total_chunks(8);

        for i in 0..8 {
            health.record_attestation(i, "holder-1", true);
            health.record_attestation(i, "holder-2", true);
        }

        assert_eq!(health.status(), HealthStatus::Replicated);

        health.clear_all();
        assert_eq!(health.status(), HealthStatus::Initializing);
    }

    #[test]
    fn test_chunk_status() {
        let mut health = ReplicationHealth::new();
        health.update_total_chunks(3);

        health.record_attestation(0, "holder-1", true);
        health.record_attestation(0, "holder-2", true);
        health.record_attestation(1, "holder-1", true);
        // chunk 2: no attestations

        let status = health.chunk_status();
        assert_eq!(status, vec![(0, 2), (1, 1), (2, 0)]);
    }

    #[test]
    fn test_health_status_emoji() {
        assert_eq!(HealthStatus::Replicated.emoji(), "🟢");
        assert_eq!(HealthStatus::Partial.emoji(), "🟡");
        assert_eq!(HealthStatus::AtRisk.emoji(), "🔴");
        assert_eq!(HealthStatus::Initializing.emoji(), "🔵");
    }

    #[test]
    fn test_recovery_confidence() {
        let mut health = ReplicationHealth::new();

        // Initializing
        assert_eq!(
            health.recovery_confidence(),
            "🔵 Bootstrap — no replication yet (expected during initialization)"
        );

        // Replicated
        health.update_total_chunks(1);
        health.record_attestation(0, "holder-1", true);
        health.record_attestation(0, "holder-2", true);
        assert_eq!(
            health.recovery_confidence(),
            "✅ Yes — all chunks available from multiple holders"
        );
    }
}
