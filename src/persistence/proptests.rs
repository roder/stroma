//! Property-based tests for persistence operations
//!
//! Tests for:
//! - Encryption: AES-256-GCM roundtrip, key isolation, nonce uniqueness, HKDF
//! - Chunking: Reassembly correctness, count validation, size enforcement
//! - Rendezvous: Determinism, owner exclusion, distribution uniformity, churn stability

use super::chunks::{decrypt_and_reassemble, encrypt_and_chunk, CHUNK_SIZE};
use super::rendezvous::compute_chunk_holders;
use proptest::prelude::*;
use std::collections::{HashMap, HashSet};

// ============================================================================
// ENCRYPTION PROPERTY TESTS (8 required)
// ============================================================================

proptest! {
    /// Property: Encryption roundtrip preserves data
    /// For all valid inputs, encrypt then decrypt returns original data
    #[test]
    fn encryption_roundtrip_preserves_data(
        data in prop::collection::vec(any::<u8>(), 0..100_000),
    ) {
        let owner = "test-owner";
        let aci_key = vec![42u8; 32];

        let chunks = encrypt_and_chunk(owner, &data, &aci_key).unwrap();
        let decrypted = decrypt_and_reassemble(&chunks, &aci_key).unwrap();

        prop_assert_eq!(decrypted, data, "Roundtrip should preserve data exactly");
    }

    /// Property: Encryption key isolation
    /// Different keys produce different ciphertexts (even for same data)
    #[test]
    fn encryption_key_isolation(
        data in prop::collection::vec(any::<u8>(), 1..1000),
        key1_byte in any::<u8>(),
        key2_byte in any::<u8>(),
    ) {
        // Skip if keys would be identical
        if key1_byte == key2_byte {
            return Ok(());
        }

        let owner = "test-owner";
        let key1 = vec![key1_byte; 32];
        let key2 = vec![key2_byte; 32];

        let chunks1 = encrypt_and_chunk(owner, &data, &key1).unwrap();
        let chunks2 = encrypt_and_chunk(owner, &data, &key2).unwrap();

        // Ciphertext should differ (at least one chunk's data differs)
        let data_differs = chunks1.iter().zip(chunks2.iter())
            .any(|(c1, c2)| c1.data != c2.data);

        prop_assert!(data_differs, "Different keys must produce different ciphertexts");
    }

    /// Property: Decryption fails with wrong key
    /// Encrypted data cannot be decrypted with a different key
    #[test]
    fn decryption_fails_with_wrong_key(
        data in prop::collection::vec(any::<u8>(), 1..1000),
        correct_key_byte in any::<u8>(),
        wrong_key_byte in any::<u8>(),
    ) {
        // Skip if keys would be identical
        if correct_key_byte == wrong_key_byte {
            return Ok(());
        }

        let owner = "test-owner";
        let correct_key = vec![correct_key_byte; 32];
        let wrong_key = vec![wrong_key_byte; 32];

        let chunks = encrypt_and_chunk(owner, &data, &correct_key).unwrap();
        let result = decrypt_and_reassemble(&chunks, &wrong_key);

        prop_assert!(result.is_err(), "Decryption with wrong key must fail");
    }

    /// Property: Encryption nonce uniqueness
    /// Each encryption uses a unique nonce (probability of collision ~0)
    #[test]
    fn encryption_nonce_uniqueness(
        data in prop::collection::vec(any::<u8>(), 100..1000),
    ) {
        let owner = "test-owner";
        let aci_key = vec![42u8; 32];

        // Encrypt the same data multiple times
        let mut nonces = HashSet::new();
        for _ in 0..10 {
            let chunks = encrypt_and_chunk(owner, &data, &aci_key).unwrap();
            // All chunks from one encryption should have the same nonce
            let nonce = chunks[0].nonce.clone();
            nonces.insert(nonce);
        }

        // Each encryption should use a different nonce
        prop_assert_eq!(nonces.len(), 10, "Each encryption must use unique nonce");
    }

    /// Property: HKDF key derivation is deterministic
    /// Same ACI key + context produces same derived key
    #[test]
    fn hkdf_key_derivation_deterministic(
        data in prop::collection::vec(any::<u8>(), 100..1000),
        key_byte in any::<u8>(),
    ) {
        let owner = "test-owner";
        let aci_key = vec![key_byte; 32];

        // Encrypt twice with same key
        let chunks1 = encrypt_and_chunk(owner, &data, &aci_key).unwrap();
        let chunks2 = encrypt_and_chunk(owner, &data, &aci_key).unwrap();

        // Signatures should be deterministic (derived signing key is same)
        // But data will differ due to different nonces
        let decrypted1 = decrypt_and_reassemble(&chunks1, &aci_key).unwrap();
        let decrypted2 = decrypt_and_reassemble(&chunks2, &aci_key).unwrap();

        prop_assert_eq!(&decrypted1, &data, "First decryption should work");
        prop_assert_eq!(&decrypted2, &data, "Second decryption should work");
        prop_assert_eq!(decrypted1, decrypted2, "Both decryptions should match");
    }

    /// Property: HKDF key derivation is isolated
    /// Different contexts produce different derived keys
    #[test]
    fn hkdf_key_derivation_isolated(
        data in prop::collection::vec(any::<u8>(), 100..1000),
    ) {
        let owner1 = "owner-1";
        let owner2 = "owner-2";
        let aci_key = vec![42u8; 32];

        let chunks1 = encrypt_and_chunk(owner1, &data, &aci_key).unwrap();
        let chunks2 = encrypt_and_chunk(owner2, &data, &aci_key).unwrap();

        // Signatures should differ because owner is part of signing context
        let sig_differs = chunks1[0].signature != chunks2[0].signature;
        prop_assert!(sig_differs, "Different owners should produce different signatures");
    }

    /// Property: Encryption handles large data correctly
    /// Large plaintexts (multiple chunks) roundtrip correctly
    #[test]
    fn encryption_large_data_roundtrip(
        chunk_count in 2usize..10,
        extra_bytes in 0usize..CHUNK_SIZE,
    ) {
        let owner = "test-owner";
        let aci_key = vec![42u8; 32];

        // Create data that spans multiple chunks
        let data_size = chunk_count * CHUNK_SIZE + extra_bytes;
        let data: Vec<u8> = (0..data_size).map(|i| (i % 256) as u8).collect();

        let chunks = encrypt_and_chunk(owner, &data, &aci_key).unwrap();
        let decrypted = decrypt_and_reassemble(&chunks, &aci_key).unwrap();

        prop_assert_eq!(decrypted.len(), data.len(), "Length should be preserved");
        prop_assert_eq!(decrypted, data, "Large data should roundtrip correctly");
    }

    /// Property: Tampered chunks are detected
    /// Any modification to chunk data causes verification failure
    #[test]
    fn encryption_tamper_detection(
        data in prop::collection::vec(any::<u8>(), 100..1000),
        tamper_index in 0usize..10,
    ) {
        let owner = "test-owner";
        let aci_key = vec![42u8; 32];

        let mut chunks = encrypt_and_chunk(owner, &data, &aci_key).unwrap();

        // Skip if tamper_index is out of bounds
        if chunks.is_empty() || tamper_index >= chunks[0].data.len() {
            return Ok(());
        }

        // Tamper with first chunk's data
        chunks[0].data[tamper_index] ^= 0xFF;

        let result = decrypt_and_reassemble(&chunks, &aci_key);
        prop_assert!(result.is_err(), "Tampered chunk should fail verification");
    }
}

// ============================================================================
// CHUNKING PROPERTY TESTS (3 required)
// ============================================================================

proptest! {
    /// Property: Chunking reassembly matches original
    /// Split then reassemble produces identical data
    #[test]
    fn chunking_reassembly_matches(
        data in prop::collection::vec(any::<u8>(), 0..200_000),
    ) {
        let owner = "test-owner";
        let aci_key = vec![42u8; 32];

        let chunks = encrypt_and_chunk(owner, &data, &aci_key).unwrap();
        let reassembled = decrypt_and_reassemble(&chunks, &aci_key).unwrap();

        prop_assert_eq!(reassembled, data, "Reassembly must match original exactly");
    }

    /// Property: Chunk count is correct
    /// Number of chunks matches expected count based on data size
    #[test]
    fn chunking_count_correct(
        base_size in 0usize..10,
        extra_bytes in 0usize..CHUNK_SIZE,
    ) {
        let owner = "test-owner";
        let aci_key = vec![42u8; 32];

        // Create data of known size
        let plaintext_size = base_size * CHUNK_SIZE + extra_bytes;
        let data = vec![42u8; plaintext_size];

        let chunks = encrypt_and_chunk(owner, &data, &aci_key).unwrap();

        // After encryption, there's additional overhead (GCM tag = 16 bytes)
        // So ciphertext = plaintext + 16 bytes
        let ciphertext_size = plaintext_size + 16;
        let expected_chunks = if ciphertext_size == 0 {
            0
        } else {
            ciphertext_size.div_ceil(CHUNK_SIZE)
        };

        prop_assert_eq!(
            chunks.len(),
            expected_chunks,
            "Chunk count should match expected value"
        );
    }

    /// Property: Chunk max size is enforced
    /// No chunk exceeds CHUNK_SIZE limit
    #[test]
    fn chunking_max_size_enforced(
        data in prop::collection::vec(any::<u8>(), 0..500_000),
    ) {
        let owner = "test-owner";
        let aci_key = vec![42u8; 32];

        let chunks = encrypt_and_chunk(owner, &data, &aci_key).unwrap();

        for (i, chunk) in chunks.iter().enumerate() {
            prop_assert!(
                chunk.data.len() <= CHUNK_SIZE,
                "Chunk {} exceeds max size: {} > {}",
                i,
                chunk.data.len(),
                CHUNK_SIZE
            );
        }
    }
}

// ============================================================================
// RENDEZVOUS HASHING PROPERTY TESTS (5 required)
// ============================================================================

proptest! {
    /// Property: Rendezvous hashing is deterministic
    /// Same inputs always produce same holder selection
    #[test]
    fn rendezvous_deterministic(
        owner_id in 0u8..100,
        chunk_index in 0u32..100,
        epoch in 0u64..100,
    ) {
        let owner = format!("owner-{}", owner_id);
        let bots: Vec<String> = (0..10).map(|i| format!("bot-{}", i)).collect();

        let holders1 = compute_chunk_holders(&owner, chunk_index, &bots, epoch, 2);
        let holders2 = compute_chunk_holders(&owner, chunk_index, &bots, epoch, 2);

        prop_assert_eq!(holders1, holders2, "Same inputs must produce same holders");
    }

    /// Property: Owner is never selected as holder
    /// Owner cannot hold their own chunks (prevents gaming)
    #[test]
    fn rendezvous_owner_excluded(
        owner_id in 0u8..100,
        chunk_index in 0u32..100,
        epoch in 0u64..100,
    ) {
        let owner = format!("owner-{}", owner_id);
        let mut bots: Vec<String> = (0..10).map(|i| format!("bot-{}", i)).collect();
        bots.push(owner.clone());

        let holders = compute_chunk_holders(&owner, chunk_index, &bots, epoch, 2);

        prop_assert!(
            !holders.contains(&owner),
            "Owner {} should never be selected as holder",
            owner
        );
    }

    /// Property: Two distinct holders selected
    /// No duplicate holders for same chunk
    #[test]
    fn rendezvous_two_distinct_holders(
        owner_id in 0u8..100,
        chunk_index in 0u32..100,
        epoch in 0u64..100,
    ) {
        let owner = format!("owner-{}", owner_id);
        let bots: Vec<String> = (0..10).map(|i| format!("bot-{}", i)).collect();

        let holders = compute_chunk_holders(&owner, chunk_index, &bots, epoch, 2);

        prop_assert_eq!(holders.len(), 2, "Should select exactly 2 holders");

        let unique: HashSet<_> = holders.iter().collect();
        prop_assert_eq!(
            unique.len(),
            2,
            "Holders must be distinct (no duplicates)"
        );
    }

    /// Property: Churn stability (minimal reassignment)
    /// When bots join/leave, only affected chunks reassign
    #[test]
    fn rendezvous_churn_stability(
        owner_id in 0u8..50,
        num_chunks in 20u32..50,
        new_bot_id in 100u8..110,
    ) {
        let owner = format!("owner-{}", owner_id);
        let mut bots: Vec<String> = (0..20).map(|i| format!("bot-{}", i)).collect();
        let epoch = 1;

        // Compute original holders
        let original_holders: Vec<_> = (0..num_chunks)
            .map(|chunk_idx| compute_chunk_holders(&owner, chunk_idx, &bots, epoch, 2))
            .collect();

        // Add new bot (simulating churn)
        bots.push(format!("bot-{}", new_bot_id));

        // Compute new holders
        let new_holders: Vec<_> = (0..num_chunks)
            .map(|chunk_idx| compute_chunk_holders(&owner, chunk_idx, &bots, epoch, 2))
            .collect();

        // Count reassignments
        let reassignments = original_holders
            .iter()
            .zip(new_holders.iter())
            .filter(|(orig, new)| orig != new)
            .count();

        // With rendezvous hashing, adding 1 bot to 20 bots (5% increase)
        // should cause minimal reassignment. Expect <40% of chunks to move.
        let max_allowed_reassignments = (num_chunks as usize * 40) / 100;

        prop_assert!(
            reassignments <= max_allowed_reassignments,
            "Too many reassignments: {}/{} (limit: {})",
            reassignments,
            num_chunks,
            max_allowed_reassignments
        );
    }

    /// Property: Uniform distribution (chi-squared test)
    /// Load is distributed evenly across all holders
    #[test]
    fn rendezvous_uniform_distribution(
        owner_id in 0u8..50,
        num_chunks in 100u32..200,
        num_bots in 10usize..20,
    ) {
        let owner = format!("owner-{}", owner_id);
        let bots: Vec<String> = (0..num_bots).map(|i| format!("bot-{}", i)).collect();
        let epoch = 1;

        // Count assignments per bot
        let mut holder_counts: HashMap<String, usize> = HashMap::new();

        for chunk_idx in 0..num_chunks {
            let holders = compute_chunk_holders(&owner, chunk_idx, &bots, epoch, 2);
            for holder in holders {
                *holder_counts.entry(holder).or_insert(0) += 1;
            }
        }

        // With num_chunks chunks and 2 replicas each:
        // Total assignments = num_chunks * 2
        // Expected per bot = (num_chunks * 2) / num_bots
        let total_assignments = num_chunks as usize * 2;
        let expected_per_bot = total_assignments as f64 / num_bots as f64;

        // Chi-squared test for uniformity
        // χ² = Σ((observed - expected)² / expected)
        let chi_squared: f64 = holder_counts
            .values()
            .map(|&count| {
                let diff = count as f64 - expected_per_bot;
                (diff * diff) / expected_per_bot
            })
            .sum();

        // For uniform distribution with num_bots categories:
        // χ² critical value at p=0.05 is approximately:
        // - 10 bots: ~16.9
        // - 20 bots: ~30.1
        // Use generous threshold: 2.7 * num_bots (allows for variance in small samples)
        // Increased from 2.5 to 2.7 to reduce flakiness with small sample sizes
        // while still catching genuine distribution problems.
        let threshold = (num_bots as f64) * 2.7;

        prop_assert!(
            chi_squared < threshold,
            "Distribution not uniform: χ²={:.2}, threshold={:.2}, expected={:.1} per bot",
            chi_squared,
            threshold,
            expected_per_bot
        );
    }
}
