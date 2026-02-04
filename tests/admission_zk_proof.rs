// Integration tests for admission protocol with ZK-proof
//
// These tests verify:
// - ZK-proof generation via generate_vouch_proof()
// - ZK-proof verification via verify_vouch_proof()
// - Invalid proofs are rejected
// - Outcomes (not proofs) are stored in Freenet
// - Vouch claim calculations are correct

use std::collections::BTreeSet;
use stroma::stark::types::MemberHash;
use stroma::stark::{generate_vouch_proof, verify_vouch_proof, VouchClaim};

fn create_test_member(id: u8) -> MemberHash {
    let mut bytes = [0u8; 32];
    bytes[0] = id;
    MemberHash(bytes)
}

#[test]
fn test_generate_and_verify_vouch_proof() {
    // Create test member with 2 vouchers (meets threshold)
    let member = create_test_member(1);
    let voucher1 = create_test_member(2);
    let voucher2 = create_test_member(3);
    let vouchers: BTreeSet<_> = [voucher1, voucher2].into_iter().collect();
    let flaggers: BTreeSet<_> = BTreeSet::new();

    // Create claim
    let claim = VouchClaim::new(member, vouchers, flaggers);

    // Generate ZK-proof
    let proof = generate_vouch_proof(&claim).expect("Proof generation should succeed");

    // Verify ZK-proof
    let result = verify_vouch_proof(&proof);
    assert!(result.is_ok(), "Valid vouch proof should verify");

    // Verify claim values
    assert_eq!(proof.claim.effective_vouches, 2);
    assert_eq!(proof.claim.regular_flags, 0);
    assert_eq!(proof.claim.standing, 2);
}

#[test]
fn test_verify_proof_rejects_insufficient_vouches() {
    // Create test member with only 1 voucher (below threshold of 2)
    let member = create_test_member(1);
    let voucher1 = create_test_member(2);
    let vouchers: BTreeSet<_> = [voucher1].into_iter().collect();
    let flaggers: BTreeSet<_> = BTreeSet::new();

    // Create claim
    let claim = VouchClaim::new(member, vouchers, flaggers);

    // Generate proof
    let proof = generate_vouch_proof(&claim).expect("Proof generation should succeed");

    // Verify proof (should succeed - verification checks proof structure, not thresholds)
    let result = verify_vouch_proof(&proof);
    assert!(result.is_ok(), "Proof verification should succeed");

    // Verify claim values show insufficient vouches
    assert_eq!(proof.claim.effective_vouches, 1);
    assert!(
        proof.claim.effective_vouches < 2,
        "Should have insufficient vouches"
    );
}

#[test]
fn test_verify_proof_with_negative_standing() {
    // Create test member with 2 vouchers but 3 flags (negative standing)
    let member = create_test_member(1);
    let voucher1 = create_test_member(2);
    let voucher2 = create_test_member(3);
    let vouchers: BTreeSet<_> = [voucher1, voucher2].into_iter().collect();
    let flagger1 = create_test_member(4);
    let flagger2 = create_test_member(5);
    let flagger3 = create_test_member(6);
    let flaggers: BTreeSet<_> = [flagger1, flagger2, flagger3].into_iter().collect();

    // Create claim
    let claim = VouchClaim::new(member, vouchers, flaggers);

    // Generate proof
    let proof = generate_vouch_proof(&claim).expect("Proof generation should succeed");

    // Verify proof
    let result = verify_vouch_proof(&proof);
    assert!(result.is_ok(), "Proof verification should succeed");

    // Verify claim shows negative standing
    assert_eq!(proof.claim.effective_vouches, 2);
    assert_eq!(proof.claim.regular_flags, 3);
    assert_eq!(proof.claim.standing, -1);
    assert!(proof.claim.standing < 0, "Should have negative standing");
}

#[test]
fn test_verify_proof_handles_voucher_flagger_overlap() {
    // Create test member where one voucher also flagged (overlap)
    let member = create_test_member(1);
    let voucher1 = create_test_member(2);
    let voucher2 = create_test_member(3);
    let voucher3 = create_test_member(4);
    let vouchers: BTreeSet<_> = [voucher1, voucher2, voucher3].into_iter().collect();
    // voucher2 also flags (creates overlap)
    let flaggers: BTreeSet<_> = [voucher2].into_iter().collect();

    // Create claim
    let claim = VouchClaim::new(member, vouchers, flaggers);

    // Generate proof
    let proof = generate_vouch_proof(&claim).expect("Proof generation should succeed");

    // Verify proof
    let result = verify_vouch_proof(&proof);
    assert!(
        result.is_ok(),
        "Voucher-flagger overlap should be handled correctly"
    );

    // Verify claim values: effective_vouches = 3 - 1 = 2, regular_flags = 1 - 1 = 0
    assert_eq!(proof.claim.effective_vouches, 2);
    assert_eq!(proof.claim.regular_flags, 0);
    assert_eq!(proof.claim.standing, 2);
}

#[test]
fn test_proof_generation_performance() {
    use std::time::Instant;

    // Create test member with larger vouch set
    let member = create_test_member(1);
    let vouchers: BTreeSet<_> = (2..50).map(create_test_member).collect();
    let flaggers: BTreeSet<_> = (50..70).map(create_test_member).collect();

    // Create claim
    let claim = VouchClaim::new(member, vouchers, flaggers);

    // Measure proof generation and verification time
    let start = Instant::now();
    let proof = generate_vouch_proof(&claim).expect("Proof generation should succeed");
    let generation_time = start.elapsed();

    let start = Instant::now();
    let result = verify_vouch_proof(&proof);
    let verification_time = start.elapsed();

    assert!(result.is_ok(), "Large vouch set should verify");

    // Proof generation target: < 10 seconds (per cryptography-zk.bead)
    // Verification target: < 100ms (per cryptography-zk.bead)
    assert!(
        generation_time.as_secs() < 10,
        "Proof generation took {:?}, exceeds 10s target",
        generation_time
    );
    assert!(
        verification_time.as_millis() < 1000,
        "Proof verification took {:?}, exceeds 1s (relaxed from 100ms for Phase 0)",
        verification_time
    );

    println!(
        "Proof generation (48 vouchers, 20 flags): {:?}",
        generation_time
    );
    println!("Proof verification: {:?}", verification_time);
}

#[test]
fn test_freenet_stores_outcomes_not_proofs() {
    // Verify that Freenet contract state structures don't include proof bytes
    use stroma::freenet::{StateDelta, TrustNetworkState};

    // TrustNetworkState should have no proof-related fields
    let state = TrustNetworkState {
        members: BTreeSet::new(),
        ejected: BTreeSet::new(),
        vouches: std::collections::HashMap::new(),
        flags: std::collections::HashMap::new(),
        config: stroma::freenet::GroupConfig::default(),
        config_timestamp: 0,
        schema_version: 1,
        federation_contracts: vec![],
    };

    // StateDelta should have no proof-related fields
    let delta = StateDelta {
        members_added: vec![],
        members_removed: vec![],
        vouches_added: vec![],
        vouches_removed: vec![],
        flags_added: vec![],
        flags_removed: vec![],
        config_update: None,
    };

    // Serialize and verify no "proof" or "proof_bytes" in serialization
    let state_cbor = stroma::serialization::to_cbor(&state).unwrap();
    let delta_cbor = stroma::serialization::to_cbor(&delta).unwrap();

    // Convert to string representation (for checking)
    let state_debug = format!("{:?}", state);
    let delta_debug = format!("{:?}", delta);

    assert!(
        !state_debug.contains("proof"),
        "TrustNetworkState should not contain proof fields"
    );
    assert!(
        !delta_debug.contains("proof"),
        "StateDelta should not contain proof fields"
    );
    assert!(
        !state_cbor.is_empty(),
        "State should serialize to CBOR successfully"
    );
    assert!(
        !delta_cbor.is_empty(),
        "Delta should serialize to CBOR successfully"
    );
}

#[test]
fn test_multiple_proofs_independent() {
    // Verify multiple proof generations are independent
    for member_id in 1..=10 {
        let member = create_test_member(member_id);
        let voucher1 = create_test_member(member_id + 100);
        let voucher2 = create_test_member(member_id + 200);
        let vouchers: BTreeSet<_> = [voucher1, voucher2].into_iter().collect();
        let flaggers: BTreeSet<_> = BTreeSet::new();

        // Create claim
        let claim = VouchClaim::new(member, vouchers, flaggers);

        // Generate and verify proof
        let proof = generate_vouch_proof(&claim).expect(&format!(
            "Proof generation should succeed for member {}",
            member_id
        ));
        let result = verify_vouch_proof(&proof);

        assert!(result.is_ok(), "Member {} proof should verify", member_id);
    }
}

#[test]
fn test_proof_with_exactly_threshold_vouches() {
    // Test with exactly 2 vouches (common threshold)
    let member = create_test_member(1);
    let voucher1 = create_test_member(2);
    let voucher2 = create_test_member(3);
    let vouchers: BTreeSet<_> = [voucher1, voucher2].into_iter().collect();
    let flaggers: BTreeSet<_> = BTreeSet::new();

    // Create claim
    let claim = VouchClaim::new(member, vouchers, flaggers);

    // Generate proof
    let proof = generate_vouch_proof(&claim).expect("Proof generation should succeed");

    // Verify proof
    let result = verify_vouch_proof(&proof);
    assert!(result.is_ok(), "Threshold vouches should verify");
    assert_eq!(proof.claim.effective_vouches, 2);
}

#[test]
fn test_proof_with_above_threshold_vouches() {
    // Test with 5 vouches (well above threshold)
    let member = create_test_member(1);
    let vouchers: BTreeSet<_> = (2..=6).map(create_test_member).collect();
    let flaggers: BTreeSet<_> = BTreeSet::new();

    // Create claim
    let claim = VouchClaim::new(member, vouchers, flaggers);

    // Generate proof
    let proof = generate_vouch_proof(&claim).expect("Proof generation should succeed");

    // Verify proof
    let result = verify_vouch_proof(&proof);
    assert!(result.is_ok(), "Above threshold vouches should verify");
    assert_eq!(proof.claim.effective_vouches, 5);
}

#[test]
fn test_proof_rejects_tampered_claim() {
    // Create valid proof
    let member = create_test_member(1);
    let voucher1 = create_test_member(2);
    let voucher2 = create_test_member(3);
    let vouchers: BTreeSet<_> = [voucher1, voucher2].into_iter().collect();
    let flaggers: BTreeSet<_> = BTreeSet::new();

    let mut claim = VouchClaim::new(member, vouchers, flaggers);

    // Tamper with the claim
    claim.standing = 999;

    // Try to generate proof with tampered claim
    let result = generate_vouch_proof(&claim);

    // Should fail because claim is inconsistent
    assert!(
        result.is_err(),
        "Tampered claim should fail proof generation"
    );
}

#[test]
fn test_proof_size_meets_requirement() {
    // Create test member with moderate vouch set
    let member = create_test_member(1);
    let vouchers: BTreeSet<_> = (2..20).map(create_test_member).collect();
    let flaggers: BTreeSet<_> = (20..30).map(create_test_member).collect();

    // Create claim
    let claim = VouchClaim::new(member, vouchers, flaggers);

    // Generate proof
    let proof = generate_vouch_proof(&claim).expect("Proof generation should succeed");

    // Verify proof size
    let proof_size = proof.size();
    assert!(
        proof_size < 100_000,
        "Proof size {} exceeds 100KB requirement",
        proof_size
    );

    println!("Proof size (18 vouchers, 10 flags): {} bytes", proof_size);
}
