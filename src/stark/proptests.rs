//! Property-based tests for STARK vouch verification
//!
//! Tests for:
//! - Completeness: Valid claims produce verifying proofs
//! - Soundness: Invalid claims cannot produce verifying proofs
//! - Determinism: Same input produces same proof

use super::{
    prove_vouch_claim,
    types::{MemberHash, VouchClaim},
    verify_vouch_proof,
};
use proptest::prelude::*;
use std::collections::BTreeSet;

// Helper to generate test member hashes
fn test_member(id: u8) -> MemberHash {
    let mut bytes = [0u8; 32];
    bytes[0] = id;
    MemberHash(bytes)
}

proptest! {
    /// Property test: Completeness
    /// For all valid vouch claims, proof generation succeeds and verification passes
    #[test]
    fn prop_completeness(
        member_id in 0u8..100,
        voucher_start in 100u8..150,
        voucher_count in 0usize..20,
        flagger_start in 150u8..200,
        flagger_count in 0usize..20,
    ) {
        let member = test_member(member_id);
        let vouchers: BTreeSet<_> = (voucher_start..voucher_start.saturating_add(voucher_count as u8))
            .map(test_member)
            .collect();
        let flaggers: BTreeSet<_> = (flagger_start..flagger_start.saturating_add(flagger_count as u8))
            .map(test_member)
            .collect();

        let claim = VouchClaim::new(member, vouchers, flaggers);

        // Proof generation should succeed
        let proof_result = prove_vouch_claim(&claim);
        prop_assert!(proof_result.is_ok(), "Proof generation failed for valid claim");

        let proof = proof_result.unwrap();

        // Proof should verify
        let verify_result = verify_vouch_proof(&proof);
        prop_assert!(verify_result.is_ok(), "Verification failed for valid proof");

        // Proof size should be under limit
        prop_assert!(proof.size() < 100_000, "Proof size exceeds 100KB limit");
    }

    /// Property test: Soundness (tampered claims)
    /// Claims with tampered standing values should fail verification
    #[test]
    fn prop_soundness_tampered_standing(
        member_id in 0u8..100,
        voucher_count in 1usize..10,
        flagger_count in 0usize..10,
        tampered_standing in -100i32..100i32,
    ) {
        let member = test_member(member_id);
        let vouchers: BTreeSet<_> = (100..100 + voucher_count as u8)
            .map(test_member)
            .collect();
        let flaggers: BTreeSet<_> = (150..150 + flagger_count as u8)
            .map(test_member)
            .collect();

        let mut claim = VouchClaim::new(member, vouchers, flaggers);
        let original_standing = claim.standing;

        // Skip if tampered value equals original (not actually tampered)
        if tampered_standing == original_standing {
            return Ok(());
        }

        // Tamper with standing
        claim.standing = tampered_standing;

        // Verification should fail for inconsistent claim
        let verify_result = claim.verify_consistency();
        prop_assert!(verify_result.is_err(), "Tampered claim should fail consistency check");
    }

    /// Property test: Determinism
    /// Same vouch claim should produce same proof bytes (with same trace)
    #[test]
    fn prop_determinism(
        member_id in 0u8..100,
        voucher_count in 1usize..10,
        flagger_count in 0usize..10,
    ) {
        let member = test_member(member_id);
        let vouchers: BTreeSet<_> = (100..100 + voucher_count as u8)
            .map(test_member)
            .collect();
        let flaggers: BTreeSet<_> = (150..150 + flagger_count as u8)
            .map(test_member)
            .collect();

        let claim = VouchClaim::new(member, vouchers.clone(), flaggers.clone());

        // Generate proof twice
        let proof1 = prove_vouch_claim(&claim).unwrap();
        let proof2 = prove_vouch_claim(&claim).unwrap();

        // Proofs should be identical
        prop_assert_eq!(proof1.proof_bytes, proof2.proof_bytes,
            "Same claim should produce identical proofs");
    }

    /// Property test: Standing calculation correctness
    /// Standing = Effective_Vouches - Regular_Flags
    #[test]
    fn prop_standing_calculation(
        member_id in 0u8..100,
        voucher_count in 0usize..15,
        flagger_count in 0usize..15,
    ) {
        let member = test_member(member_id);
        let vouchers: BTreeSet<_> = (100..100 + voucher_count as u8)
            .map(test_member)
            .collect();
        let flaggers: BTreeSet<_> = (150..150 + flagger_count as u8)
            .map(test_member)
            .collect();

        let claim = VouchClaim::new(member, vouchers.clone(), flaggers.clone());

        // Calculate expected values manually
        let voucher_flaggers: BTreeSet<_> = vouchers.intersection(&flaggers).copied().collect();
        let expected_effective = vouchers.len() - voucher_flaggers.len();
        let expected_regular = flaggers.len() - voucher_flaggers.len();
        let expected_standing = expected_effective as i32 - expected_regular as i32;

        // Verify claim calculations
        prop_assert_eq!(claim.effective_vouches, expected_effective);
        prop_assert_eq!(claim.regular_flags, expected_regular);
        prop_assert_eq!(claim.standing, expected_standing);
    }
}
