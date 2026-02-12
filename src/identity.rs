//! HMAC-based identity masking with mnemonic-derived keys
//!
//! This module implements cryptographic identity masking following the security
//! constraints defined in `.beads/security-constraints.bead`.
//!
//! # Security Properties
//!
//! - **Collision Resistance**: Different Signal IDs produce different hashes
//! - **Determinism**: Same Signal ID + masking key always produces same hash
//! - **Key Isolation**: Different masking keys produce different hashes for same Signal ID
//! - **Immediate Zeroization**: Sensitive data cleared from memory after use
//!
//! # Key Derivation
//!
//! The masking key is derived from the operator's BIP-39 mnemonic via HKDF
//! in `StromaKeyring`. This ensures hash stability across Signal re-registrations.
//!
//! See: `crypto::keyring::StromaKeyring` for the key derivation hierarchy.
//!
//! # Required Pattern (from security-constraints.bead § 1)
//!
//! Uses HMAC-SHA256 with HKDF-derived keys from the mnemonic seed.
//! Do NOT use any other key source - always use `keyring.identity_masking_key()`.

use crate::freenet::contract::MemberHash;
use hkdf::Hkdf;
use ring::hmac;
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// A 32-byte cryptographic hash representing a masked identity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaskedIdentity([u8; 32]);

impl MaskedIdentity {
    /// Creates a MaskedIdentity from a 32-byte array
    pub fn from_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), 32, "MaskedIdentity must be 32 bytes");
        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Self(arr)
    }

    /// Returns the raw bytes of the masked identity
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Convert MaskedIdentity to freenet::contract::MemberHash
///
/// This enables seamless integration between the identity module's strong
/// HMAC-based masking and the Freenet contract's member hash storage.
impl From<MaskedIdentity> for MemberHash {
    fn from(masked: MaskedIdentity) -> Self {
        MemberHash::from_bytes(masked.as_bytes())
    }
}

/// Derives an HMAC key from the identity masking key
///
/// This function implements the key derivation pattern required by
/// security-constraints.bead § 1. It uses HKDF-SHA256 to derive a
/// 32-byte HMAC key from the masking key.
///
/// # Arguments
///
/// * `masking_key` - The identity masking key from `StromaKeyring::identity_masking_key()`
///
/// # Returns
///
/// A 32-byte HMAC key derived for identity masking
///
/// # Security
///
/// - Uses HKDF-SHA256 with salt "stroma-identity-masking-v1"
/// - Derives key material for HMAC-SHA256
/// - Key is returned on stack (caller must zeroize if needed)
///
/// # Note
///
/// The `masking_key` should be obtained from `StromaKeyring::identity_masking_key()`,
/// which is derived from the operator's BIP-39 mnemonic. Do NOT use the Signal ACI
/// private key directly.
fn derive_identity_masking_key(masking_key: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(b"stroma-identity-masking-v1"), masking_key);
    let mut key = [0u8; 32];
    hk.expand(b"hmac-sha256-key", &mut key)
        .expect("HKDF expand should never fail with valid length");
    key
}

/// Masks a Signal ID using HMAC with a mnemonic-derived key
///
/// This is the primary identity masking function. It takes a cleartext Signal ID
/// and a masking key (from `StromaKeyring`), derives an HMAC key, and produces
/// a masked identity hash.
///
/// # Arguments
///
/// * `signal_id` - The cleartext Signal ID to mask
/// * `masking_key` - The identity masking key from `StromaKeyring::identity_masking_key()`
///
/// # Returns
///
/// A `MaskedIdentity` containing the HMAC-SHA256 hash. This can be converted to
/// `MemberHash` for Freenet contract storage via `From<MaskedIdentity>`.
///
/// # Security Properties
///
/// - **Determinism**: Same inputs always produce same output
/// - **Key Isolation**: Different masking keys produce different hashes
/// - **Collision Resistance**: Different Signal IDs produce different hashes
/// - **One-way**: Cannot reverse hash to recover Signal ID
///
/// # Example
///
/// ```rust,ignore
/// use stroma::crypto::keyring::StromaKeyring;
/// use stroma::identity::mask_identity;
/// use stroma::freenet::contract::MemberHash;
///
/// let keyring = StromaKeyring::from_mnemonic(mnemonic)?;
/// let signal_id = "alice@signal.org";
/// let masked = mask_identity(signal_id, keyring.identity_masking_key());
///
/// // Convert to MemberHash for Freenet contract
/// let member_hash: MemberHash = masked.into();
/// ```
pub fn mask_identity(signal_id: &str, masking_key: &[u8]) -> MaskedIdentity {
    // Derive HMAC key from the masking key
    let hmac_key_bytes = derive_identity_masking_key(masking_key);
    let key = hmac::Key::new(hmac::HMAC_SHA256, &hmac_key_bytes);
    let tag = hmac::sign(&key, signal_id.as_bytes());

    // Note: hmac_key_bytes is on stack and will be overwritten
    // For extra security, caller should zeroize signal_id after use

    MaskedIdentity::from_bytes(tag.as_ref())
}

/// A wrapper for sensitive data that requires zeroization
///
/// This struct demonstrates the required pattern for handling cleartext Signal IDs
/// and other sensitive data. It automatically zeroizes its contents on drop.
///
/// # Example
///
/// ```rust,ignore
/// use stroma::identity::{SensitiveIdentityData, mask_identity};
/// use stroma::crypto::keyring::StromaKeyring;
///
/// let mut data = SensitiveIdentityData::new(
///     "alice@signal.org".to_string(),
/// );
///
/// let keyring = StromaKeyring::from_mnemonic(mnemonic)?;
/// let masked = data.process(keyring.identity_masking_key());
///
/// // data.signal_id is now zeroized
/// ```
#[derive(ZeroizeOnDrop)]
pub struct SensitiveIdentityData {
    signal_id: String,
}

impl SensitiveIdentityData {
    /// Creates a new SensitiveIdentityData wrapper
    pub fn new(signal_id: String) -> Self {
        Self { signal_id }
    }

    /// Processes the sensitive data and zeroizes it
    ///
    /// After calling this method, the signal_id field is zeroized and
    /// should not be used again.
    ///
    /// # Arguments
    ///
    /// * `masking_key` - The identity masking key from `StromaKeyring::identity_masking_key()`
    pub fn process(&mut self, masking_key: &[u8]) -> MaskedIdentity {
        let hash = mask_identity(&self.signal_id, masking_key);
        self.signal_id.zeroize();
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Test: Same inputs produce same output (determinism)
    #[test]
    fn test_determinism() {
        let masking_key = b"test-masking-key-32-bytes-long!!";
        let signal_id = "alice@signal.org";

        let hash1 = mask_identity(signal_id, masking_key);
        let hash2 = mask_identity(signal_id, masking_key);

        assert_eq!(hash1, hash2, "Same inputs must produce same output");
    }

    /// Test: Different Signal IDs produce different hashes (collision resistance)
    #[test]
    fn test_collision_resistance() {
        let masking_key = b"test-masking-key-32-bytes-long!!";
        let signal_id1 = "alice@signal.org";
        let signal_id2 = "bob@signal.org";

        let hash1 = mask_identity(signal_id1, masking_key);
        let hash2 = mask_identity(signal_id2, masking_key);

        assert_ne!(
            hash1, hash2,
            "Different Signal IDs must produce different hashes"
        );
    }

    /// Test: Different masking keys produce different hashes (key isolation)
    #[test]
    fn test_key_isolation() {
        let masking_key1 = b"masking-key-1-32-bytes-padding!!";
        let masking_key2 = b"masking-key-2-32-bytes-padding!!";
        let signal_id = "alice@signal.org";

        let hash1 = mask_identity(signal_id, masking_key1);
        let hash2 = mask_identity(signal_id, masking_key2);

        assert_ne!(
            hash1, hash2,
            "Same Signal ID with different masking keys must produce different hashes"
        );
    }

    /// Test: Zeroization of sensitive data
    #[test]
    fn test_zeroization() {
        let masking_key = b"test-masking-key-32-bytes-long!!";
        let mut data = SensitiveIdentityData::new("alice@signal.org".to_string());

        let _masked = data.process(masking_key);

        // After processing, signal_id should be zeroized
        // We can't directly test this in safe Rust, but the ZeroizeOnDrop
        // trait ensures it happens on drop
        assert_eq!(
            data.signal_id.len(),
            0,
            "Signal ID should be zeroized after processing"
        );
    }

    /// Test: MaskedIdentity byte operations
    #[test]
    fn test_masked_identity_bytes() {
        let masking_key = b"test-masking-key-32-bytes-long!!";
        let signal_id = "alice@signal.org";

        let masked = mask_identity(signal_id, masking_key);
        let bytes = masked.as_bytes();

        // Verify we can round-trip through bytes
        let masked2 = MaskedIdentity::from_bytes(bytes);
        assert_eq!(
            masked, masked2,
            "MaskedIdentity should round-trip through bytes"
        );

        // Verify byte length
        assert_eq!(bytes.len(), 32, "MaskedIdentity should be 32 bytes");
    }

    /// Test: Hash and equality properties
    #[test]
    fn test_masked_identity_hash_eq() {
        let masking_key = b"test-masking-key-32-bytes-long!!";
        let signal_id = "alice@signal.org";

        let masked1 = mask_identity(signal_id, masking_key);
        let masked2 = mask_identity(signal_id, masking_key);

        // Test equality
        assert_eq!(masked1, masked2);

        // Test that we can use it in a HashMap (requires Hash + Eq)
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(masked1, "alice");
        assert_eq!(map.get(&masked2), Some(&"alice"));
    }

    /// Test: MaskedIdentity converts to MemberHash
    #[test]
    fn test_masked_identity_to_member_hash() {
        let masking_key = b"test-masking-key-32-bytes-long!!";
        let signal_id = "alice@signal.org";

        let masked = mask_identity(signal_id, masking_key);
        let member_hash: MemberHash = masked.into();

        // Bytes should be identical
        assert_eq!(
            masked.as_bytes(),
            member_hash.as_bytes(),
            "MemberHash should have same bytes as MaskedIdentity"
        );
    }

    // Property test: Determinism with random inputs
    proptest! {
        #[test]
        fn prop_determinism(signal_id in ".*", key_seed in 0u64..u64::MAX) {
            // Generate deterministic key from seed using hash-based derivation
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(b"proptest-masking-key");
            hasher.update(key_seed.to_le_bytes());
            let masking_key: [u8; 32] = hasher.finalize().into();

            let hash1 = mask_identity(&signal_id, &masking_key);
            let hash2 = mask_identity(&signal_id, &masking_key);

            prop_assert_eq!(hash1, hash2, "Determinism: same inputs must produce same output");
        }
    }

    // Property test: Collision resistance with random Signal IDs
    proptest! {
        #[test]
        fn prop_collision_resistance(
            signal_id1 in "[a-z0-9._%+-]+@[a-z0-9.-]+\\.[a-z]{2,}",
            signal_id2 in "[a-z0-9._%+-]+@[a-z0-9.-]+\\.[a-z]{2,}",
            key_seed in 0u64..u64::MAX,
        ) {
            prop_assume!(signal_id1 != signal_id2);

            // Generate deterministic key from seed using hash-based derivation
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(b"proptest-masking-key");
            hasher.update(key_seed.to_le_bytes());
            let masking_key: [u8; 32] = hasher.finalize().into();

            let hash1 = mask_identity(&signal_id1, &masking_key);
            let hash2 = mask_identity(&signal_id2, &masking_key);

            prop_assert_ne!(hash1, hash2, "Collision resistance: different Signal IDs must produce different hashes");
        }
    }

    // Property test: Key isolation with random masking keys
    proptest! {
        #[test]
        fn prop_key_isolation(
            signal_id in "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}|\\+[0-9]{10,15}",  // Valid Signal ID format
            key1_seed in 0u64..u64::MAX,
            key2_seed in 0u64..u64::MAX,
        ) {
            prop_assume!(key1_seed != key2_seed);
            prop_assume!(!signal_id.is_empty());

            // Generate two different deterministic keys from seeds using proper hash-based derivation
            use sha2::{Sha256, Digest};

            let mut hasher1 = Sha256::new();
            hasher1.update(b"proptest-masking-key");
            hasher1.update(key1_seed.to_le_bytes());
            let masking_key1: [u8; 32] = hasher1.finalize().into();

            let mut hasher2 = Sha256::new();
            hasher2.update(b"proptest-masking-key");
            hasher2.update(key2_seed.to_le_bytes());
            let masking_key2: [u8; 32] = hasher2.finalize().into();

            let hash1 = mask_identity(&signal_id, &masking_key1);
            let hash2 = mask_identity(&signal_id, &masking_key2);

            prop_assert_ne!(hash1, hash2, "Key isolation: different masking keys must produce different hashes");
        }
    }
}
