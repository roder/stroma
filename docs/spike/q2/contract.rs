//! Q2 Spike: Contract Validation
//!
//! Tests whether Freenet contracts can reject invalid state transitions.
//! This determines if Stroma can be trustless (contract enforces) or hybrid (bot validates).
//!
//! Key API methods tested:
//! - `validate_state()` - Can return ValidateResult::Invalid to reject state
//! - `update_state()` - Can return Err(ContractError::InvalidUpdate) to reject delta
//!
//! Invariant under test: Every active member must have >= 2 vouches

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

/// Member state with vouch tracking for validation testing.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemberState {
    /// Currently active members
    pub active: BTreeSet<String>,
    /// Tombstones for removed members (grow-only)
    pub removed: BTreeSet<String>,
    /// Vouch graph: member -> set of vouchers
    pub vouches: HashMap<String, BTreeSet<String>>,
    /// Version counter
    pub version: u64,
}

/// Delta representing a state change.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemberDelta {
    /// Members to add (with their vouchers)
    pub additions: Vec<MemberAddition>,
    /// Members to remove
    pub removals: Vec<String>,
    /// New version
    pub new_version: u64,
}

/// A member addition with voucher information.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemberAddition {
    pub member: String,
    pub vouchers: Vec<String>,
}

/// Result of validation - mirrors Freenet's ValidateResult
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    Valid,
    Invalid(String),
}

/// Error type - mirrors Freenet's ContractError
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // Some variants intentionally unused in this spike
pub enum ContractError {
    InvalidUpdate(String),
    InvalidDelta(String),
    InvalidState(String),
    Other(String),
}

impl MemberState {
    pub fn new() -> Self {
        Self {
            active: BTreeSet::new(),
            removed: BTreeSet::new(),
            vouches: HashMap::new(),
            version: 0,
        }
    }

    /// Create initial state with seed members (bootstrap - they vouch for each other)
    pub fn with_seed_members(members: Vec<&str>) -> Self {
        let mut state = Self::new();

        // Add all seed members
        for member in &members {
            state.active.insert(member.to_string());
        }

        // Create triangle vouching (each member vouches for all others)
        for member in &members {
            let vouchers: BTreeSet<String> = members
                .iter()
                .filter(|v| *v != member)
                .map(|v| v.to_string())
                .collect();
            state.vouches.insert(member.to_string(), vouchers);
        }

        state.version = 1;
        state
    }

    /// Count vouches for a member (only from active members)
    pub fn vouch_count(&self, member: &str) -> usize {
        self.vouches
            .get(member)
            .map(|vouchers| vouchers.iter().filter(|v| self.active.contains(*v)).count())
            .unwrap_or(0)
    }

    /// Validate state invariants - mirrors Freenet's validate_state()
    ///
    /// INVARIANT: Every active member must have >= 2 vouches from active members
    /// (except during bootstrap with < 3 members)
    pub fn validate_state(&self) -> ValidationResult {
        // Skip validation for bootstrap (< 3 members)
        if self.active.len() < 3 {
            return ValidationResult::Valid;
        }

        // Check invariant: every member needs >= 2 vouches
        for member in &self.active {
            let vouch_count = self.vouch_count(member);
            if vouch_count < 2 {
                return ValidationResult::Invalid(format!(
                    "Member {} has only {} vouches (need >= 2)",
                    member, vouch_count
                ));
            }
        }

        // Check: no member in both active and removed
        for member in &self.active {
            if self.removed.contains(member) {
                return ValidationResult::Invalid(format!(
                    "Member {} in both active and removed sets",
                    member
                ));
            }
        }

        ValidationResult::Valid
    }

    /// Update state with delta - mirrors Freenet's update_state()
    ///
    /// Can reject invalid deltas by returning Err(ContractError::InvalidUpdate)
    pub fn update_state(&mut self, delta: &MemberDelta) -> Result<(), ContractError> {
        // === PRE-VALIDATION: Check if delta would create invalid state ===

        // Check additions: each new member needs >= 2 vouches from active members
        for addition in &delta.additions {
            // Skip if member is tombstoned
            if self.removed.contains(&addition.member) {
                return Err(ContractError::InvalidUpdate(format!(
                    "Cannot add tombstoned member: {}",
                    addition.member
                )));
            }

            // Count vouches from currently active members
            let valid_vouches: Vec<_> = addition
                .vouchers
                .iter()
                .filter(|v| self.active.contains(*v))
                .collect();

            if valid_vouches.len() < 2 {
                return Err(ContractError::InvalidUpdate(format!(
                    "Member {} has only {} valid vouches (need >= 2). Vouchers: {:?}, Active: {:?}",
                    addition.member,
                    valid_vouches.len(),
                    addition.vouchers,
                    self.active
                )));
            }
        }

        // === APPLY DELTA (order matters for commutativity) ===

        // 1. Apply removals first (tombstone)
        for member in &delta.removals {
            self.active.remove(member);
            self.removed.insert(member.clone());
            // Note: We keep vouches in the graph for historical reference
            // but vouch_count() only counts active vouchers
        }

        // 2. Apply additions
        for addition in &delta.additions {
            if !self.removed.contains(&addition.member) {
                self.active.insert(addition.member.clone());
                self.vouches.insert(
                    addition.member.clone(),
                    addition.vouchers.iter().cloned().collect(),
                );
            }
        }

        // Update version
        self.version = self.version.max(delta.new_version);

        // === POST-VALIDATION: Verify resulting state is valid ===
        match self.validate_state() {
            ValidationResult::Valid => Ok(()),
            ValidationResult::Invalid(reason) => {
                // Note: In a real implementation, we'd rollback here
                // For the spike, we'll just report the validation failure
                Err(ContractError::InvalidState(format!(
                    "Delta created invalid state: {}",
                    reason
                )))
            }
        }
    }

    /// Apply delta without validation (for testing commutativity)
    pub fn apply_delta_unchecked(&mut self, delta: &MemberDelta) {
        // Apply removals first
        for member in &delta.removals {
            self.active.remove(member);
            self.removed.insert(member.clone());
        }

        // Apply additions (only if not tombstoned)
        for addition in &delta.additions {
            if !self.removed.contains(&addition.member) {
                self.active.insert(addition.member.clone());
                self.vouches.insert(
                    addition.member.clone(),
                    addition.vouchers.iter().cloned().collect(),
                );
            }
        }

        self.version = self.version.max(delta.new_version);
    }
}

impl Default for MemberState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_members_valid() {
        let state = MemberState::with_seed_members(vec!["Alice", "Bob", "Carol"]);

        // Each member should have 2 vouches (from the other two)
        assert_eq!(state.vouch_count("Alice"), 2);
        assert_eq!(state.vouch_count("Bob"), 2);
        assert_eq!(state.vouch_count("Carol"), 2);

        // State should be valid
        assert_eq!(state.validate_state(), ValidationResult::Valid);
    }

    #[test]
    fn test_valid_addition_accepted() {
        let mut state = MemberState::with_seed_members(vec!["Alice", "Bob", "Carol"]);

        // Add Dave with 2 vouches from active members
        let delta = MemberDelta {
            additions: vec![MemberAddition {
                member: "Dave".to_string(),
                vouchers: vec!["Alice".to_string(), "Bob".to_string()],
            }],
            removals: vec![],
            new_version: 2,
        };

        // Should succeed
        let result = state.update_state(&delta);
        assert!(result.is_ok(), "Valid addition should be accepted");
        assert!(state.active.contains("Dave"));
    }

    #[test]
    fn test_invalid_addition_rejected() {
        let mut state = MemberState::with_seed_members(vec!["Alice", "Bob", "Carol"]);

        // Try to add Dave with only 1 vouch
        let delta = MemberDelta {
            additions: vec![MemberAddition {
                member: "Dave".to_string(),
                vouchers: vec!["Alice".to_string()], // Only 1 vouch!
            }],
            removals: vec![],
            new_version: 2,
        };

        // Should be rejected
        let result = state.update_state(&delta);
        assert!(result.is_err(), "Invalid addition should be rejected");
        assert!(!state.active.contains("Dave"));

        // Check error type
        match result {
            Err(ContractError::InvalidUpdate(msg)) => {
                assert!(msg.contains("only 1 valid vouches"));
            }
            _ => panic!("Expected InvalidUpdate error"),
        }
    }

    #[test]
    fn test_tombstoned_member_rejected() {
        let mut state = MemberState::with_seed_members(vec!["Alice", "Bob", "Carol"]);

        // Remove Alice
        let remove_delta = MemberDelta {
            additions: vec![],
            removals: vec!["Alice".to_string()],
            new_version: 2,
        };
        state.apply_delta_unchecked(&remove_delta);

        // Try to re-add Alice
        let readd_delta = MemberDelta {
            additions: vec![MemberAddition {
                member: "Alice".to_string(),
                vouchers: vec!["Bob".to_string(), "Carol".to_string()],
            }],
            removals: vec![],
            new_version: 3,
        };

        // Should be rejected (tombstoned)
        let result = state.update_state(&readd_delta);
        assert!(
            result.is_err(),
            "Tombstoned member re-add should be rejected"
        );

        match result {
            Err(ContractError::InvalidUpdate(msg)) => {
                assert!(msg.contains("tombstoned"));
            }
            _ => panic!("Expected InvalidUpdate error about tombstone"),
        }
    }

    #[test]
    fn test_vouch_invalidation_scenario() {
        let mut state = MemberState::with_seed_members(vec!["Alice", "Bob", "Carol"]);

        // Add Dave with vouches from Alice and Bob
        let add_dave = MemberDelta {
            additions: vec![MemberAddition {
                member: "Dave".to_string(),
                vouchers: vec!["Alice".to_string(), "Bob".to_string()],
            }],
            removals: vec![],
            new_version: 2,
        };
        assert!(state.update_state(&add_dave).is_ok());

        // Now remove Alice (Dave's voucher)
        let remove_alice = MemberDelta {
            additions: vec![],
            removals: vec!["Alice".to_string()],
            new_version: 3,
        };

        // This should succeed (removal is allowed)
        // But it DOES affect Dave's vouch count
        state.apply_delta_unchecked(&remove_alice);

        // Dave now has only 1 active voucher (Bob)
        assert_eq!(state.vouch_count("Dave"), 1);

        // State validation should FAIL
        let validation = state.validate_state();
        assert!(
            matches!(validation, ValidationResult::Invalid(_)),
            "State with < 2 vouches should be invalid"
        );
    }

    #[test]
    fn test_validate_state_catches_invalid() {
        let mut state = MemberState::new();
        state.active.insert("Alice".to_string());
        state.active.insert("Bob".to_string());
        state.active.insert("Carol".to_string());

        // Alice has no vouches
        state.vouches.insert(
            "Bob".to_string(),
            ["Alice", "Carol"].iter().map(|s| s.to_string()).collect(),
        );
        state.vouches.insert(
            "Carol".to_string(),
            ["Alice", "Bob"].iter().map(|s| s.to_string()).collect(),
        );
        // Alice missing vouches!

        let validation = state.validate_state();
        match validation {
            ValidationResult::Invalid(msg) => {
                assert!(msg.contains("Alice"));
                assert!(msg.contains("0 vouches"));
            }
            ValidationResult::Valid => panic!("Should have detected invalid state"),
        }
    }
}
