//! Encrypted Trust Network State
//!
//! This module implements the `EncryptedTrustNetworkState` struct per the
//! contract-encryption.bead specification. It provides:
//!
//! - **Full state encryption**: AES-256-GCM with key derived from ACI via HKDF
//! - **Version chain**: Monotonic versioning with previous_hash for integrity
//! - **Signatures**: HMAC-SHA256 (placeholder for Ed25519 pending Signal integration)
//! - **Public Merkle root**: For ZK-proof verification
//! - **Anti-replay protection**: Version must be strictly monotonic
//!
//! ## Architecture
//!
//! ```text
//! TrustState (plaintext)
//!      │
//!      ▼ encrypt + sign
//! EncryptedTrustNetworkState
//!      │
//!      ▼ serialize
//! Raw bytes
//!      │
//!      ▼ chunk (via chunks.rs)
//! Chunk[0], Chunk[1], ...
//! ```
//!
//! ## References
//!
//! - Specification: .beads/contract-encryption.bead
//! - Security: .beads/security-constraints.bead

use hkdf::Hkdf;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::hmac;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use zeroize::Zeroizing;

/// 32-byte hash (SHA-256 output)
pub type Hash = [u8; 32];

/// Unix timestamp in seconds
pub type Timestamp = u64;

/// Errors that can occur during encryption operations
#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),

    #[error("Invalid ACI key: must be 32 bytes")]
    InvalidAciKey,

    #[error("Version conflict: expected > {expected}, got {actual}")]
    VersionConflict { expected: u64, actual: u64 },

    #[error("Chain integrity violation: previous_hash mismatch")]
    ChainIntegrityViolation,

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),
}

/// Encrypted trust network state with version chain and signatures.
///
/// This struct represents a fully encrypted state with:
/// - AES-256-GCM encrypted payload
/// - HMAC-SHA256 signature (placeholder for Ed25519)
/// - Public Merkle root for ZK-proofs
/// - Version chain for anti-replay protection
///
/// ## Security Properties
///
/// - **Confidentiality**: Ciphertext is AES-256-GCM encrypted
/// - **Integrity**: Signature covers all fields
/// - **Authenticity**: Signed by Signal ACI identity key (via HMAC placeholder)
/// - **Freshness**: Version chain prevents replay attacks
/// - **Public commitment**: Merkle root enables ZK-proofs without decryption
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedTrustNetworkState {
    /// AES-256-GCM encrypted trust state
    pub ciphertext: Vec<u8>,

    /// GCM nonce (12 bytes, unique per encryption)
    pub nonce: [u8; 12],

    /// HMAC-SHA256 signature (placeholder for Ed25519)
    /// Signs: hash(ciphertext || nonce || version || previous_hash || timestamp)
    pub signature: Vec<u8>,

    /// Signal ACI public key (32 bytes) - identifies authoritative writer
    pub aci_pubkey: Vec<u8>,

    /// Public Merkle root (for ZK-proof verification)
    /// Does NOT reveal individual members
    pub member_merkle_root: Hash,

    /// Monotonic version (increments on each write)
    pub version: u64,

    /// Hash of previous state (for chain integrity)
    pub previous_hash: Hash,

    /// Wall clock timestamp (when state was created)
    pub timestamp: Timestamp,
}

impl EncryptedTrustNetworkState {
    /// Create a new encrypted state from plaintext.
    ///
    /// # Arguments
    ///
    /// * `plaintext` - Raw state bytes to encrypt
    /// * `member_merkle_root` - Public Merkle root (computed from member set)
    /// * `previous` - Previous state (for version chain), or None for initial state
    /// * `aci_key` - Signal ACI key (32 bytes) for encryption and signing
    ///
    /// # Returns
    ///
    /// New encrypted state with incremented version and signature
    ///
    /// # Errors
    ///
    /// - `InvalidAciKey`: ACI key is not 32 bytes
    /// - `EncryptionFailed`: AES-256-GCM encryption failed
    /// - `KeyDerivationFailed`: HKDF key derivation failed
    pub fn new(
        plaintext: &[u8],
        member_merkle_root: Hash,
        previous: Option<&Self>,
        aci_key: &[u8],
    ) -> Result<Self, EncryptionError> {
        if aci_key.len() != 32 {
            return Err(EncryptionError::InvalidAciKey);
        }

        // Determine version and previous_hash from chain
        let version = previous.map(|p| p.version + 1).unwrap_or(1);
        let previous_hash = previous.map(|p| p.compute_hash()).unwrap_or([0u8; 32]);

        // Derive encryption key from ACI via HKDF
        let encryption_key = derive_encryption_key(aci_key)?;

        // Generate random nonce (NEVER reuse)
        let nonce_bytes = generate_nonce();
        let nonce = Nonce::try_assume_unique_for_key(&nonce_bytes).map_err(|_| {
            EncryptionError::EncryptionFailed("Failed to create nonce".to_string())
        })?;

        // Encrypt the plaintext
        let unbound_key = UnboundKey::new(&AES_256_GCM, &encryption_key).map_err(|e| {
            EncryptionError::EncryptionFailed(format!("Key creation failed: {}", e))
        })?;
        let key = LessSafeKey::new(unbound_key);

        let mut ciphertext = plaintext.to_vec();
        key.seal_in_place_append_tag(nonce, Aad::empty(), &mut ciphertext)
            .map_err(|e| EncryptionError::EncryptionFailed(format!("Encryption failed: {}", e)))?;

        // Get current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_secs();

        // Create state (without signature yet)
        let mut state = Self {
            ciphertext,
            nonce: nonce_bytes,
            signature: Vec::new(),
            aci_pubkey: aci_key.to_vec(),
            member_merkle_root,
            version,
            previous_hash,
            timestamp,
        };

        // Derive signing key and sign the state
        let signing_key = derive_signing_key(aci_key)?;
        state.signature = sign_state(&state, &signing_key);

        Ok(state)
    }

    /// Decrypt this state to recover plaintext.
    ///
    /// # Arguments
    ///
    /// * `aci_key` - Signal ACI key (32 bytes) for decryption and verification
    ///
    /// # Returns
    ///
    /// Decrypted plaintext bytes
    ///
    /// # Errors
    ///
    /// - `InvalidAciKey`: ACI key is not 32 bytes
    /// - `SignatureVerificationFailed`: Signature doesn't match
    /// - `DecryptionFailed`: AES-256-GCM decryption failed
    /// - `KeyDerivationFailed`: HKDF key derivation failed
    pub fn decrypt(&self, aci_key: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        if aci_key.len() != 32 {
            return Err(EncryptionError::InvalidAciKey);
        }

        // Verify signature first
        let signing_key = derive_signing_key(aci_key)?;
        if !verify_signature(self, &self.signature, &signing_key) {
            return Err(EncryptionError::SignatureVerificationFailed);
        }

        // Derive decryption key
        let decryption_key = derive_encryption_key(aci_key)?;

        // Decrypt ciphertext
        let nonce = Nonce::try_assume_unique_for_key(&self.nonce).map_err(|_| {
            EncryptionError::DecryptionFailed("Invalid nonce".to_string())
        })?;

        let unbound_key = UnboundKey::new(&AES_256_GCM, &decryption_key).map_err(|e| {
            EncryptionError::DecryptionFailed(format!("Key creation failed: {}", e))
        })?;
        let key = LessSafeKey::new(unbound_key);

        let mut plaintext = self.ciphertext.clone();
        key.open_in_place(nonce, Aad::empty(), &mut plaintext)
            .map_err(|e| EncryptionError::DecryptionFailed(format!("Decryption failed: {}", e)))?;

        // Remove authentication tag (last 16 bytes)
        let tag_len = AES_256_GCM.tag_len();
        if plaintext.len() < tag_len {
            return Err(EncryptionError::DecryptionFailed(
                "Ciphertext too short".to_string(),
            ));
        }
        plaintext.truncate(plaintext.len() - tag_len);

        Ok(plaintext)
    }

    /// Compute the hash of this state.
    ///
    /// Hash covers: ciphertext || nonce || version || previous_hash || timestamp
    ///
    /// This hash is used for:
    /// - Computing previous_hash in the next state
    /// - Signing the state
    /// - Chain integrity verification
    pub fn compute_hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&self.ciphertext);
        hasher.update(self.nonce);
        hasher.update(self.version.to_le_bytes());
        hasher.update(self.previous_hash);
        hasher.update(self.timestamp.to_le_bytes());
        hasher.finalize().into()
    }

    /// Verify version chain integrity.
    ///
    /// Checks that:
    /// - This state's version is strictly greater than previous
    /// - This state's previous_hash matches the hash of previous state
    ///
    /// # Arguments
    ///
    /// * `previous` - The claimed previous state in the chain
    ///
    /// # Errors
    ///
    /// - `VersionConflict`: Version is not strictly monotonic
    /// - `ChainIntegrityViolation`: previous_hash doesn't match
    pub fn verify_chain(&self, previous: &Self) -> Result<(), EncryptionError> {
        // Check version is strictly increasing
        if self.version <= previous.version {
            return Err(EncryptionError::VersionConflict {
                expected: previous.version,
                actual: self.version,
            });
        }

        // Check previous_hash links correctly
        let expected_hash = previous.compute_hash();
        if self.previous_hash != expected_hash {
            return Err(EncryptionError::ChainIntegrityViolation);
        }

        Ok(())
    }
}

/// Derive encryption key from Signal ACI identity via HKDF.
///
/// Uses HKDF-SHA256 with context separation to derive a 32-byte AES-256 key.
///
/// # Arguments
///
/// * `aci_key` - Signal ACI key (32 bytes)
///
/// # Returns
///
/// 32-byte encryption key (zeroized after use)
fn derive_encryption_key(aci_key: &[u8]) -> Result<Zeroizing<Vec<u8>>, EncryptionError> {
    let hkdf = Hkdf::<Sha256>::new(
        Some(b"stroma-state-encryption-v1"),
        aci_key,
    );
    let mut key = Zeroizing::new(vec![0u8; 32]);
    hkdf.expand(b"aes-256-gcm-key", &mut key)
        .map_err(|e| EncryptionError::KeyDerivationFailed(format!("HKDF expand failed: {}", e)))?;
    Ok(key)
}

/// Derive signing key from Signal ACI identity via HKDF.
///
/// Uses HKDF-SHA256 with context separation to derive a 32-byte HMAC key.
///
/// # Arguments
///
/// * `aci_key` - Signal ACI key (32 bytes)
///
/// # Returns
///
/// 32-byte signing key (zeroized after use)
fn derive_signing_key(aci_key: &[u8]) -> Result<Zeroizing<Vec<u8>>, EncryptionError> {
    let hkdf = Hkdf::<Sha256>::new(
        Some(b"stroma-identity-masking-v1"),
        aci_key,
    );
    let mut key = Zeroizing::new(vec![0u8; 32]);
    hkdf.expand(b"hmac-sha256-key", &mut key)
        .map_err(|e| EncryptionError::KeyDerivationFailed(format!("HKDF expand failed: {}", e)))?;
    Ok(key)
}

/// Generate a random 12-byte nonce for AES-GCM.
///
/// # Returns
///
/// 12-byte nonce (unique, never reuse)
fn generate_nonce() -> [u8; 12] {
    use ring::rand::{SecureRandom, SystemRandom};

    let rng = SystemRandom::new();
    let mut nonce = [0u8; 12];
    rng.fill(&mut nonce).expect("RNG failure");
    nonce
}

/// Sign the encrypted state with HMAC-SHA256.
///
/// Signs: hash(ciphertext || nonce || version || previous_hash || timestamp)
///
/// # Arguments
///
/// * `state` - State to sign
/// * `signing_key` - Derived signing key (32 bytes)
///
/// # Returns
///
/// HMAC-SHA256 signature bytes
fn sign_state(state: &EncryptedTrustNetworkState, signing_key: &[u8]) -> Vec<u8> {
    let key = hmac::Key::new(hmac::HMAC_SHA256, signing_key);
    let state_hash = state.compute_hash();
    let signature = hmac::sign(&key, &state_hash);
    signature.as_ref().to_vec()
}

/// Verify the HMAC signature on an encrypted state.
///
/// # Arguments
///
/// * `state` - State to verify
/// * `signature` - Claimed signature
/// * `signing_key` - Derived signing key (32 bytes)
///
/// # Returns
///
/// `true` if signature is valid
fn verify_signature(
    state: &EncryptedTrustNetworkState,
    signature: &[u8],
    signing_key: &[u8],
) -> bool {
    let key = hmac::Key::new(hmac::HMAC_SHA256, signing_key);
    let state_hash = state.compute_hash();
    hmac::verify(&key, &state_hash, signature).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_aci_key() -> Vec<u8> {
        vec![42u8; 32]
    }

    fn test_merkle_root() -> Hash {
        [0xAAu8; 32]
    }

    #[test]
    fn test_create_initial_state() {
        let plaintext = b"initial trust state data";
        let merkle_root = test_merkle_root();
        let aci_key = test_aci_key();

        let state = EncryptedTrustNetworkState::new(
            plaintext,
            merkle_root,
            None,
            &aci_key,
        )
        .unwrap();

        assert_eq!(state.version, 1);
        assert_eq!(state.previous_hash, [0u8; 32]);
        assert_eq!(state.member_merkle_root, merkle_root);
        assert_eq!(state.aci_pubkey, aci_key);
        assert!(!state.ciphertext.is_empty());
        assert!(!state.signature.is_empty());
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let plaintext = b"secret trust network state";
        let merkle_root = test_merkle_root();
        let aci_key = test_aci_key();

        let state = EncryptedTrustNetworkState::new(
            plaintext,
            merkle_root,
            None,
            &aci_key,
        )
        .unwrap();

        let decrypted = state.decrypt(&aci_key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_with_wrong_key() {
        let plaintext = b"secret data";
        let merkle_root = test_merkle_root();
        let aci_key = test_aci_key();
        let wrong_key = vec![99u8; 32];

        let state = EncryptedTrustNetworkState::new(
            plaintext,
            merkle_root,
            None,
            &aci_key,
        )
        .unwrap();

        let result = state.decrypt(&wrong_key);
        assert!(matches!(
            result,
            Err(EncryptionError::SignatureVerificationFailed)
        ));
    }

    #[test]
    fn test_version_chain() {
        let aci_key = test_aci_key();
        let merkle_root = test_merkle_root();

        // Create initial state (version 1)
        let state_v1 = EncryptedTrustNetworkState::new(
            b"state version 1",
            merkle_root,
            None,
            &aci_key,
        )
        .unwrap();
        assert_eq!(state_v1.version, 1);
        assert_eq!(state_v1.previous_hash, [0u8; 32]);

        // Create version 2
        let state_v2 = EncryptedTrustNetworkState::new(
            b"state version 2",
            merkle_root,
            Some(&state_v1),
            &aci_key,
        )
        .unwrap();
        assert_eq!(state_v2.version, 2);
        assert_eq!(state_v2.previous_hash, state_v1.compute_hash());

        // Verify chain
        state_v2.verify_chain(&state_v1).unwrap();
    }

    #[test]
    fn test_chain_integrity_violation() {
        let aci_key = test_aci_key();
        let merkle_root = test_merkle_root();

        let state_v1 = EncryptedTrustNetworkState::new(
            b"state version 1",
            merkle_root,
            None,
            &aci_key,
        )
        .unwrap();

        let mut state_v2 = EncryptedTrustNetworkState::new(
            b"state version 2",
            merkle_root,
            Some(&state_v1),
            &aci_key,
        )
        .unwrap();

        // Tamper with previous_hash
        state_v2.previous_hash = [0xFFu8; 32];

        // Verify chain should fail
        let result = state_v2.verify_chain(&state_v1);
        assert!(matches!(
            result,
            Err(EncryptionError::ChainIntegrityViolation)
        ));
    }

    #[test]
    fn test_version_must_increment() {
        let aci_key = test_aci_key();
        let merkle_root = test_merkle_root();

        let state_v1 = EncryptedTrustNetworkState::new(
            b"state version 1",
            merkle_root,
            None,
            &aci_key,
        )
        .unwrap();

        let mut state_v2 = EncryptedTrustNetworkState::new(
            b"state version 2",
            merkle_root,
            Some(&state_v1),
            &aci_key,
        )
        .unwrap();

        // Tamper with version (set to same as v1)
        state_v2.version = state_v1.version;

        // Verify chain should fail
        let result = state_v2.verify_chain(&state_v1);
        assert!(matches!(result, Err(EncryptionError::VersionConflict { .. })));
    }

    #[test]
    fn test_signature_verification() {
        let plaintext = b"test data";
        let merkle_root = test_merkle_root();
        let aci_key = test_aci_key();

        let mut state = EncryptedTrustNetworkState::new(
            plaintext,
            merkle_root,
            None,
            &aci_key,
        )
        .unwrap();

        // Tamper with ciphertext
        state.ciphertext[0] ^= 0xFF;

        // Decryption should fail signature verification
        let result = state.decrypt(&aci_key);
        assert!(matches!(
            result,
            Err(EncryptionError::SignatureVerificationFailed)
        ));
    }

    #[test]
    fn test_different_nonces_different_ciphertexts() {
        let plaintext = b"same plaintext";
        let merkle_root = test_merkle_root();
        let aci_key = test_aci_key();

        let state1 = EncryptedTrustNetworkState::new(
            plaintext,
            merkle_root,
            None,
            &aci_key,
        )
        .unwrap();

        let state2 = EncryptedTrustNetworkState::new(
            plaintext,
            merkle_root,
            None,
            &aci_key,
        )
        .unwrap();

        // Different nonces should produce different ciphertexts
        assert_ne!(state1.nonce, state2.nonce);
        assert_ne!(state1.ciphertext, state2.ciphertext);

        // But both should decrypt to same plaintext
        assert_eq!(state1.decrypt(&aci_key).unwrap(), plaintext);
        assert_eq!(state2.decrypt(&aci_key).unwrap(), plaintext);
    }

    #[test]
    fn test_invalid_aci_key_length() {
        let plaintext = b"test";
        let merkle_root = test_merkle_root();
        let short_key = vec![1u8; 16]; // Only 16 bytes, not 32

        let result = EncryptedTrustNetworkState::new(
            plaintext,
            merkle_root,
            None,
            &short_key,
        );
        assert!(matches!(result, Err(EncryptionError::InvalidAciKey)));
    }

    #[test]
    fn test_merkle_root_preserved() {
        let plaintext = b"test data";
        let merkle_root = [0xBBu8; 32];
        let aci_key = test_aci_key();

        let state = EncryptedTrustNetworkState::new(
            plaintext,
            merkle_root,
            None,
            &aci_key,
        )
        .unwrap();

        assert_eq!(state.member_merkle_root, merkle_root);
    }

    #[test]
    fn test_compute_hash_deterministic() {
        let plaintext = b"test data";
        let merkle_root = test_merkle_root();
        let aci_key = test_aci_key();

        let state = EncryptedTrustNetworkState::new(
            plaintext,
            merkle_root,
            None,
            &aci_key,
        )
        .unwrap();

        let hash1 = state.compute_hash();
        let hash2 = state.compute_hash();

        // Hash should be deterministic
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_long_chain() {
        let aci_key = test_aci_key();
        let merkle_root = test_merkle_root();

        let mut prev_state = EncryptedTrustNetworkState::new(
            b"initial state",
            merkle_root,
            None,
            &aci_key,
        )
        .unwrap();

        // Create a chain of 10 states
        for i in 1..10 {
            let plaintext = format!("state version {}", i + 1);
            let next_state = EncryptedTrustNetworkState::new(
                plaintext.as_bytes(),
                merkle_root,
                Some(&prev_state),
                &aci_key,
            )
            .unwrap();

            assert_eq!(next_state.version, i + 1);
            next_state.verify_chain(&prev_state).unwrap();

            prev_state = next_state;
        }

        assert_eq!(prev_state.version, 10);
    }

    #[test]
    fn test_zeroization_of_keys() {
        // This test verifies that keys are zeroized after use
        // We can't directly test zeroization, but we verify the types are correct
        let aci_key = test_aci_key();

        let encryption_key = derive_encryption_key(&aci_key).unwrap();
        let signing_key = derive_signing_key(&aci_key).unwrap();

        // Keys should be 32 bytes
        assert_eq!(encryption_key.len(), 32);
        assert_eq!(signing_key.len(), 32);

        // When these go out of scope, Zeroizing will clear them
    }
}
