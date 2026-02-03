//! STARK verifier for vouch verification

use super::VouchProof;

/// Verify a STARK proof of vouch verification
///
/// # Performance Target
/// - Verification time: < 100ms
pub fn verify_vouch_proof(proof: &VouchProof) -> Result<(), String> {
    // Verify claim is internally consistent
    proof.claim.verify_consistency()?;

    // TODO: Implement actual STARK proof verification
    // This is a placeholder for task #5
    if proof.proof_bytes.is_empty() {
        return Err("Empty proof".to_string());
    }

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
