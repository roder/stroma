//! Unified key derivation from BIP-39 mnemonic seed
//!
//! This module centralizes all cryptographic key derivation in Stroma.
//! All keys are derived from the operator's 24-word BIP-39 mnemonic using
//! HKDF-SHA256 with domain separation.
//!
//! ## Key Hierarchy
//!
//! ```text
//! BIP-39 Mnemonic (24 words)
//!         │
//!         ▼
//! bip39::Mnemonic::to_seed("")  → [u8; 64]
//!         │
//!         ▼
//! HKDF-SHA256(salt="stroma-master-v1", seed)
//!         │
//!         ├─► HKDF expand("identity-masking") → identity_masking_key
//!         ├─► HKDF expand("voter-dedup")      → voter_pepper
//!         ├─► HKDF expand("chunk-encryption") → chunk_encryption_key
//!         └─► HKDF expand("chunk-signing")    → chunk_signing_key
//! ```
//!
//! ## Security Properties
//!
//! - **Mnemonic as Root**: The 24-word mnemonic is the single root of trust,
//!   stable across Signal re-registrations and device re-linking.
//! - **Domain Separation**: Each key purpose uses a unique HKDF info string.
//! - **Versioning**: Domain strings include "v1" suffix for future rotation.
//! - **Zeroization**: All keys implement `ZeroizeOnDrop`.
//!
//! ## Rotation Support
//!
//! The `key_epoch` field tracks which derivation generation is active.
//! Future rotation increments this and uses "v2" domain strings.
//! See `.beads/security-constraints.bead` for rotation strategy.
//!
//! ## References
//!
//! - Security constraints: `.beads/security-constraints.bead`
//! - Contract encryption: `.beads/contract-encryption.bead`

use bip39::Mnemonic;
use hkdf::Hkdf;
use sha2::Sha256;
use thiserror::Error;
use zeroize::Zeroize;

/// Domain separation salt for master key derivation (versioned for rotation)
const MASTER_SALT: &[u8] = b"stroma-master-v1";

/// HKDF info strings for purpose-specific key derivation
mod purposes {
    pub const IDENTITY_MASKING: &[u8] = b"identity-masking";
    pub const VOTER_DEDUP: &[u8] = b"voter-dedup";
    pub const CHUNK_ENCRYPTION: &[u8] = b"chunk-encryption";
    pub const CHUNK_SIGNING: &[u8] = b"chunk-signing";
    pub const STATE_ENCRYPTION: &[u8] = b"state-encryption";
    pub const STATE_SIGNING: &[u8] = b"state-signing";
}

/// Errors that can occur during key derivation
#[derive(Debug, Error)]
pub enum KeyringError {
    /// Invalid BIP-39 mnemonic
    #[error("Invalid mnemonic: {0}")]
    InvalidMnemonic(String),

    /// HKDF expansion failed (should never happen with valid lengths)
    #[error("Key derivation failed: {0}")]
    DerivationFailed(String),
}

/// Unified keyring holding all derived keys from the BIP-39 mnemonic.
///
/// This struct centralizes key management and ensures all keys are derived
/// from a single root of trust (the mnemonic) with proper domain separation.
///
/// # Security
///
/// - All fields implement `Zeroize` and are cleared on drop
/// - Keys are derived at construction time and never re-derived
/// - The mnemonic itself is NOT stored (only derived keys)
///
/// # Example
///
/// ```rust,ignore
/// use stroma::crypto::keyring::StromaKeyring;
///
/// let mnemonic = "abandon abandon abandon ... about"; // 24 words
/// let keyring = StromaKeyring::from_mnemonic(mnemonic)?;
///
/// // Use derived keys
/// let masked = mask_identity(signal_id, keyring.identity_masking_key());
/// ```
pub struct StromaKeyring {
    /// Key epoch (derivation generation), default 1
    /// Incremented on rotation, enables migration tracking
    epoch: u64,

    /// Key for HMAC-SHA256 identity masking
    identity_masking_key: [u8; 32],

    /// Pepper for voter deduplication HMAC
    voter_pepper: [u8; 32],

    /// Key for chunk AES-256-GCM encryption
    chunk_encryption_key: [u8; 32],

    /// Key for chunk Ed25519 signing
    chunk_signing_key: [u8; 32],

    /// Key for trust state AES-256-GCM encryption
    state_encryption_key: [u8; 32],

    /// Key for trust state signing
    state_signing_key: [u8; 32],
}

impl Drop for StromaKeyring {
    fn drop(&mut self) {
        // Zeroize all sensitive key material when the keyring is dropped.
        // This provides defense-in-depth against memory disclosure attacks.
        self.identity_masking_key.zeroize();
        self.voter_pepper.zeroize();
        self.chunk_encryption_key.zeroize();
        self.chunk_signing_key.zeroize();
        self.state_encryption_key.zeroize();
        self.state_signing_key.zeroize();
    }
}

impl StromaKeyring {
    /// Create a keyring from a BIP-39 mnemonic phrase.
    ///
    /// This is the primary constructor. It:
    /// 1. Parses and validates the mnemonic
    /// 2. Derives a 64-byte seed using BIP-39 `to_seed("")`
    /// 3. Uses HKDF-SHA256 to derive all purpose-specific keys
    ///
    /// # Arguments
    ///
    /// * `mnemonic` - A 24-word BIP-39 mnemonic phrase (space-separated)
    ///
    /// # Returns
    ///
    /// * `Ok(StromaKeyring)` - Keyring with all derived keys
    /// * `Err(KeyringError)` - If mnemonic is invalid or derivation fails
    ///
    /// # Security
    ///
    /// - The mnemonic string is NOT stored in the keyring
    /// - Caller should zeroize the mnemonic string after this call
    /// - All derived keys are zeroized on keyring drop
    pub fn from_mnemonic(mnemonic: &str) -> Result<Self, KeyringError> {
        // Parse and validate the BIP-39 mnemonic
        let parsed =
            Mnemonic::parse(mnemonic).map_err(|e| KeyringError::InvalidMnemonic(e.to_string()))?;

        // Derive 64-byte seed using BIP-39 standard (PBKDF2 with 2048 rounds)
        // Empty passphrase "" is standard for this use case
        let seed = parsed.to_seed("");

        // Create HKDF instance with master salt
        let hkdf = Hkdf::<Sha256>::new(Some(MASTER_SALT), &seed);

        // Derive all purpose-specific keys
        let identity_masking_key = Self::derive_key(&hkdf, purposes::IDENTITY_MASKING)?;
        let voter_pepper = Self::derive_key(&hkdf, purposes::VOTER_DEDUP)?;
        let chunk_encryption_key = Self::derive_key(&hkdf, purposes::CHUNK_ENCRYPTION)?;
        let chunk_signing_key = Self::derive_key(&hkdf, purposes::CHUNK_SIGNING)?;
        let state_encryption_key = Self::derive_key(&hkdf, purposes::STATE_ENCRYPTION)?;
        let state_signing_key = Self::derive_key(&hkdf, purposes::STATE_SIGNING)?;

        Ok(Self {
            epoch: 1,
            identity_masking_key,
            voter_pepper,
            chunk_encryption_key,
            chunk_signing_key,
            state_encryption_key,
            state_signing_key,
        })
    }

    /// Derive a 32-byte key using HKDF expand with the given info string.
    fn derive_key(hkdf: &Hkdf<Sha256>, info: &[u8]) -> Result<[u8; 32], KeyringError> {
        let mut key = [0u8; 32];
        hkdf.expand(info, &mut key)
            .map_err(|e| KeyringError::DerivationFailed(format!("{:?}", e)))?;
        Ok(key)
    }

    /// Get the key epoch (derivation generation).
    ///
    /// This is 1 for the initial derivation. Future rotation increments this.
    /// The epoch should be stored alongside hashes to enable migration.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Get the identity masking key for HMAC-SHA256.
    ///
    /// Use this with `mask_identity()` from the `identity` module.
    pub fn identity_masking_key(&self) -> &[u8; 32] {
        &self.identity_masking_key
    }

    /// Get the voter deduplication pepper for poll HMAC.
    ///
    /// Use this with `PollManager` for privacy-preserving vote deduplication.
    pub fn voter_pepper(&self) -> &[u8; 32] {
        &self.voter_pepper
    }

    /// Get the chunk encryption key for AES-256-GCM.
    ///
    /// Use this with the persistence module for chunk encryption.
    pub fn chunk_encryption_key(&self) -> &[u8; 32] {
        &self.chunk_encryption_key
    }

    /// Get the chunk signing key.
    ///
    /// Use this with the persistence module for chunk authentication.
    pub fn chunk_signing_key(&self) -> &[u8; 32] {
        &self.chunk_signing_key
    }

    /// Get the state encryption key for trust state AES-256-GCM.
    ///
    /// Use this with the persistence module for encrypted trust state.
    pub fn state_encryption_key(&self) -> &[u8; 32] {
        &self.state_encryption_key
    }

    /// Get the state signing key for trust state authentication.
    pub fn state_signing_key(&self) -> &[u8; 32] {
        &self.state_signing_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Standard BIP-39 test mnemonic (DO NOT use in production)
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    #[test]
    fn test_keyring_from_valid_mnemonic() {
        let keyring = StromaKeyring::from_mnemonic(TEST_MNEMONIC);
        assert!(keyring.is_ok(), "Should create keyring from valid mnemonic");

        let keyring = keyring.unwrap();
        assert_eq!(keyring.epoch(), 1, "Initial epoch should be 1");
    }

    #[test]
    fn test_keyring_from_invalid_mnemonic() {
        let result = StromaKeyring::from_mnemonic("invalid mnemonic words");
        assert!(result.is_err(), "Should reject invalid mnemonic");
    }

    #[test]
    fn test_keyring_determinism() {
        // Same mnemonic should always produce same keys
        let keyring1 = StromaKeyring::from_mnemonic(TEST_MNEMONIC).unwrap();
        let keyring2 = StromaKeyring::from_mnemonic(TEST_MNEMONIC).unwrap();

        assert_eq!(
            keyring1.identity_masking_key(),
            keyring2.identity_masking_key(),
            "Identity masking keys should match"
        );
        assert_eq!(
            keyring1.voter_pepper(),
            keyring2.voter_pepper(),
            "Voter peppers should match"
        );
        assert_eq!(
            keyring1.chunk_encryption_key(),
            keyring2.chunk_encryption_key(),
            "Chunk encryption keys should match"
        );
    }

    #[test]
    fn test_keyring_key_isolation() {
        let keyring = StromaKeyring::from_mnemonic(TEST_MNEMONIC).unwrap();

        // All keys should be different from each other
        assert_ne!(
            keyring.identity_masking_key(),
            keyring.voter_pepper(),
            "Identity and voter keys should differ"
        );
        assert_ne!(
            keyring.identity_masking_key(),
            keyring.chunk_encryption_key(),
            "Identity and chunk keys should differ"
        );
        assert_ne!(
            keyring.voter_pepper(),
            keyring.chunk_encryption_key(),
            "Voter and chunk keys should differ"
        );
    }

    #[test]
    fn test_different_mnemonics_produce_different_keys() {
        let mnemonic1 = TEST_MNEMONIC;
        // A different valid 24-word BIP-39 mnemonic (from BIP-39 test vectors)
        // Vector 18 (256-bit): all zeros entropy produces "abandon...about"
        // Vector 19 (256-bit): all ones entropy produces this sequence
        let mnemonic2 = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote";

        let keyring1 = StromaKeyring::from_mnemonic(mnemonic1).unwrap();
        let keyring2 = StromaKeyring::from_mnemonic(mnemonic2).unwrap();

        assert_ne!(
            keyring1.identity_masking_key(),
            keyring2.identity_masking_key(),
            "Different mnemonics should produce different keys"
        );
    }

    #[test]
    fn test_keys_are_32_bytes() {
        let keyring = StromaKeyring::from_mnemonic(TEST_MNEMONIC).unwrap();

        assert_eq!(keyring.identity_masking_key().len(), 32);
        assert_eq!(keyring.voter_pepper().len(), 32);
        assert_eq!(keyring.chunk_encryption_key().len(), 32);
        assert_eq!(keyring.chunk_signing_key().len(), 32);
        assert_eq!(keyring.state_encryption_key().len(), 32);
        assert_eq!(keyring.state_signing_key().len(), 32);
    }

    #[test]
    fn test_keys_are_non_zero() {
        let keyring = StromaKeyring::from_mnemonic(TEST_MNEMONIC).unwrap();

        // Keys should not be all zeros (would indicate derivation failure)
        assert_ne!(keyring.identity_masking_key(), &[0u8; 32]);
        assert_ne!(keyring.voter_pepper(), &[0u8; 32]);
        assert_ne!(keyring.chunk_encryption_key(), &[0u8; 32]);
    }
}
