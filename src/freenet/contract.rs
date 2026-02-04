//! Freenet contract implementation for Stroma trust state.
//!
//! Per freenet-integration.bead:
//! - Two-layer architecture (trust state + persistence)
//! - ComposableState with set-based deltas (Q1 validated)
//! - Small deltas (~100-500 bytes), infrequent updates

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};

/// Trust contract state.
///
/// Per persistence-model.bead Layer 1:
/// - Storage: BTreeSet (members), HashMap (vouches, flags)
/// - Sync: Native Freenet ComposableState
/// - Updates: Small deltas (~100-500 bytes), infrequent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustContract {
    /// Contract version for schema evolution.
    pub version: u32,
    /// Member identities (masked HMACs).
    pub members: BTreeSet<MemberHash>,
    /// Vouches: voucher -> vouchee mappings.
    pub vouches: HashMap<MemberHash, BTreeSet<MemberHash>>,
    /// Flags: flagger -> flagged mappings.
    pub flags: HashMap<MemberHash, BTreeSet<MemberHash>>,
}

/// Member identity hash (HMAC-masked).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MemberHash([u8; 32]);

impl MemberHash {
    /// Create from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes[..32]);
        Self(hash)
    }

    /// Get bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Compute HMAC-masked identity.
    pub fn from_identity(identity: &str, pepper: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(identity.as_bytes());
        hasher.update(pepper);
        let hash_bytes = hasher.finalize();
        Self::from_bytes(&hash_bytes)
    }
}

/// Contract delta (state update).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustDelta {
    /// Add a member.
    AddMember { member: MemberHash },
    /// Remove a member.
    RemoveMember { member: MemberHash },
    /// Add a vouch.
    AddVouch {
        voucher: MemberHash,
        vouchee: MemberHash,
    },
    /// Remove a vouch.
    RemoveVouch {
        voucher: MemberHash,
        vouchee: MemberHash,
    },
    /// Add a flag.
    AddFlag {
        flagger: MemberHash,
        flagged: MemberHash,
    },
    /// Remove a flag.
    RemoveFlag {
        flagger: MemberHash,
        flagged: MemberHash,
    },
}

impl TrustContract {
    /// Create new empty contract.
    pub fn new() -> Self {
        Self {
            version: 1,
            members: BTreeSet::new(),
            vouches: HashMap::new(),
            flags: HashMap::new(),
        }
    }

    /// Apply a delta to the contract state.
    ///
    /// Per persistence-model.bead: "Set-based deltas (Q1 validated)"
    pub fn apply_delta(&mut self, delta: &TrustDelta) {
        match delta {
            TrustDelta::AddMember { member } => {
                self.members.insert(*member);
            }
            TrustDelta::RemoveMember { member } => {
                self.members.remove(member);
                // Clean up associated vouches and flags
                self.vouches.remove(member);
                self.flags.remove(member);
            }
            TrustDelta::AddVouch { voucher, vouchee } => {
                self.vouches.entry(*voucher).or_default().insert(*vouchee);
            }
            TrustDelta::RemoveVouch { voucher, vouchee } => {
                if let Some(vouchees) = self.vouches.get_mut(voucher) {
                    vouchees.remove(vouchee);
                    if vouchees.is_empty() {
                        self.vouches.remove(voucher);
                    }
                }
            }
            TrustDelta::AddFlag { flagger, flagged } => {
                self.flags.entry(*flagger).or_default().insert(*flagged);
            }
            TrustDelta::RemoveFlag { flagger, flagged } => {
                if let Some(flagged_members) = self.flags.get_mut(flagger) {
                    flagged_members.remove(flagged);
                    if flagged_members.is_empty() {
                        self.flags.remove(flagger);
                    }
                }
            }
        }
    }

    /// Merge two contract states (commutative).
    ///
    /// Per freenet-integration.bead: "Native Freenet ComposableState"
    pub fn merge(&mut self, other: &TrustContract) {
        // Merge members (set union)
        self.members.extend(other.members.iter().copied());

        // Merge vouches
        for (voucher, vouchees) in &other.vouches {
            self.vouches
                .entry(*voucher)
                .or_default()
                .extend(vouchees.iter().copied());
        }

        // Merge flags
        for (flagger, flagged_members) in &other.flags {
            self.flags
                .entry(*flagger)
                .or_default()
                .extend(flagged_members.iter().copied());
        }
    }

    /// Get all members.
    pub fn members(&self) -> &BTreeSet<MemberHash> {
        &self.members
    }

    /// Get vouches for a member.
    pub fn vouches_for(&self, member: &MemberHash) -> Vec<MemberHash> {
        self.vouches
            .iter()
            .filter_map(|(voucher, vouchees)| {
                if vouchees.contains(member) {
                    Some(*voucher)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get flags for a member.
    pub fn flags_for(&self, member: &MemberHash) -> Vec<MemberHash> {
        self.flags
            .iter()
            .filter_map(|(flagger, flagged_members)| {
                if flagged_members.contains(member) {
                    Some(*flagger)
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Default for TrustContract {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_member(id: u8) -> MemberHash {
        MemberHash::from_bytes(&[id; 32])
    }

    #[test]
    fn test_new_contract() {
        let contract = TrustContract::new();
        assert_eq!(contract.version, 1);
        assert!(contract.members.is_empty());
        assert!(contract.vouches.is_empty());
        assert!(contract.flags.is_empty());
    }

    #[test]
    fn test_add_member() {
        let mut contract = TrustContract::new();
        let member = test_member(1);

        contract.apply_delta(&TrustDelta::AddMember { member });
        assert!(contract.members.contains(&member));
    }

    #[test]
    fn test_remove_member() {
        let mut contract = TrustContract::new();
        let member = test_member(1);

        contract.apply_delta(&TrustDelta::AddMember { member });
        contract.apply_delta(&TrustDelta::RemoveMember { member });
        assert!(!contract.members.contains(&member));
    }

    #[test]
    fn test_add_vouch() {
        let mut contract = TrustContract::new();
        let voucher = test_member(1);
        let vouchee = test_member(2);

        contract.apply_delta(&TrustDelta::AddVouch { voucher, vouchee });
        assert_eq!(contract.vouches_for(&vouchee), vec![voucher]);
    }

    #[test]
    fn test_remove_vouch() {
        let mut contract = TrustContract::new();
        let voucher = test_member(1);
        let vouchee = test_member(2);

        contract.apply_delta(&TrustDelta::AddVouch { voucher, vouchee });
        contract.apply_delta(&TrustDelta::RemoveVouch { voucher, vouchee });
        assert!(contract.vouches_for(&vouchee).is_empty());
    }

    #[test]
    fn test_add_flag() {
        let mut contract = TrustContract::new();
        let flagger = test_member(1);
        let flagged = test_member(2);

        contract.apply_delta(&TrustDelta::AddFlag { flagger, flagged });
        assert_eq!(contract.flags_for(&flagged), vec![flagger]);
    }

    #[test]
    fn test_remove_flag() {
        let mut contract = TrustContract::new();
        let flagger = test_member(1);
        let flagged = test_member(2);

        contract.apply_delta(&TrustDelta::AddFlag { flagger, flagged });
        contract.apply_delta(&TrustDelta::RemoveFlag { flagger, flagged });
        assert!(contract.flags_for(&flagged).is_empty());
    }

    #[test]
    fn test_member_hash_from_identity() {
        let hash1 = MemberHash::from_identity("alice", b"pepper");
        let hash2 = MemberHash::from_identity("alice", b"pepper");
        let hash3 = MemberHash::from_identity("bob", b"pepper");

        // Same identity + pepper = same hash
        assert_eq!(hash1, hash2);
        // Different identity = different hash
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_member_hash_pepper_changes_output() {
        let hash1 = MemberHash::from_identity("alice", b"pepper1");
        let hash2 = MemberHash::from_identity("alice", b"pepper2");

        // Different pepper = different hash
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_merge_members() {
        let mut contract1 = TrustContract::new();
        let mut contract2 = TrustContract::new();

        let member1 = test_member(1);
        let member2 = test_member(2);

        contract1.apply_delta(&TrustDelta::AddMember { member: member1 });
        contract2.apply_delta(&TrustDelta::AddMember { member: member2 });

        contract1.merge(&contract2);

        assert!(contract1.members.contains(&member1));
        assert!(contract1.members.contains(&member2));
    }

    #[test]
    fn test_merge_vouches() {
        let mut contract1 = TrustContract::new();
        let mut contract2 = TrustContract::new();

        let voucher1 = test_member(1);
        let voucher2 = test_member(2);
        let vouchee = test_member(3);

        contract1.apply_delta(&TrustDelta::AddVouch {
            voucher: voucher1,
            vouchee,
        });
        contract2.apply_delta(&TrustDelta::AddVouch {
            voucher: voucher2,
            vouchee,
        });

        contract1.merge(&contract2);

        let vouchers = contract1.vouches_for(&vouchee);
        assert_eq!(vouchers.len(), 2);
        assert!(vouchers.contains(&voucher1));
        assert!(vouchers.contains(&voucher2));
    }

    #[test]
    fn test_merge_flags() {
        let mut contract1 = TrustContract::new();
        let mut contract2 = TrustContract::new();

        let flagger1 = test_member(1);
        let flagger2 = test_member(2);
        let flagged = test_member(3);

        contract1.apply_delta(&TrustDelta::AddFlag {
            flagger: flagger1,
            flagged,
        });
        contract2.apply_delta(&TrustDelta::AddFlag {
            flagger: flagger2,
            flagged,
        });

        contract1.merge(&contract2);

        let flaggers = contract1.flags_for(&flagged);
        assert_eq!(flaggers.len(), 2);
        assert!(flaggers.contains(&flagger1));
        assert!(flaggers.contains(&flagger2));
    }

    #[test]
    fn test_merge_is_commutative() {
        let mut contract_a = TrustContract::new();
        let mut contract_b = TrustContract::new();

        let member1 = test_member(1);
        let member2 = test_member(2);

        contract_a.apply_delta(&TrustDelta::AddMember { member: member1 });
        contract_b.apply_delta(&TrustDelta::AddMember { member: member2 });

        // Merge A + B
        let mut result_ab = contract_a.clone();
        result_ab.merge(&contract_b);

        // Merge B + A
        let mut result_ba = contract_b.clone();
        result_ba.merge(&contract_a);

        // Results should be identical
        assert_eq!(result_ab.members, result_ba.members);
    }

    #[test]
    fn test_remove_member_cleans_vouches() {
        let mut contract = TrustContract::new();
        let member = test_member(1);
        let vouchee = test_member(2);

        // Member vouches for vouchee
        contract.apply_delta(&TrustDelta::AddMember { member });
        contract.apply_delta(&TrustDelta::AddVouch {
            voucher: member,
            vouchee,
        });

        // Remove member
        contract.apply_delta(&TrustDelta::RemoveMember { member });

        // Vouch should be cleaned up
        assert!(!contract.vouches.contains_key(&member));
    }

    #[test]
    fn test_remove_member_cleans_flags() {
        let mut contract = TrustContract::new();
        let member = test_member(1);
        let flagged = test_member(2);

        // Member flags another
        contract.apply_delta(&TrustDelta::AddMember { member });
        contract.apply_delta(&TrustDelta::AddFlag {
            flagger: member,
            flagged,
        });

        // Remove member
        contract.apply_delta(&TrustDelta::RemoveMember { member });

        // Flag should be cleaned up
        assert!(!contract.flags.contains_key(&member));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut contract = TrustContract::new();
        let member = test_member(1);
        contract.apply_delta(&TrustDelta::AddMember { member });

        // Serialize
        let serialized = serde_json::to_string(&contract).unwrap();

        // Deserialize
        let deserialized: TrustContract = serde_json::from_str(&serialized).unwrap();

        assert_eq!(contract, deserialized);
    }

    #[test]
    fn test_delta_serialization_roundtrip() {
        let delta = TrustDelta::AddMember {
            member: test_member(1),
        };

        let serialized = serde_json::to_string(&delta).unwrap();
        let deserialized: TrustDelta = serde_json::from_str(&serialized).unwrap();

        assert_eq!(delta, deserialized);
    }

    #[test]
    fn test_multiple_vouchers_for_same_vouchee() {
        let mut contract = TrustContract::new();
        let voucher1 = test_member(1);
        let voucher2 = test_member(2);
        let vouchee = test_member(3);

        contract.apply_delta(&TrustDelta::AddVouch {
            voucher: voucher1,
            vouchee,
        });
        contract.apply_delta(&TrustDelta::AddVouch {
            voucher: voucher2,
            vouchee,
        });

        let vouchers = contract.vouches_for(&vouchee);
        assert_eq!(vouchers.len(), 2);
    }

    #[test]
    fn test_multiple_flaggers_for_same_flagged() {
        let mut contract = TrustContract::new();
        let flagger1 = test_member(1);
        let flagger2 = test_member(2);
        let flagged = test_member(3);

        contract.apply_delta(&TrustDelta::AddFlag {
            flagger: flagger1,
            flagged,
        });
        contract.apply_delta(&TrustDelta::AddFlag {
            flagger: flagger2,
            flagged,
        });

        let flaggers = contract.flags_for(&flagged);
        assert_eq!(flaggers.len(), 2);
    }
}
