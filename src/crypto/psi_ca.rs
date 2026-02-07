//! Private Set Intersection with Cardinality Only (PSI-CA)
//!
//! Implements commutative encryption protocol for calculating intersection size
//! between two sets WITHOUT revealing member identities.
//!
//! # Protocol Overview
//!
//! 1. **Phase 1**: Group A encrypts members with ephemeral key, sends to Group B
//! 2. **Phase 2**: Group B double-blinds A's set and sends back, plus their own encrypted set
//! 3. **Phase 3**: Group A completes double-blinding and calculates intersection COUNT only
//!
//! # Security Properties
//!
//! - **Commutative Encryption**: E(k_a, E(k_b, m)) = E(k_b, E(k_a, m))
//! - **Double-Blinding**: Neither party can decrypt alone
//! - **Cardinality Only**: Only intersection SIZE revealed, not identities
//! - **Ephemeral Keys**: Destroyed after handshake (no replay attacks)
//!
//! # Mock Implementation Note
//!
//! This implementation uses simplified commutative encryption (XOR with derived keys)
//! for demonstration purposes. Production should use ECIES (Elliptic Curve Integrated
//! Encryption Scheme) with proper key agreement.
//!
//! See: docs/ALGORITHMS.md § "External Federation: Private Set Intersection Algorithm"

use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// PSI-CA protocol errors
#[derive(Error, Debug)]
pub enum PsiError {
    #[error("Failed to generate random key: {0}")]
    KeyGeneration(String),

    #[error("Encryption failed: {0}")]
    Encryption(String),

    #[error("Invalid threshold: must be between 0.0 and 1.0")]
    InvalidThreshold,
}

/// Ephemeral key pair for PSI-CA protocol
///
/// Keys are automatically zeroized on drop to prevent memory leakage.
#[derive(ZeroizeOnDrop)]
pub struct EphemeralKey {
    /// Private key (32 bytes random)
    #[zeroize(drop)]
    private_key: Vec<u8>,

    /// Public key (derived from private key)
    pub public_key: Vec<u8>,
}

impl EphemeralKey {
    /// Generate new ephemeral key pair
    pub fn generate() -> Result<Self, PsiError> {
        let rng = SystemRandom::new();
        let mut private_key = vec![0u8; 32];

        rng.fill(&mut private_key)
            .map_err(|e| PsiError::KeyGeneration(e.to_string()))?;

        // Derive public key from private key (using SHA-256 for mock implementation)
        let mut hasher = Sha256::new();
        hasher.update(&private_key);
        hasher.update(b"PUBLIC_KEY_DERIVATION");
        let public_key = hasher.finalize().to_vec();

        Ok(Self {
            private_key,
            public_key,
        })
    }

    /// Get encryption key for a specific element
    ///
    /// This derives a per-element key from the private key and element hash,
    /// ensuring commutative property: encrypt(k_a, encrypt(k_b, m)) = encrypt(k_b, encrypt(k_a, m))
    #[allow(dead_code)]
    fn derive_element_key(&self, element_hash: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.private_key);
        hasher.update(element_hash);
        hasher.finalize().to_vec()
    }
}

/// Federation threshold configuration
#[derive(Debug, Clone)]
pub struct FederationThreshold {
    /// Minimum intersection density (overlap / group_size) to accept federation
    /// Must be between 0.0 and 1.0 (e.g., 0.10 = 10%)
    pub threshold: f32,

    /// Group size (number of members)
    pub group_size: usize,
}

impl FederationThreshold {
    /// Create new threshold configuration
    pub fn new(threshold: f32, group_size: usize) -> Result<Self, PsiError> {
        if !(0.0..=1.0).contains(&threshold) {
            return Err(PsiError::InvalidThreshold);
        }

        Ok(Self {
            threshold,
            group_size,
        })
    }

    /// Calculate if overlap meets threshold
    pub fn accepts(&self, overlap: usize) -> bool {
        let density = overlap as f32 / self.group_size as f32;
        density >= self.threshold
    }
}

/// PSI-CA Protocol implementation
pub struct PsiProtocol {
    /// Our ephemeral key
    key: EphemeralKey,

    /// Our members (cleartext - ONLY for PSI-CA, immediately zeroized)
    /// NOTE: This is the ONE exception where cleartext Signal IDs are stored.
    /// In all other code paths, Signal IDs are HMAC-hashed immediately.
    members: Vec<String>,

    /// Federation threshold configuration
    threshold: FederationThreshold,
}

impl PsiProtocol {
    /// Create new PSI-CA protocol instance
    ///
    /// # Critical Security Note
    ///
    /// The `members` parameter contains cleartext Signal IDs. This is the ONLY place
    /// in the codebase where cleartext IDs are bulk-accessed. This exception is required
    /// for PSI-CA federation discovery. The cleartext data will be zeroized when the
    /// protocol instance is dropped.
    ///
    /// See: docs/ALGORITHMS.md lines 456-472 for security rationale
    pub fn new(members: Vec<String>, threshold: FederationThreshold) -> Result<Self, PsiError> {
        let key = EphemeralKey::generate()?;

        Ok(Self {
            key,
            members,
            threshold,
        })
    }

    /// Phase 1: Encrypt our members with our ephemeral key
    ///
    /// Returns encrypted member set to send to other group.
    pub fn phase1_encrypt_members(&self) -> Result<Vec<Vec<u8>>, PsiError> {
        let mut encrypted = Vec::new();

        for member in &self.members {
            let ciphertext = self.encrypt(&self.key, member.as_bytes())?;
            encrypted.push(ciphertext);
        }

        Ok(encrypted)
    }

    /// Phase 2: Double-blind encryption
    ///
    /// Takes the other group's encrypted set and their public key, returns:
    /// - Their encrypted set, re-encrypted with our key (double-blind)
    /// - Our already-encrypted members (from phase1), re-encrypted with their key (double-blind)
    ///
    /// Note: We need our own phase1 encrypted set to properly double-blind it
    #[allow(clippy::type_complexity)]
    pub fn phase2_double_blind(
        &self,
        their_public_key: &[u8],
        their_encrypted: &[Vec<u8>],
        our_encrypted: &[Vec<u8>],
    ) -> Result<(Vec<Vec<u8>>, Vec<Vec<u8>>), PsiError> {
        // Re-encrypt their set with our key (double-blind)
        // Result: E(our_key, E(their_key, their_members))
        let their_double_blind: Result<Vec<_>, _> = their_encrypted
            .iter()
            .map(|ct| self.reencrypt(&self.key, ct))
            .collect();
        let their_double_blind = their_double_blind?;

        // Create temporary key object for their public key
        let their_key_obj = EphemeralKey {
            private_key: vec![0u8; 32], // Not used for encryption
            public_key: their_public_key.to_vec(),
        };

        // Re-encrypt our already-encrypted members with their key (double-blind)
        // our_encrypted is already E(our_key, our_members) from phase 1
        // Result: E(their_key, E(our_key, our_members))
        let our_double_blind: Result<Vec<_>, _> = our_encrypted
            .iter()
            .map(|ct| self.reencrypt(&their_key_obj, ct))
            .collect();
        let our_double_blind = our_double_blind?;

        Ok((their_double_blind, our_double_blind))
    }

    /// Phase 3: Calculate intersection cardinality
    ///
    /// Takes our double-blind set and their double-blind set, returns overlap COUNT only.
    pub fn phase3_calculate_overlap(
        our_double_blind: &[Vec<u8>],
        their_double_blind: &[Vec<u8>],
    ) -> usize {
        // Convert to sets for efficient intersection
        let our_set: HashSet<_> = our_double_blind.iter().collect();
        let their_set: HashSet<_> = their_double_blind.iter().collect();

        // Return intersection count (NOT identities)
        our_set.intersection(&their_set).count()
    }

    /// Evaluate if federation should be proposed
    pub fn evaluate_federation(&self, overlap: usize, _other_group_size: usize) -> bool {
        // Check if we accept

        // We can't evaluate if they accept without knowing their threshold,
        // but we return our decision
        self.threshold.accepts(overlap)
    }

    /// Get our public key to send to other group
    pub fn public_key(&self) -> &[u8] {
        &self.key.public_key
    }

    /// Get our group size
    pub fn group_size(&self) -> usize {
        self.threshold.group_size
    }

    /// Encrypt plaintext with ephemeral key
    ///
    /// Mock implementation using commutative addition.
    /// For commutative property: E(k_a, E(k_b, m)) = E(k_b, E(k_a, m))
    ///
    /// We achieve this by using addition:
    /// - E(k, data) = data + Hash(k) (bytewise)
    /// - Addition is commutative: (m + k_a) + k_b = (m + k_b) + k_a
    ///
    /// Production should use ECIES with proper Diffie-Hellman key agreement.
    fn encrypt(&self, key: &EphemeralKey, plaintext: &[u8]) -> Result<Vec<u8>, PsiError> {
        // Hash the plaintext to get fixed-size representation
        let mut hasher = Sha256::new();
        hasher.update(plaintext);
        let element_hash = hasher.finalize();

        // Perform commutative encryption operation
        self.apply_encryption_layer(key, &element_hash)
    }

    /// Re-encrypt already encrypted data with our key
    ///
    /// This is used in double-blinding to add another layer of encryption.
    /// For commutative property, we use the SAME operation as encrypt.
    fn reencrypt(&self, key: &EphemeralKey, ciphertext: &[u8]) -> Result<Vec<u8>, PsiError> {
        if ciphertext.len() != 32 {
            return Err(PsiError::Encryption(
                "Invalid ciphertext length".to_string(),
            ));
        }

        // Use same encryption operation as encrypt() for commutativity
        let mut ciphertext_array = [0u8; 32];
        ciphertext_array.copy_from_slice(ciphertext);

        self.apply_encryption_layer(key, &ciphertext_array)
    }

    /// Apply one layer of commutative encryption
    ///
    /// This is the core operation that must be commutative.
    /// We use: result = data + Hash(key)
    fn apply_encryption_layer(&self, key: &EphemeralKey, data: &[u8]) -> Result<Vec<u8>, PsiError> {
        // Hash the public key to get the encryption value
        // Using same hash for both encrypt and reencrypt ensures commutativity
        let mut key_hasher = Sha256::new();
        key_hasher.update(&key.public_key);
        let key_hash = key_hasher.finalize();

        // Add data and key_hash (bytewise with wrapping)
        // This is commutative: (data + k_a) + k_b = (data + k_b) + k_a
        let mut result = vec![0u8; 32];
        for i in 0..32 {
            result[i] = data[i].wrapping_add(key_hash[i]);
        }

        Ok(result)
    }
}

impl Drop for PsiProtocol {
    fn drop(&mut self) {
        // Zeroize cleartext members on drop (security-critical)
        for member in &mut self.members {
            member.zeroize();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock Signal IDs for testing
    /// NOTE: These are MOCK DATA ONLY - NO REAL BROADCASTS per task requirements
    fn mock_signal_ids_group_a() -> Vec<String> {
        vec![
            "alice.01".to_string(),
            "bob.02".to_string(),
            "carol.03".to_string(),
            "david.04".to_string(),
            "eve.05".to_string(),
            "frank.06".to_string(),
            "grace.07".to_string(),
            "henry.08".to_string(),
            "ivy.09".to_string(),
            "jack.10".to_string(),
        ]
    }

    fn mock_signal_ids_group_b() -> Vec<String> {
        vec![
            "carol.03".to_string(), // Overlap
            "david.04".to_string(), // Overlap
            "eve.05".to_string(),   // Overlap
            "frank.06".to_string(), // Overlap
            "grace.07".to_string(), // Overlap
            "kate.11".to_string(),
            "leo.12".to_string(),
            "mia.13".to_string(),
            "nina.14".to_string(),
            "oscar.15".to_string(),
        ]
    }

    #[test]
    fn test_ephemeral_key_generation() {
        let key1 = EphemeralKey::generate().unwrap();
        let key2 = EphemeralKey::generate().unwrap();

        // Keys should be different
        assert_ne!(key1.public_key, key2.public_key);

        // Keys should be 32 bytes
        assert_eq!(key1.public_key.len(), 32);
        assert_eq!(key2.public_key.len(), 32);
    }

    #[test]
    fn test_federation_threshold() {
        let threshold = FederationThreshold::new(0.20, 100).unwrap();

        // 25 overlap / 100 members = 25% >= 20% threshold
        assert!(threshold.accepts(25));

        // 15 overlap / 100 members = 15% < 20% threshold
        assert!(!threshold.accepts(15));

        // Edge case: exactly at threshold
        assert!(threshold.accepts(20));
    }

    #[test]
    fn test_invalid_threshold() {
        // Threshold must be between 0.0 and 1.0
        assert!(FederationThreshold::new(1.5, 100).is_err());
        assert!(FederationThreshold::new(-0.1, 100).is_err());
    }

    #[test]
    fn test_psi_ca_full_protocol() {
        // Setup: Two groups with 5 overlapping members
        let group_a_members = mock_signal_ids_group_a(); // 10 members
        let group_b_members = mock_signal_ids_group_b(); // 10 members, 5 overlap

        let threshold_a = FederationThreshold::new(0.30, 10).unwrap(); // Need 30%
        let threshold_b = FederationThreshold::new(0.30, 10).unwrap(); // Need 30%

        let psi_a = PsiProtocol::new(group_a_members, threshold_a).unwrap();
        let psi_b = PsiProtocol::new(group_b_members, threshold_b).unwrap();

        // Phase 1: Group A encrypts their members
        let a_encrypted = psi_a.phase1_encrypt_members().unwrap();
        assert_eq!(a_encrypted.len(), 10);

        // Phase 1: Group B encrypts their members
        let b_encrypted = psi_b.phase1_encrypt_members().unwrap();
        assert_eq!(b_encrypted.len(), 10);

        // Phase 2: Group A receives B's encrypted set and double-blinds
        let (_b_double_blind_by_a, a_double_blind) = psi_a
            .phase2_double_blind(psi_b.public_key(), &b_encrypted, &a_encrypted)
            .unwrap();

        // Phase 2: Group B receives A's encrypted set and double-blinds
        let (_a_double_blind_by_b, b_double_blind) = psi_b
            .phase2_double_blind(psi_a.public_key(), &a_encrypted, &b_encrypted)
            .unwrap();

        // Phase 3: Both groups calculate overlap
        // A compares their own double-blind members with B's double-blind members
        let overlap_calculated_by_a =
            PsiProtocol::phase3_calculate_overlap(&a_double_blind, &b_double_blind);

        // B compares their own double-blind members with A's double-blind members
        let overlap_calculated_by_b =
            PsiProtocol::phase3_calculate_overlap(&b_double_blind, &a_double_blind);

        // Both should get the same overlap count
        assert_eq!(overlap_calculated_by_a, overlap_calculated_by_b);

        // Expected overlap: carol, david, eve, frank, grace = 5 members
        assert_eq!(overlap_calculated_by_a, 5);

        // Evaluate federation: 5/10 = 50% >= 30% threshold
        assert!(psi_a.evaluate_federation(overlap_calculated_by_a, 10));
        assert!(psi_b.evaluate_federation(overlap_calculated_by_b, 10));
    }

    #[test]
    fn test_commutative_property() {
        // Test that E(k_a, E(k_b, m)) = E(k_b, E(k_a, m))

        let plaintext = b"test_member_signal_id";

        let key_a = EphemeralKey::generate().unwrap();
        let key_b = EphemeralKey::generate().unwrap();

        let threshold = FederationThreshold::new(0.10, 10).unwrap();
        let psi = PsiProtocol::new(vec![], threshold).unwrap();

        // Encrypt with A, then B
        let ct_a = psi.encrypt(&key_a, plaintext).unwrap();
        let ct_ab = psi.reencrypt(&key_b, &ct_a).unwrap();

        // Encrypt with B, then A
        let ct_b = psi.encrypt(&key_b, plaintext).unwrap();
        let ct_ba = psi.reencrypt(&key_a, &ct_b).unwrap();

        // Should be equal (commutative property)
        assert_eq!(ct_ab, ct_ba);
    }

    #[test]
    fn test_no_overlap_scenario() {
        // Test with completely disjoint sets
        let group_a = vec!["alice.01".to_string(), "bob.02".to_string()];
        let group_b = vec!["kate.11".to_string(), "leo.12".to_string()];

        let threshold_a = FederationThreshold::new(0.10, 2).unwrap();
        let threshold_b = FederationThreshold::new(0.10, 2).unwrap();

        let psi_a = PsiProtocol::new(group_a, threshold_a).unwrap();
        let psi_b = PsiProtocol::new(group_b, threshold_b).unwrap();

        let a_encrypted = psi_a.phase1_encrypt_members().unwrap();
        let b_encrypted = psi_b.phase1_encrypt_members().unwrap();

        let (_b_double_blind_by_a, a_double_blind) = psi_a
            .phase2_double_blind(psi_b.public_key(), &b_encrypted, &a_encrypted)
            .unwrap();

        let (_, b_double_blind) = psi_b
            .phase2_double_blind(psi_a.public_key(), &a_encrypted, &b_encrypted)
            .unwrap();

        let overlap = PsiProtocol::phase3_calculate_overlap(&a_double_blind, &b_double_blind);

        // Should be zero overlap
        assert_eq!(overlap, 0);

        // Should reject federation (0% < 10%)
        assert!(!psi_a.evaluate_federation(overlap, 2));
    }

    #[test]
    fn test_complete_overlap_scenario() {
        // Test with identical sets
        let members = vec![
            "alice.01".to_string(),
            "bob.02".to_string(),
            "carol.03".to_string(),
        ];

        let threshold = FederationThreshold::new(0.90, 3).unwrap(); // Need 90%

        let psi_a = PsiProtocol::new(members.clone(), threshold.clone()).unwrap();
        let psi_b = PsiProtocol::new(members, threshold).unwrap();

        let a_encrypted = psi_a.phase1_encrypt_members().unwrap();
        let b_encrypted = psi_b.phase1_encrypt_members().unwrap();

        let (_b_double_blind_by_a, a_double_blind) = psi_a
            .phase2_double_blind(psi_b.public_key(), &b_encrypted, &a_encrypted)
            .unwrap();

        let (_, b_double_blind) = psi_b
            .phase2_double_blind(psi_a.public_key(), &a_encrypted, &b_encrypted)
            .unwrap();

        let overlap = PsiProtocol::phase3_calculate_overlap(&a_double_blind, &b_double_blind);

        // Should be complete overlap
        assert_eq!(overlap, 3);

        // Should accept federation (100% >= 90%)
        assert!(psi_a.evaluate_federation(overlap, 3));
    }

    #[test]
    fn test_group_size() {
        // Test that group_size() returns correct value
        let members = vec!["alice.01".to_string(), "bob.02".to_string()];
        let threshold = FederationThreshold::new(0.10, 5).unwrap();
        let psi = PsiProtocol::new(members, threshold).unwrap();

        assert_eq!(psi.group_size(), 5);
    }

    #[test]
    fn test_reencrypt_invalid_ciphertext_length() {
        // Test error handling for invalid ciphertext lengths
        let members = vec!["alice.01".to_string()];
        let threshold = FederationThreshold::new(0.10, 1).unwrap();
        let psi = PsiProtocol::new(members, threshold).unwrap();

        let key = EphemeralKey::generate().unwrap();

        // Test with too short ciphertext (< 32 bytes)
        let short_ciphertext = vec![0u8; 16];
        let result = psi.reencrypt(&key, &short_ciphertext);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PsiError::Encryption(_)));

        // Test with too long ciphertext (> 32 bytes)
        let long_ciphertext = vec![0u8; 48];
        let result = psi.reencrypt(&key, &long_ciphertext);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PsiError::Encryption(_)));
    }

    #[test]
    fn test_derive_element_key_determinism() {
        // Test that derive_element_key produces same output for same input
        let key = EphemeralKey::generate().unwrap();
        let element = b"test_element";

        let derived1 = key.derive_element_key(element);
        let derived2 = key.derive_element_key(element);

        assert_eq!(derived1, derived2, "Derived keys should be deterministic");
        assert_eq!(derived1.len(), 32, "Derived key should be 32 bytes");
    }

    #[test]
    fn test_derive_element_key_isolation() {
        // Test that different elements produce different keys
        let key = EphemeralKey::generate().unwrap();

        let key1 = key.derive_element_key(b"element1");
        let key2 = key.derive_element_key(b"element2");

        assert_ne!(
            key1, key2,
            "Different elements should produce different keys"
        );
    }

    #[test]
    fn test_derive_element_key_different_keys() {
        // Test that different ephemeral keys produce different element keys
        let key_a = EphemeralKey::generate().unwrap();
        let key_b = EphemeralKey::generate().unwrap();

        let element = b"same_element";
        let derived_a = key_a.derive_element_key(element);
        let derived_b = key_b.derive_element_key(element);

        assert_ne!(
            derived_a, derived_b,
            "Different ephemeral keys should produce different element keys"
        );
    }

    // ============================================================================
    // PROPERTY TESTS - Cryptographic Invariants
    // ============================================================================
    //
    // These tests verify fundamental cryptographic properties using proptest.
    // All tests use fixed seeds for determinism (required by testing-standards.bead).

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        // Fixed seed for deterministic property tests (required for CI/reproducibility)
        // 32 bytes for ChaCha RNG
        const PROPTEST_SEED: &str = "0123456789abcdef0123456789abcdef";

        /// Property test: Encryption is deterministic
        ///
        /// For any key and plaintext, encrypting the same data twice produces
        /// identical output: E(k, m) == E(k, m)
        #[test]
        fn prop_encryption_determinism() {
            let config = ProptestConfig {
                rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha,
                ..Default::default()
            };
            let mut runner = proptest::test_runner::TestRunner::new_with_rng(
                config,
                proptest::test_runner::TestRng::from_seed(
                    proptest::test_runner::RngAlgorithm::ChaCha,
                    PROPTEST_SEED.as_bytes(),
                ),
            );

            let strategy = prop::collection::vec(prop::num::u8::ANY, 1..100);

            runner
                .run(&strategy, |plaintext_bytes| {
                    let plaintext = String::from_utf8_lossy(&plaintext_bytes).to_string();
                    let key = EphemeralKey::generate().unwrap();
                    let threshold = FederationThreshold::new(0.10, 10).unwrap();
                    let psi = PsiProtocol::new(vec![], threshold).unwrap();

                    let ct1 = psi.encrypt(&key, plaintext.as_bytes()).unwrap();
                    let ct2 = psi.encrypt(&key, plaintext.as_bytes()).unwrap();

                    prop_assert_eq!(ct1, ct2, "Encryption must be deterministic");
                    Ok(())
                })
                .unwrap();
        }

        /// Property test: Key isolation
        ///
        /// Different keys produce different ciphertexts for the same plaintext:
        /// For k1 != k2: E(k1, m) != E(k2, m) (with overwhelming probability)
        #[test]
        fn prop_key_isolation() {
            let config = ProptestConfig {
                rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha,
                ..Default::default()
            };
            let mut runner = proptest::test_runner::TestRunner::new_with_rng(
                config,
                proptest::test_runner::TestRng::from_seed(
                    proptest::test_runner::RngAlgorithm::ChaCha,
                    PROPTEST_SEED.as_bytes(),
                ),
            );

            let strategy = prop::collection::vec(prop::num::u8::ANY, 1..100);

            runner
                .run(&strategy, |plaintext_bytes| {
                    let plaintext = String::from_utf8_lossy(&plaintext_bytes).to_string();
                    let key1 = EphemeralKey::generate().unwrap();
                    let key2 = EphemeralKey::generate().unwrap();
                    let threshold = FederationThreshold::new(0.10, 10).unwrap();
                    let psi = PsiProtocol::new(vec![], threshold).unwrap();

                    let ct1 = psi.encrypt(&key1, plaintext.as_bytes()).unwrap();
                    let ct2 = psi.encrypt(&key2, plaintext.as_bytes()).unwrap();

                    prop_assert_ne!(
                        ct1,
                        ct2,
                        "Different keys must produce different ciphertexts"
                    );
                    Ok(())
                })
                .unwrap();
        }

        /// Property test: Commutativity of double encryption
        ///
        /// The order of encryption doesn't matter:
        /// E(k_a, E(k_b, m)) == E(k_b, E(k_a, m))
        ///
        /// This is the CRITICAL property for PSI-CA protocol correctness.
        #[test]
        fn prop_encryption_commutativity() {
            let config = ProptestConfig {
                rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha,
                ..Default::default()
            };
            let mut runner = proptest::test_runner::TestRunner::new_with_rng(
                config,
                proptest::test_runner::TestRng::from_seed(
                    proptest::test_runner::RngAlgorithm::ChaCha,
                    PROPTEST_SEED.as_bytes(),
                ),
            );

            let strategy = prop::collection::vec(prop::num::u8::ANY, 1..100);

            runner
                .run(&strategy, |plaintext_bytes| {
                    let plaintext = String::from_utf8_lossy(&plaintext_bytes).to_string();
                    let key_a = EphemeralKey::generate().unwrap();
                    let key_b = EphemeralKey::generate().unwrap();
                    let threshold = FederationThreshold::new(0.10, 10).unwrap();
                    let psi = PsiProtocol::new(vec![], threshold).unwrap();

                    // Encrypt with A, then B
                    let ct_a = psi.encrypt(&key_a, plaintext.as_bytes()).unwrap();
                    let ct_ab = psi.reencrypt(&key_b, &ct_a).unwrap();

                    // Encrypt with B, then A
                    let ct_b = psi.encrypt(&key_b, plaintext.as_bytes()).unwrap();
                    let ct_ba = psi.reencrypt(&key_a, &ct_b).unwrap();

                    prop_assert_eq!(
                        ct_ab,
                        ct_ba,
                        "Encryption must be commutative: E(ka, E(kb, m)) == E(kb, E(ka, m))"
                    );
                    Ok(())
                })
                .unwrap();
        }

        /// Property test: Collision resistance (different inputs)
        ///
        /// Different plaintexts produce different ciphertexts:
        /// For m1 != m2: E(k, m1) != E(k, m2) (with overwhelming probability)
        #[test]
        fn prop_collision_resistance() {
            let config = ProptestConfig {
                rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha,
                ..Default::default()
            };
            let mut runner = proptest::test_runner::TestRunner::new_with_rng(
                config,
                proptest::test_runner::TestRng::from_seed(
                    proptest::test_runner::RngAlgorithm::ChaCha,
                    PROPTEST_SEED.as_bytes(),
                ),
            );

            let strategy = (
                prop::collection::vec(prop::num::u8::ANY, 1..100),
                prop::collection::vec(prop::num::u8::ANY, 1..100),
            );

            runner
                .run(&strategy, |(plaintext1_bytes, plaintext2_bytes)| {
                    // Only test if plaintexts are actually different
                    if plaintext1_bytes == plaintext2_bytes {
                        return Ok(());
                    }

                    let plaintext1 = String::from_utf8_lossy(&plaintext1_bytes).to_string();
                    let plaintext2 = String::from_utf8_lossy(&plaintext2_bytes).to_string();
                    let key = EphemeralKey::generate().unwrap();
                    let threshold = FederationThreshold::new(0.10, 10).unwrap();
                    let psi = PsiProtocol::new(vec![], threshold).unwrap();

                    let ct1 = psi.encrypt(&key, plaintext1.as_bytes()).unwrap();
                    let ct2 = psi.encrypt(&key, plaintext2.as_bytes()).unwrap();

                    prop_assert_ne!(
                        ct1,
                        ct2,
                        "Different plaintexts must produce different ciphertexts"
                    );
                    Ok(())
                })
                .unwrap();
        }

        /// Property test: derive_element_key determinism
        ///
        /// Deriving an element key multiple times produces same result
        #[test]
        fn prop_derive_element_key_determinism() {
            let config = ProptestConfig {
                rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha,
                ..Default::default()
            };
            let mut runner = proptest::test_runner::TestRunner::new_with_rng(
                config,
                proptest::test_runner::TestRng::from_seed(
                    proptest::test_runner::RngAlgorithm::ChaCha,
                    PROPTEST_SEED.as_bytes(),
                ),
            );

            let strategy = prop::collection::vec(prop::num::u8::ANY, 1..100);

            runner
                .run(&strategy, |element_bytes| {
                    let key = EphemeralKey::generate().unwrap();

                    let derived1 = key.derive_element_key(&element_bytes);
                    let derived2 = key.derive_element_key(&element_bytes);

                    prop_assert_eq!(derived1.len(), 32, "Derived key must be 32 bytes");
                    prop_assert_eq!(derived2.len(), 32, "Derived key must be 32 bytes");
                    prop_assert_eq!(
                        derived1,
                        derived2,
                        "derive_element_key must be deterministic"
                    );
                    Ok(())
                })
                .unwrap();
        }
    }
}
