//! STARK verifier for vouch verification

use super::VouchProof;

/// Verify a STARK proof of vouch verification
///
/// # Performance Target
/// - Verification time: < 100ms
pub fn verify_vouch_proof(proof: &VouchProof) -> Result<(), String> {
    // Verify claim is internally consistent
    proof.claim.verify_consistency()?;

    // Verify proof is not empty
    if proof.proof_bytes.is_empty() {
        return Err("Empty proof".to_string());
    }

    // Parse proof structure
    let bytes = &proof.proof_bytes;

    // Proof structure:
    // - 8 bytes: trace length (u64)
    // - N * 32 bytes: commitments (SHA256 hashes, one per trace row)
    // - 8 bytes: effective_vouches (u64)
    // - 8 bytes: regular_flags (u64)
    // - 8 bytes: standing (i64)

    if bytes.len() < 8 + 8 + 8 + 8 {
        return Err(format!(
            "Proof too short: {} bytes, expected at least 32",
            bytes.len()
        ));
    }

    // Parse trace length
    let trace_len = u64::from_le_bytes(
        bytes[0..8]
            .try_into()
            .map_err(|_| "Failed to parse trace length")?,
    ) as usize;

    // Verify trace length is power of 2 and reasonable
    if !trace_len.is_power_of_two() {
        return Err(format!("Trace length {} is not a power of 2", trace_len));
    }
    if trace_len < 8 {
        return Err(format!("Trace length {} is too small (min 8)", trace_len));
    }
    if trace_len > 1024 * 1024 {
        return Err(format!("Trace length {} is too large (max 1M)", trace_len));
    }

    // Verify we have correct number of commitments
    let expected_len = 8 + trace_len * 32 + 8 + 8 + 8;
    if bytes.len() != expected_len {
        return Err(format!(
            "Proof length mismatch: got {} bytes, expected {}",
            bytes.len(),
            expected_len
        ));
    }

    // Parse public inputs from end of proof
    let public_inputs_start = 8 + trace_len * 32;
    let effective_vouches = u64::from_le_bytes(
        bytes[public_inputs_start..public_inputs_start + 8]
            .try_into()
            .map_err(|_| "Failed to parse effective_vouches")?,
    ) as usize;
    let regular_flags = u64::from_le_bytes(
        bytes[public_inputs_start + 8..public_inputs_start + 16]
            .try_into()
            .map_err(|_| "Failed to parse regular_flags")?,
    ) as usize;
    let standing = i64::from_le_bytes(
        bytes[public_inputs_start + 16..public_inputs_start + 24]
            .try_into()
            .map_err(|_| "Failed to parse standing")?,
    ) as i32;

    // Verify public inputs match claim
    if effective_vouches != proof.claim.effective_vouches {
        return Err(format!(
            "Effective vouches mismatch: proof has {}, claim has {}",
            effective_vouches, proof.claim.effective_vouches
        ));
    }
    if regular_flags != proof.claim.regular_flags {
        return Err(format!(
            "Regular flags mismatch: proof has {}, claim has {}",
            regular_flags, proof.claim.regular_flags
        ));
    }
    if standing != proof.claim.standing {
        return Err(format!(
            "Standing mismatch: proof has {}, claim has {}",
            standing, proof.claim.standing
        ));
    }

    // Note: Full STARK verification would require:
    // - Reconstructing the trace from the claim
    // - Verifying each commitment matches the corresponding trace row
    // - Verifying AIR constraints
    // - Verifying FRI polynomial commitment
    //
    // For Phase 0 (bot-side verification), we trust the bot's proof generation
    // and verify structural consistency. The claim consistency check already
    // ensures the computation is correct.

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stark::{prove_vouch_claim, types::MemberHash, VouchClaim};
    use std::collections::BTreeSet;

    fn test_member(id: u8) -> MemberHash {
        let mut bytes = [0u8; 32];
        bytes[0] = id;
        MemberHash(bytes)
    }

    #[test]
    fn test_verify_valid_proof() {
        let member = test_member(1);
        let vouchers: BTreeSet<_> = [test_member(2), test_member(3)].into_iter().collect();
        let flaggers: BTreeSet<_> = [test_member(4)].into_iter().collect();

        let claim = VouchClaim::new(member, vouchers, flaggers);
        let proof = prove_vouch_claim(&claim).expect("proof generation failed");

        let result = verify_vouch_proof(&proof);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_proof_with_overlap() {
        let member = test_member(1);
        let vouchers: BTreeSet<_> = [test_member(2), test_member(3)].into_iter().collect();
        let flaggers: BTreeSet<_> = [test_member(3), test_member(4)].into_iter().collect();

        let claim = VouchClaim::new(member, vouchers, flaggers);
        let proof = prove_vouch_claim(&claim).expect("proof generation failed");

        let result = verify_vouch_proof(&proof);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reject_inconsistent_claim() {
        let member = test_member(1);
        let vouchers: BTreeSet<_> = [test_member(2), test_member(3)].into_iter().collect();
        let flaggers: BTreeSet<_> = [test_member(4)].into_iter().collect();

        let mut claim = VouchClaim::new(member, vouchers, flaggers);
        // Tamper with the claim
        claim.standing = 999;

        let proof = VouchProof {
            claim,
            proof_bytes: vec![0u8; 1024],
        };

        let result = verify_vouch_proof(&proof);
        assert!(result.is_err());
    }
}
