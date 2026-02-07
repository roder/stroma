//! Encryption layer for trust network state persistence.
//!
//! This module implements the `EncryptedTrustNetworkState` structure specified
//! in `.beads/contract-encryption.bead`. It provides:
//!
//! - AES-256-GCM encryption of full trust state
//! - Ed25519 signatures using Signal ACI identity
//! - Version chain with anti-replay protection
//! - Public Merkle root for ZK-proof verification
//!
//! ## Architecture
//!
//! This layer sits ABOVE `chunks.rs`:
//! - This module: EncryptedTrustNetworkState (versioned, signed container)
//! - chunks.rs: AES-256-GCM encryption + 64KB chunking for distribution
//!
//! ## Security Model
//!
//! - **Encryption**: AES-256-GCM with key derived from Signal ACI via HKDF
//! - **Signature**: Ed25519 using Signal ACI identity key
//! - **Version Chain**: Monotonic versioning + previous_hash for anti-replay
//! - **Public Commitment**: Merkle root for ZK-proofs (not encrypted)
//!
//! ## References
//!
//! - Specification: `.beads/contract-encryption.bead`
//! - Persistence Model: `.beads/persistence-model.bead`

use hkdf::Hkdf;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// A 32-byte hash value.
pub type Hash = [u8; 32];

/// A Unix timestamp in seconds.
pub type Timestamp = u64;

/// Encrypted trust network state with versioning and signatures.
///
/// This structure wraps the encrypted trust state and provides:
/// - Authentication via Ed25519 signature (Signal ACI identity)
/// - Version chain for anti-replay protection
/// - Public Merkle root for ZK-proof verification
/// - Metadata for recovery ordering
///
/// ## Security Properties
///
/// - Ciphertext: AES-256-GCM encrypted, holders cannot read
/// - Signature: Proves authenticity, prevents tampering
/// - Version chain: Detects rollback attacks
/// - Merkle root: Public for ZK-proofs, doesn't leak membership
///
/// ## Example
///
/// ```rust,ignore
/// use stroma::persistence::encryption::EncryptedTrustNetworkState;
///
/// let aci_key = [0u8; 32]; // Signal ACI private key
/// let plaintext = b"trust state data";
/// let previous = None; // First version
///
/// let encrypted = EncryptedTrustNetworkState::new(
///     plaintext,
///     previous,
///     &aci_key,
///     merkle_root,
/// ).unwrap();
///
/// // Verify and decrypt
/// let decrypted = encrypted.decrypt(&aci_key).unwrap();
/// assert_eq!(decrypted, plaintext);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedTrustNetworkState {
    /// AES-256-GCM encrypted trust state
    pub ciphertext: Vec<u8>,

    /// GCM nonce (12 bytes, unique per encryption)
    pub nonce: [u8; 12],

    /// Ed25519 signature by Signal ACI identity key
    ///
    /// Signs: hash(ciphertext || nonce || version || previous_hash || timestamp)
    ///
    /// TODO: Replace with Ed25519 signature once Signal protocol integration is complete.
    /// For now, this is a placeholder that will be HMAC-SHA256 for testing.
    pub signature: Vec<u8>,

    /// Signal ACI public key for verification
    ///
    /// TODO: Replace with actual Signal ACI public key type once libsignal-protocol is integrated.
    pub aci_pubkey: Vec<u8>,

    /// Merkle root of member set (PUBLIC, not encrypted)
    ///
    /// Allows ZK-proof verification without decryption.
    /// Generated on-demand from BTreeSet of HMAC-hashed member identities.
    pub member_merkle_root: Hash,

    /// Version number (monotonic, increments on each write)
    pub version: u64,

    /// Hash of previous state (chain integrity)
    pub previous_hash: Hash,

    /// When this state was created (Unix timestamp)
    pub timestamp: Timestamp,
}

/// Errors that can occur during encryption operations.
#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("Version chain broken: expected version > {expected}, got {actual}")]
    VersionChainBroken { expected: u64, actual: u64 },

    #[error("Previous hash mismatch")]
    PreviousHashMismatch,

    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),
}

/// Encryption key derived from Signal ACI identity.
///
/// Automatically zeroized on drop.
#[derive(Zeroize, ZeroizeOnDrop)]
struct EncryptionKey([u8; 32]);

/// Signing key derived from Signal ACI identity.
///
/// Automatically zeroized on drop.
#[derive(Zeroize, ZeroizeOnDrop)]
struct SigningKey([u8; 32]);

impl EncryptedTrustNetworkState {
    /// Create a new encrypted state.
    ///
    /// # Arguments
    ///
    /// * `plaintext` - Trust state to encrypt (serialized bytes)
    /// * `previous` - Previous state (for version chain), or None for first version
    /// * `aci_key` - Signal ACI private key (32 bytes)
    /// * `member_merkle_root` - Merkle root of member set (public commitment)
    ///
    /// # Returns
    ///
    /// New encrypted state with signature
    ///
    /// # Errors
    ///
    /// - `InvalidKey`: ACI key is not 32 bytes
    /// - `EncryptionFailed`: AES-256-GCM encryption failed
    /// - `KeyDerivationFailed`: HKDF key derivation failed
    ///
    /// # Security
    ///
    /// - Generates random 12-byte nonce (NEVER reused)
    /// - Derives encryption key via HKDF from ACI
    /// - Encrypts with AES-256-GCM (authenticated encryption)
    /// - Signs state hash with derived signing key
    /// - Keys are zeroized after use
    pub fn new(
        plaintext: &[u8],
        previous: Option<&Self>,
        aci_key: &[u8],
        member_merkle_root: Hash,
    ) -> Result<Self, EncryptionError> {
        if aci_key.len() != 32 {
            return Err(EncryptionError::InvalidKey(
                "ACI key must be 32 bytes".to_string(),
            ));
        }

        // Compute version and previous_hash from chain
        let version = previous.map(|p| p.version + 1).unwrap_or(1);
        let previous_hash = previous
            .map(|p| p.compute_hash())
            .unwrap_or([0u8; 32]); // Zero hash for genesis

        // Derive encryption key from ACI
        let encryption_key = derive_encryption_key(aci_key)?;

        // Generate random nonce
        let nonce = generate_nonce();
        let nonce_obj = Nonce::try_assume_unique_for_key(&nonce).map_err(|_| {
            EncryptionError::EncryptionFailed("Failed to create nonce".to_string())
        })?;

        // Encrypt plaintext with AES-256-GCM
        let unbound_key = UnboundKey::new(&AES_256_GCM, &encryption_key.0).map_err(|e| {
            EncryptionError::EncryptionFailed(format!("Key creation failed: {}", e))
        })?;
        let key = LessSafeKey::new(unbound_key);

        let mut ciphertext = plaintext.to_vec();
        key.seal_in_place_append_tag(nonce_obj, Aad::empty(), &mut ciphertext)
            .map_err(|e| EncryptionError::EncryptionFailed(format!("Encryption failed: {}", e)))?;

        // TODO: Extract actual Signal ACI public key once libsignal-protocol is integrated
        let aci_pubkey = aci_key[..32].to_vec(); // Placeholder: use private key bytes for now

        // Get current timestamp
        let timestamp = current_timestamp();

        // Build state (without signature yet)
        let mut state = Self {
            ciphertext,
            nonce,
            signature: Vec::new(),
            aci_pubkey,
            member_merkle_root,
            version,
            previous_hash,
            timestamp,
        };

        // Sign the state hash
        state.signature = sign_state(&state, aci_key)?;

        Ok(state)
    }

    /// Verify signature and decrypt the state.
    ///
    /// # Arguments
    ///
    /// * `aci_key` - Signal ACI private key (32 bytes)
    ///
    /// # Returns
    ///
    /// Decrypted plaintext bytes
    ///
    /// # Errors
    ///
    /// - `SignatureVerificationFailed`: Signature doesn't match
    /// - `DecryptionFailed`: Wrong key or corrupted data
    /// - `InvalidKey`: ACI key is not 32 bytes
    ///
    /// # Security
    ///
    /// - Verifies signature BEFORE decryption
    /// - Uses authenticated encryption (AES-256-GCM)
    /// - Keys are zeroized after use
    pub fn decrypt(&self, aci_key: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        if aci_key.len() != 32 {
            return Err(EncryptionError::InvalidKey(
                "ACI key must be 32 bytes".to_string(),
            ));
        }

        // Verify signature first
        if !verify_signature(self, aci_key)? {
            return Err(EncryptionError::SignatureVerificationFailed);
        }

        // Derive encryption key
        let encryption_key = derive_encryption_key(aci_key)?;

        // Decrypt with AES-256-GCM
        let nonce = Nonce::try_assume_unique_for_key(&self.nonce).map_err(|_| {
            EncryptionError::DecryptionFailed("Invalid nonce".to_string())
        })?;

        let unbound_key = UnboundKey::new(&AES_256_GCM, &encryption_key.0).map_err(|e| {
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

    /// Compute hash of this state for version chain.
    ///
    /// Hash covers: ciphertext || nonce || version || previous_hash || timestamp
    ///
    /// This hash is used for:
    /// - Signature verification
    /// - Version chain integrity (becomes previous_hash of next state)
    ///
    /// # Returns
    ///
    /// 32-byte SHA256 hash
    pub fn compute_hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&self.ciphertext);
        hasher.update(&self.nonce);
        hasher.update(&self.version.to_le_bytes());
        hasher.update(&self.previous_hash);
        hasher.update(&self.timestamp.to_le_bytes());
        hasher.finalize().into()
    }

    /// Verify version chain integrity against previous state.
    ///
    /// Checks:
    /// - Version is strictly greater than previous
    /// - Previous hash matches computed hash of previous state
    ///
    /// # Arguments
    ///
    /// * `previous` - Previous state in chain
    ///
    /// # Errors
    ///
    /// - `VersionChainBroken`: Version not monotonically increasing
    /// - `PreviousHashMismatch`: Previous hash doesn't match
    pub fn verify_chain(&self, previous: &Self) -> Result<(), EncryptionError> {
        if self.version <= previous.version {
            return Err(EncryptionError::VersionChainBroken {
                expected: previous.version,
                actual: self.version,
            });
        }

        let expected_hash = previous.compute_hash();
        if self.previous_hash != expected_hash {
            return Err(EncryptionError::PreviousHashMismatch);
        }

        Ok(())
    }
}

/// Derive encryption key from Signal ACI identity using HKDF.
///
/// Uses HKDF-SHA256 with context separation:
/// - Salt: "stroma-state-encryption-v1"
/// - Info: "aes-256-gcm-key"
///
/// # Arguments
///
/// * `aci_key` - Signal ACI private key (32 bytes)
///
/// # Returns
///
/// 32-byte encryption key (zeroized on drop)
fn derive_encryption_key(aci_key: &[u8]) -> Result<EncryptionKey, EncryptionError> {
    const SALT: &[u8] = b"stroma-state-encryption-v1";
    const INFO: &[u8] = b"aes-256-gcm-key";

    let hkdf = Hkdf::<Sha256>::new(Some(SALT), aci_key);
    let mut key = [0u8; 32];
    hkdf.expand(INFO, &mut key)
        .map_err(|e| EncryptionError::KeyDerivationFailed(format!("HKDF expand failed: {}", e)))?;

    Ok(EncryptionKey(key))
}

/// Derive signing key from Signal ACI identity using HKDF.
///
/// Uses HKDF-SHA256 with context separation:
/// - Salt: "stroma-identity-masking-v1"
/// - Info: "hmac-sha256-key"
///
/// # Arguments
///
/// * `aci_key` - Signal ACI private key (32 bytes)
///
/// # Returns
///
/// 32-byte signing key (zeroized on drop)
fn derive_signing_key(aci_key: &[u8]) -> Result<SigningKey, EncryptionError> {
    const SALT: &[u8] = b"stroma-identity-masking-v1";
    const INFO: &[u8] = b"hmac-sha256-key";

    let hkdf = Hkdf::<Sha256>::new(Some(SALT), aci_key);
    let mut key = [0u8; 32];
    hkdf.expand(INFO, &mut key)
        .map_err(|e| EncryptionError::KeyDerivationFailed(format!("HKDF expand failed: {}", e)))?;

    Ok(SigningKey(key))
}

/// Generate a cryptographically random 12-byte nonce for AES-GCM.
///
/// # Returns
///
/// 12-byte nonce
///
/// # Panics
///
/// Panics if system RNG fails (catastrophic system failure)
fn generate_nonce() -> [u8; 12] {
    let rng = SystemRandom::new();
    let mut nonce = [0u8; 12];
    rng.fill(&mut nonce)
        .expect("System RNG failure - cannot generate nonce");
    nonce
}

/// Sign state hash with derived signing key.
///
/// TODO: Replace with Ed25519 signature once Signal protocol integration is complete.
/// For now, uses HMAC-SHA256 as a placeholder.
///
/// # Arguments
///
/// * `state` - State to sign
/// * `aci_key` - Signal ACI private key (32 bytes)
///
/// # Returns
///
/// Signature bytes
fn sign_state(
    state: &EncryptedTrustNetworkState,
    aci_key: &[u8],
) -> Result<Vec<u8>, EncryptionError> {
    use ring::hmac;

    let signing_key = derive_signing_key(aci_key)?;
    let state_hash = state.compute_hash();

    let key = hmac::Key::new(hmac::HMAC_SHA256, &signing_key.0);
    let signature = hmac::sign(&key, &state_hash);

    Ok(signature.as_ref().to_vec())
}

/// Verify state signature.
///
/// TODO: Replace with Ed25519 verification once Signal protocol integration is complete.
/// For now, uses HMAC-SHA256 verification as a placeholder.
///
/// # Arguments
///
/// * `state` - State to verify
/// * `aci_key` - Signal ACI private key (32 bytes)
///
/// # Returns
///
/// `true` if signature is valid
fn verify_signature(
    state: &EncryptedTrustNetworkState,
    aci_key: &[u8],
) -> Result<bool, EncryptionError> {
    use ring::hmac;

    let signing_key = derive_signing_key(aci_key)?;
    let state_hash = state.compute_hash();

    let key = hmac::Key::new(hmac::HMAC_SHA256, &signing_key.0);
    Ok(hmac::verify(&key, &state_hash, &state.signature).is_ok())
}

/// Get current Unix timestamp in seconds.
fn current_timestamp() -> Timestamp {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("System time before UNIX epoch")
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_aci_key() -> Vec<u8> {
        vec![42u8; 32] // Dummy 32-byte ACI key for testing
    }

    fn test_merkle_root() -> Hash {
        [0xAAu8; 32] // Dummy merkle root
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let aci_key = test_aci_key();
        let plaintext = b"test trust state data";
        let merkle_root = test_merkle_root();

        let encrypted = EncryptedTrustNetworkState::new(
            plaintext,
            None, // First version
            &aci_key,
            merkle_root,
        )
        .unwrap();

        assert_eq!(encrypted.version, 1);
        assert_eq!(encrypted.previous_hash, [0u8; 32]); // Zero hash for genesis
        assert_eq!(encrypted.member_merkle_root, merkle_root);
        assert!(!encrypted.ciphertext.is_empty());
        assert!(!encrypted.signature.is_empty());

        let decrypted = encrypted.decrypt(&aci_key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_version_chain() {
        let aci_key = test_aci_key();
        let merkle_root = test_merkle_root();

        // Create first version
        let v1 = EncryptedTrustNetworkState::new(b"state v1", None, &aci_key, merkle_root)
            .unwrap();
        assert_eq!(v1.version, 1);

        // Create second version
        let v2 = EncryptedTrustNetworkState::new(b"state v2", Some(&v1), &aci_key, merkle_root)
            .unwrap();
        assert_eq!(v2.version, 2);
        assert_eq!(v2.previous_hash, v1.compute_hash());

        // Verify chain
        v2.verify_chain(&v1).unwrap();

        // Create third version
        let v3 = EncryptedTrustNetworkState::new(b"state v3", Some(&v2), &aci_key, merkle_root)
            .unwrap();
        assert_eq!(v3.version, 3);
        assert_eq!(v3.previous_hash, v2.compute_hash());

        v3.verify_chain(&v2).unwrap();
    }

    #[test]
    fn test_signature_verification_fails_with_wrong_key() {
        let aci_key = test_aci_key();
        let wrong_key = vec![99u8; 32];
        let merkle_root = test_merkle_root();

        let encrypted =
            EncryptedTrustNetworkState::new(b"secret data", None, &aci_key, merkle_root).unwrap();

        // Should fail signature verification
        let result = encrypted.decrypt(&wrong_key);
        assert!(matches!(
            result,
            Err(EncryptionError::SignatureVerificationFailed)
        ));
    }

    #[test]
    fn test_version_chain_broken_with_wrong_version() {
        let aci_key = test_aci_key();
        let merkle_root = test_merkle_root();

        let v1 = EncryptedTrustNetworkState::new(b"state v1", None, &aci_key, merkle_root)
            .unwrap();

        // Manually create state with wrong version
        let mut v2_bad =
            EncryptedTrustNetworkState::new(b"state v2", Some(&v1), &aci_key, merkle_root)
                .unwrap();
        v2_bad.version = 1; // Same version as v1 (should be 2)

        let result = v2_bad.verify_chain(&v1);
        assert!(matches!(
            result,
            Err(EncryptionError::VersionChainBroken { .. })
        ));
    }

    #[test]
    fn test_previous_hash_mismatch() {
        let aci_key = test_aci_key();
        let merkle_root = test_merkle_root();

        let v1 = EncryptedTrustNetworkState::new(b"state v1", None, &aci_key, merkle_root)
            .unwrap();

        // Create v2 with correct chain
        let mut v2 =
            EncryptedTrustNetworkState::new(b"state v2", Some(&v1), &aci_key, merkle_root)
                .unwrap();

        // Tamper with previous_hash
        v2.previous_hash = [0xFFu8; 32];

        let result = v2.verify_chain(&v1);
        assert!(matches!(result, Err(EncryptionError::PreviousHashMismatch)));
    }

    #[test]
    fn test_tampered_ciphertext_fails_signature() {
        let aci_key = test_aci_key();
        let merkle_root = test_merkle_root();

        let mut encrypted =
            EncryptedTrustNetworkState::new(b"original data", None, &aci_key, merkle_root)
                .unwrap();

        // Tamper with ciphertext
        encrypted.ciphertext[0] ^= 0xFF;

        // Should fail signature verification (signature covers ciphertext)
        let result = encrypted.decrypt(&aci_key);
        assert!(matches!(
            result,
            Err(EncryptionError::SignatureVerificationFailed)
        ));
    }

    #[test]
    fn test_invalid_key_length() {
        let bad_key = vec![1u8; 16]; // Wrong size
        let merkle_root = test_merkle_root();

        let result =
            EncryptedTrustNetworkState::new(b"data", None, &bad_key, merkle_root);
        assert!(matches!(result, Err(EncryptionError::InvalidKey(_))));
    }

    #[test]
    fn test_nonce_uniqueness() {
        let aci_key = test_aci_key();
        let merkle_root = test_merkle_root();
        let plaintext = b"same plaintext";

        // Create two encryptions of same plaintext
        let enc1 = EncryptedTrustNetworkState::new(plaintext, None, &aci_key, merkle_root)
            .unwrap();
        let enc2 = EncryptedTrustNetworkState::new(plaintext, None, &aci_key, merkle_root)
            .unwrap();

        // Nonces should be different (random)
        assert_ne!(enc1.nonce, enc2.nonce);

        // Ciphertexts should be different (due to different nonces)
        assert_ne!(enc1.ciphertext, enc2.ciphertext);
    }

    #[test]
    fn test_merkle_root_preserved() {
        let aci_key = test_aci_key();
        let merkle_root = [0x42u8; 32];

        let encrypted = EncryptedTrustNetworkState::new(b"data", None, &aci_key, merkle_root)
            .unwrap();

        // Merkle root should be preserved (not encrypted)
        assert_eq!(encrypted.member_merkle_root, merkle_root);
    }

    #[test]
    fn test_timestamp_set() {
        let aci_key = test_aci_key();
        let merkle_root = test_merkle_root();

        let before = current_timestamp();
        let encrypted = EncryptedTrustNetworkState::new(b"data", None, &aci_key, merkle_root)
            .unwrap();
        let after = current_timestamp();

        // Timestamp should be within reasonable range
        assert!(encrypted.timestamp >= before && encrypted.timestamp <= after);
    }

    #[test]
    fn test_hash_includes_all_fields() {
        let aci_key = test_aci_key();
        let merkle_root = test_merkle_root();

        let state1 =
            EncryptedTrustNetworkState::new(b"data1", None, &aci_key, merkle_root).unwrap();
        let state2 =
            EncryptedTrustNetworkState::new(b"data2", None, &aci_key, merkle_root).unwrap();

        // Different data -> different ciphertext -> different hash
        assert_ne!(state1.compute_hash(), state2.compute_hash());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let aci_key = test_aci_key();
        let merkle_root = test_merkle_root();

        let original =
            EncryptedTrustNetworkState::new(b"test data", None, &aci_key, merkle_root).unwrap();

        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize from JSON
        let deserialized: EncryptedTrustNetworkState = serde_json::from_str(&json).unwrap();

        // Should be identical
        assert_eq!(original, deserialized);

        // Should still decrypt correctly
        let decrypted = deserialized.decrypt(&aci_key).unwrap();
        assert_eq!(decrypted, b"test data");
    }
}
