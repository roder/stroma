//! Minimal test contract to demonstrate conflict resolution.
//!
//! This contract uses set-based membership with tombstones for removals,
//! following CRDT patterns for natural mergeability.
//!
//! Key insight from Freenet docs: "Implementations must ensure that state delta
//! updates are commutative. When applying multiple delta updates to a state,
//! the order in which these updates are applied should not affect the final state."

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Simple member set state - uses CRDT-style sets for commutativity.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimpleMemberSet {
    /// Currently active members
    pub active: BTreeSet<String>,
    /// Tombstones for removed members (grow-only, never cleared)
    pub removed: BTreeSet<String>,
    /// Version counter for debugging/tracking
    pub version: u64,
}

/// Delta representing a state change (additions and removals).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimpleMemberSetDelta {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub new_version: u64,
}

#[allow(dead_code)]
impl SimpleMemberSet {
    pub fn new() -> Self {
        Self {
            active: BTreeSet::new(),
            removed: BTreeSet::new(),
            version: 0,
        }
    }

    /// Create a summary of the current state (for delta generation).
    pub fn summarize(&self) -> (BTreeSet<String>, BTreeSet<String>, u64) {
        (self.active.clone(), self.removed.clone(), self.version)
    }

    /// Generate delta from old summary to current state.
    pub fn delta(
        &self,
        old: &(BTreeSet<String>, BTreeSet<String>, u64),
    ) -> Option<SimpleMemberSetDelta> {
        let added: Vec<_> = self.active.difference(&old.0).cloned().collect();
        let removed: Vec<_> = self.removed.difference(&old.1).cloned().collect();

        if added.is_empty() && removed.is_empty() {
            None
        } else {
            Some(SimpleMemberSetDelta {
                added,
                removed,
                new_version: self.version,
            })
        }
    }

    /// Apply a delta to the state.
    ///
    /// CRITICAL: This must be COMMUTATIVE - same result regardless of delta order.
    /// We use "remove-wins" semantics: tombstones block additions.
    pub fn apply_delta(&mut self, delta: &SimpleMemberSetDelta) {
        // First, apply removals (add to tombstone set)
        for member in &delta.removed {
            self.active.remove(member);
            self.removed.insert(member.clone());
        }

        // Then, apply additions (only if not tombstoned)
        // "Remove-wins" semantics: tombstone blocks late additions
        for member in &delta.added {
            if !self.removed.contains(member) {
                self.active.insert(member.clone());
            }
        }

        // Update version (max ensures determinism)
        self.version = self.version.max(delta.new_version);
    }

    /// Verify state invariants.
    /// Returns error message if state is invalid.
    pub fn verify(&self) -> Result<(), String> {
        // Invariant: No member should be in both active and removed sets
        for member in &self.active {
            if self.removed.contains(member) {
                return Err(format!("Member {} in both active and removed", member));
            }
        }
        Ok(())
    }

    /// Helper: Add a member (for building test states)
    pub fn add_member(&mut self, member: String) {
        if !self.removed.contains(&member) {
            self.active.insert(member);
            self.version += 1;
        }
    }

    /// Helper: Remove a member (for building test states)
    pub fn remove_member(&mut self, member: String) {
        self.active.remove(&member);
        self.removed.insert(member);
        self.version += 1;
    }
}

impl Default for SimpleMemberSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commutativity_add_then_remove() {
        let mut initial = SimpleMemberSet::new();
        initial.add_member("A".to_string());
        initial.add_member("B".to_string());

        let delta_add = SimpleMemberSetDelta {
            added: vec!["X".to_string()],
            removed: vec![],
            new_version: 3,
        };

        let delta_remove = SimpleMemberSetDelta {
            added: vec![],
            removed: vec!["A".to_string()],
            new_version: 4,
        };

        // Order 1: Add then Remove
        let mut state1 = initial.clone();
        state1.apply_delta(&delta_add);
        state1.apply_delta(&delta_remove);

        // Order 2: Remove then Add
        let mut state2 = initial.clone();
        state2.apply_delta(&delta_remove);
        state2.apply_delta(&delta_add);

        // States must be equal (commutativity)
        assert_eq!(state1, state2, "Delta application must be commutative");
    }

    #[test]
    fn test_tombstone_blocks_late_add() {
        let mut initial = SimpleMemberSet::new();
        initial.add_member("A".to_string());

        // Remove A
        let delta_remove = SimpleMemberSetDelta {
            added: vec![],
            removed: vec!["A".to_string()],
            new_version: 2,
        };

        // Try to add A back
        let delta_add = SimpleMemberSetDelta {
            added: vec!["A".to_string()],
            removed: vec![],
            new_version: 3,
        };

        // Apply remove then add
        let mut state = initial.clone();
        state.apply_delta(&delta_remove);
        state.apply_delta(&delta_add);

        // A should NOT be in active (tombstone wins)
        assert!(
            !state.active.contains("A"),
            "Tombstone should block late add"
        );
        assert!(state.removed.contains("A"), "A should remain tombstoned");
    }

    #[test]
    fn test_verify_catches_invalid_state() {
        let mut state = SimpleMemberSet::new();
        state.active.insert("A".to_string());
        // Artificially create invalid state (shouldn't happen with apply_delta)
        state.removed.insert("A".to_string());

        assert!(
            state.verify().is_err(),
            "verify() should catch invalid state"
        );
    }
}
