//! Types for vouch verification STARK proofs

use crate::freenet::contract::MemberHash as ContractMemberHash;
use crate::identity::MaskedIdentity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use zeroize::Zeroize;

/// A 32-byte hash representing a member identity
///
/// This type is equivalent to `freenet::contract::MemberHash` and can be
/// converted to/from it via the `From` implementations.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Zeroize,
)]
pub struct MemberHash(pub [u8; 32]);

impl From<ContractMemberHash> for MemberHash {
    fn from(hash: ContractMemberHash) -> Self {
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(hash.as_bytes());
        MemberHash(bytes)
    }
}

impl From<MemberHash> for ContractMemberHash {
    fn from(hash: MemberHash) -> Self {
        ContractMemberHash::from_bytes(&hash.0)
    }
}

impl From<MaskedIdentity> for MemberHash {
    fn from(masked: MaskedIdentity) -> Self {
        MemberHash(*masked.as_bytes())
    }
}

/// A claim about a member's vouch verification state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VouchClaim {
    /// The member whose standing is being verified
    pub member: MemberHash,

    /// Set of members who vouched for this member
    pub vouchers: BTreeSet<MemberHash>,

    /// Set of members who flagged this member
    pub flaggers: BTreeSet<MemberHash>,

    /// Expected effective vouches (vouchers - voucher_flaggers)
    pub effective_vouches: usize,

    /// Expected regular flags (flaggers - voucher_flaggers)
    pub regular_flags: usize,

    /// Expected standing (effective_vouches - regular_flags)
    pub standing: i32,
}

impl VouchClaim {
    /// Create a new vouch claim
    pub fn new(
        member: MemberHash,
        vouchers: BTreeSet<MemberHash>,
        flaggers: BTreeSet<MemberHash>,
    ) -> Self {
        // Calculate voucher-flaggers (intersection)
        let voucher_flaggers: BTreeSet<_> = vouchers.intersection(&flaggers).copied().collect();

        let effective_vouches = vouchers.len() - voucher_flaggers.len();
        let regular_flags = flaggers.len() - voucher_flaggers.len();
        let standing = effective_vouches as i32 - regular_flags as i32;

        Self {
            member,
            vouchers,
            flaggers,
            effective_vouches,
            regular_flags,
            standing,
        }
    }

    /// Verify the claim is internally consistent
    pub fn verify_consistency(&self) -> Result<(), String> {
        // Calculate expected values
        let voucher_flaggers: BTreeSet<_> = self
            .vouchers
            .intersection(&self.flaggers)
            .copied()
            .collect();

        let expected_effective = self.vouchers.len() - voucher_flaggers.len();
        let expected_regular = self.flaggers.len() - voucher_flaggers.len();
        let expected_standing = expected_effective as i32 - expected_regular as i32;

        // Check consistency
        if self.effective_vouches != expected_effective {
            return Err(format!(
                "Effective vouches mismatch: claimed {}, calculated {}",
                self.effective_vouches, expected_effective
            ));
        }

        if self.regular_flags != expected_regular {
            return Err(format!(
                "Regular flags mismatch: claimed {}, calculated {}",
                self.regular_flags, expected_regular
            ));
        }

        if self.standing != expected_standing {
            return Err(format!(
                "Standing mismatch: claimed {}, calculated {}",
                self.standing, expected_standing
            ));
        }

        Ok(())
    }
}

/// A STARK proof of vouch verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VouchProof {
    /// The claim being proven
    pub claim: VouchClaim,

    /// The STARK proof bytes
    pub proof_bytes: Vec<u8>,
}

impl VouchProof {
    /// Get the size of the proof in bytes
    pub fn size(&self) -> usize {
        self.proof_bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_member(id: u8) -> MemberHash {
        let mut bytes = [0u8; 32];
        bytes[0] = id;
        MemberHash(bytes)
    }

    #[test]
    fn test_vouch_claim_no_overlap() {
        let member = test_member(1);
        let vouchers = [test_member(2), test_member(3)].into_iter().collect();
        let flaggers = [test_member(4), test_member(5)].into_iter().collect();

        let claim = VouchClaim::new(member, vouchers, flaggers);

        assert_eq!(claim.effective_vouches, 2);
        assert_eq!(claim.regular_flags, 2);
        assert_eq!(claim.standing, 0);
        assert!(claim.verify_consistency().is_ok());
    }

    #[test]
    fn test_vouch_claim_with_overlap() {
        let member = test_member(1);
        let vouchers = [test_member(2), test_member(3), test_member(4)]
            .into_iter()
            .collect();
        let flaggers = [test_member(3), test_member(5)].into_iter().collect();

        let claim = VouchClaim::new(member, vouchers, flaggers);

        // Member 3 is both voucher and flagger (1 overlap)
        assert_eq!(claim.effective_vouches, 2); // 3 - 1
        assert_eq!(claim.regular_flags, 1); // 2 - 1
        assert_eq!(claim.standing, 1); // 2 - 1
        assert!(claim.verify_consistency().is_ok());
    }

    #[test]
    fn test_vouch_claim_negative_standing() {
        let member = test_member(1);
        let vouchers = [test_member(2)].into_iter().collect();
        let flaggers = [test_member(3), test_member(4), test_member(5)]
            .into_iter()
            .collect();

        let claim = VouchClaim::new(member, vouchers, flaggers);

        assert_eq!(claim.effective_vouches, 1);
        assert_eq!(claim.regular_flags, 3);
        assert_eq!(claim.standing, -2);
        assert!(claim.verify_consistency().is_ok());
    }

    #[test]
    fn test_vouch_claim_all_vouchers_flagged() {
        let member = test_member(1);
        let vouchers = [test_member(2), test_member(3)].into_iter().collect();
        let flaggers = [test_member(2), test_member(3)].into_iter().collect();

        let claim = VouchClaim::new(member, vouchers, flaggers);

        assert_eq!(claim.effective_vouches, 0); // All invalidated
        assert_eq!(claim.regular_flags, 0); // All were vouchers
        assert_eq!(claim.standing, 0);
        assert!(claim.verify_consistency().is_ok());
    }
}
