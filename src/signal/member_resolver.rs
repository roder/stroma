//! Transient MemberHash <-> ServiceId Mapping
//!
//! CRITICAL SECURITY CONSTRAINTS:
//! - Transient only (RAM, NEVER persisted to disk)
//! - Rebuilt on bot startup from Signal group roster
//! - Uses HMAC with mnemonic-derived masking key
//! - NO cleartext Signal IDs stored or logged
//! - Updated on member add/remove from group
//!
//! PURPOSE:
//! Needed to resolve select_validator() MemberHash to ServiceId for sending PMs.
//! The Blind Matchmaker returns a MemberHash, but Signal requires ServiceId to send messages.
//!
//! See: st-yft95, .beads/security-constraints.bead § 1

use crate::freenet::contract::MemberHash;
use crate::identity::mask_identity;
use crate::signal::traits::ServiceId;
use std::collections::HashMap;
use zeroize::Zeroize;

/// Transient bidirectional mapping between MemberHash and ServiceId
///
/// SECURITY:
/// - All mappings are computed using HMAC (same as MemberHash derivation)
/// - Mappings exist only in RAM and are rebuilt on bot startup
/// - ServiceId strings are zeroized when mappings are cleared
/// - Never persisted to disk or logged
#[derive(Debug)]
pub struct MemberResolver {
    /// Identity masking key from StromaKeyring (mnemonic-derived)
    identity_masking_key: [u8; 32],

    /// Forward mapping: MemberHash -> ServiceId
    /// Used to resolve validator selection to Signal PM recipient
    hash_to_id: HashMap<MemberHash, ServiceId>,

    /// Reverse mapping: ServiceId -> MemberHash
    /// Used to look up member hash from incoming Signal messages
    id_to_hash: HashMap<ServiceId, MemberHash>,
}

impl MemberResolver {
    /// Create new resolver with mnemonic-derived identity masking key
    ///
    /// # Arguments
    /// * `identity_masking_key` - Key from `StromaKeyring::identity_masking_key()`
    pub fn new(identity_masking_key: [u8; 32]) -> Self {
        Self {
            identity_masking_key,
            hash_to_id: HashMap::new(),
            id_to_hash: HashMap::new(),
        }
    }

    /// Add member and compute bidirectional mapping
    ///
    /// Computes MemberHash from ServiceId using HMAC with mnemonic-derived key.
    /// Creates both forward and reverse mappings for O(1) lookup in either direction.
    ///
    /// # Arguments
    /// * `service_id` - Signal ServiceId (ACI or PNI)
    ///
    /// # Returns
    /// The computed MemberHash for this ServiceId
    ///
    /// # Security
    /// - Uses same HMAC derivation as identity masking
    /// - Deterministic: same ServiceId always produces same MemberHash
    /// - One-way: cannot reverse MemberHash to ServiceId without this mapping
    pub fn add_member(&mut self, service_id: ServiceId) -> MemberHash {
        // Compute MemberHash using HMAC (same as identity masking)
        let masked = mask_identity(&service_id.0, &self.identity_masking_key);
        let member_hash: MemberHash = masked.into();

        // Store bidirectional mappings
        self.hash_to_id.insert(member_hash, service_id.clone());
        self.id_to_hash.insert(service_id, member_hash);

        member_hash
    }

    /// Remove member and clear mappings
    ///
    /// Removes both forward and reverse mappings for the given ServiceId.
    /// Zeroizes the ServiceId string for security.
    ///
    /// # Arguments
    /// * `service_id` - Signal ServiceId to remove
    ///
    /// # Returns
    /// The MemberHash that was removed, if it existed
    pub fn remove_member(&mut self, service_id: &ServiceId) -> Option<MemberHash> {
        // Look up member hash
        let member_hash = self.id_to_hash.remove(service_id)?;

        // Remove forward mapping
        self.hash_to_id.remove(&member_hash);

        Some(member_hash)
    }

    /// Resolve MemberHash to ServiceId
    ///
    /// Used to convert Blind Matchmaker selection (MemberHash) to Signal PM recipient (ServiceId).
    ///
    /// # Arguments
    /// * `member_hash` - The MemberHash to resolve
    ///
    /// # Returns
    /// The corresponding ServiceId, if member exists in resolver
    pub fn get_service_id(&self, member_hash: &MemberHash) -> Option<&ServiceId> {
        self.hash_to_id.get(member_hash)
    }

    /// Resolve ServiceId to MemberHash
    ///
    /// Used to look up member hash from incoming Signal messages.
    ///
    /// # Arguments
    /// * `service_id` - The ServiceId to resolve
    ///
    /// # Returns
    /// The corresponding MemberHash, if member exists in resolver
    pub fn get_member_hash(&self, service_id: &ServiceId) -> Option<&MemberHash> {
        self.id_to_hash.get(service_id)
    }

    /// Check if member exists in resolver
    pub fn contains_member(&self, service_id: &ServiceId) -> bool {
        self.id_to_hash.contains_key(service_id)
    }

    /// Get count of members in resolver
    pub fn member_count(&self) -> usize {
        self.id_to_hash.len()
    }

    /// Rebuild mappings from Signal group roster
    ///
    /// Called on bot startup to reconstruct the transient resolver state.
    /// Clears existing mappings and rebuilds from the provided roster.
    ///
    /// # Arguments
    /// * `roster` - Vector of ServiceIds from Signal group membership
    ///
    /// # Security
    /// - Old mappings are cleared and zeroized before rebuilding
    /// - Each ServiceId is hashed using HMAC with mnemonic-derived identity masking key
    pub fn rebuild_from_roster(&mut self, roster: Vec<ServiceId>) {
        // Clear existing mappings (zeroizes ServiceId strings)
        self.clear();

        // Rebuild mappings from roster
        for service_id in roster {
            self.add_member(service_id);
        }
    }

    /// Clear all mappings
    ///
    /// Removes all mappings and zeroizes ServiceId strings.
    /// Called before bot shutdown or roster rebuild.
    pub fn clear(&mut self) {
        // Zeroize ServiceId strings before clearing
        for (mut service_id, _) in self.id_to_hash.drain() {
            service_id.0.zeroize();
        }

        self.hash_to_id.clear();
    }
}

impl Drop for MemberResolver {
    /// Ensure all mappings are cleared and zeroized on drop
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_service_id(name: &str) -> ServiceId {
        ServiceId(format!("{}@signal.org", name))
    }

    fn test_masking_key() -> [u8; 32] {
        *b"test-identity-masking-key-32b!!!"
    }

    #[test]
    fn test_add_member() {
        let mut resolver = MemberResolver::new(test_masking_key());

        let alice_id = test_service_id("alice");
        let alice_hash = resolver.add_member(alice_id.clone());

        // Verify bidirectional mapping
        assert_eq!(resolver.get_service_id(&alice_hash), Some(&alice_id));
        assert_eq!(resolver.get_member_hash(&alice_id), Some(&alice_hash));
        assert_eq!(resolver.member_count(), 1);
    }

    #[test]
    fn test_determinism() {
        let mut resolver = MemberResolver::new(test_masking_key());

        let alice_id = test_service_id("alice");
        let hash1 = resolver.add_member(alice_id.clone());

        // Clear and re-add
        resolver.clear();
        let hash2 = resolver.add_member(alice_id.clone());

        // Same ServiceId should produce same MemberHash
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_remove_member() {
        let mut resolver = MemberResolver::new(test_masking_key());

        let alice_id = test_service_id("alice");
        let alice_hash = resolver.add_member(alice_id.clone());

        // Remove member
        let removed_hash = resolver.remove_member(&alice_id);
        assert_eq!(removed_hash, Some(alice_hash));

        // Verify both mappings are cleared
        assert_eq!(resolver.get_service_id(&alice_hash), None);
        assert_eq!(resolver.get_member_hash(&alice_id), None);
        assert_eq!(resolver.member_count(), 0);
    }

    #[test]
    fn test_rebuild_from_roster() {
        let mut resolver = MemberResolver::new(test_masking_key());

        // Initial roster
        let alice_id = test_service_id("alice");
        let bob_id = test_service_id("bob");
        let alice_hash = resolver.add_member(alice_id.clone());
        let bob_hash = resolver.add_member(bob_id.clone());

        assert_eq!(resolver.member_count(), 2);

        // Rebuild with different roster
        let carol_id = test_service_id("carol");
        resolver.rebuild_from_roster(vec![alice_id.clone(), carol_id.clone()]);

        // Verify old mappings cleared
        assert_eq!(resolver.get_service_id(&bob_hash), None);
        assert_eq!(resolver.get_member_hash(&bob_id), None);

        // Verify new mappings exist
        assert_eq!(resolver.get_service_id(&alice_hash), Some(&alice_id));
        assert!(resolver.contains_member(&carol_id));
        assert_eq!(resolver.member_count(), 2);
    }

    #[test]
    fn test_bidirectional_consistency() {
        let mut resolver = MemberResolver::new(test_masking_key());

        let alice_id = test_service_id("alice");
        let bob_id = test_service_id("bob");
        let carol_id = test_service_id("carol");

        let alice_hash = resolver.add_member(alice_id.clone());
        let bob_hash = resolver.add_member(bob_id.clone());
        let carol_hash = resolver.add_member(carol_id.clone());

        // Forward lookups
        assert_eq!(resolver.get_service_id(&alice_hash), Some(&alice_id));
        assert_eq!(resolver.get_service_id(&bob_hash), Some(&bob_id));
        assert_eq!(resolver.get_service_id(&carol_hash), Some(&carol_id));

        // Reverse lookups
        assert_eq!(resolver.get_member_hash(&alice_id), Some(&alice_hash));
        assert_eq!(resolver.get_member_hash(&bob_id), Some(&bob_hash));
        assert_eq!(resolver.get_member_hash(&carol_id), Some(&carol_hash));
    }

    #[test]
    fn test_different_service_ids_produce_different_hashes() {
        let mut resolver = MemberResolver::new(test_masking_key());

        let alice_id = test_service_id("alice");
        let bob_id = test_service_id("bob");

        let alice_hash = resolver.add_member(alice_id);
        let bob_hash = resolver.add_member(bob_id);

        // Different ServiceIds should produce different MemberHashes
        assert_ne!(alice_hash, bob_hash);
    }

    #[test]
    fn test_different_key_produces_different_hashes() {
        let key1 = *b"masking-key-1-32-bytes-padding!!";
        let key2 = *b"masking-key-2-32-bytes-padding!!";

        let mut resolver1 = MemberResolver::new(key1);
        let mut resolver2 = MemberResolver::new(key2);

        let alice_id = test_service_id("alice");

        let hash1 = resolver1.add_member(alice_id.clone());
        let hash2 = resolver2.add_member(alice_id);

        // Same ServiceId with different key should produce different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_clear_removes_all_mappings() {
        let mut resolver = MemberResolver::new(test_masking_key());

        let alice_id = test_service_id("alice");
        let bob_id = test_service_id("bob");

        resolver.add_member(alice_id.clone());
        resolver.add_member(bob_id.clone());

        assert_eq!(resolver.member_count(), 2);

        resolver.clear();

        assert_eq!(resolver.member_count(), 0);
        assert!(!resolver.contains_member(&alice_id));
        assert!(!resolver.contains_member(&bob_id));
    }

    #[test]
    fn test_contains_member() {
        let mut resolver = MemberResolver::new(test_masking_key());

        let alice_id = test_service_id("alice");
        let bob_id = test_service_id("bob");

        assert!(!resolver.contains_member(&alice_id));

        resolver.add_member(alice_id.clone());

        assert!(resolver.contains_member(&alice_id));
        assert!(!resolver.contains_member(&bob_id));
    }
}
