//! Attestation Module for Cryptographic Receipts
//!
//! This module implements cryptographic attestations (receipts) from chunk holders
//! proving they possess assigned chunks. Without attestations, holders cannot prove
//! chunk possession.
//!
//! ## Design
//!
//! - **Attestation**: Holder's HMAC-SHA256 signature on chunk receipt
//! - **Verification**: Verifies holder's signature using their identity key
//! - **Integration**: Works with ReplicationHealth for tracking verified attestations
//!
//! ## Security Properties
//!
//! - Holders sign (owner || chunk_index || holder || timestamp)
//! - Signature verification prevents false attestations
//! - Timestamp prevents replay attacks (attestations expire)
//! - Each holder uses their own identity key for signing
//!
//! ## References
//!
//! - Persistence: docs/PERSISTENCE.md § Distribution
//! - Review: docs/todo/phase-2.5-review.md lines 58-77, 296-318

use crate::persistence::health::ReplicationHealth;
use ring::hmac;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Maximum age of an attestation in seconds (7 days)
///
/// Attestations older than this are considered stale and should not be accepted.
/// This prevents replay attacks and ensures attestations represent current state.
const MAX_ATTESTATION_AGE_SECS: u64 = 7 * 24 * 60 * 60;

/// Cryptographic attestation from a chunk holder.
///
/// An attestation is a cryptographic receipt proving that a holder
/// possesses a specific chunk. The holder signs the attestation with
/// their identity key.
///
/// # Example
///
/// ```ignore
/// // Holder creates attestation
/// let attestation = Attestation::create(
///     "owner-contract-hash",
///     chunk_index,
///     "holder-contract-hash",
///     &holder_identity_key,
/// )?;
///
/// // Owner verifies attestation
/// if attestation.verify(&holder_identity_key)? {
///     println!("Holder {} confirmed possession of chunk {}",
///              attestation.holder, attestation.chunk_index);
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attestation {
    /// Owner's contract hash (whose state this chunk belongs to)
    pub owner: String,

    /// Chunk index (0-based)
    pub chunk_index: u32,

    /// Holder's contract hash (who possesses this chunk)
    pub holder: String,

    /// Unix timestamp when attestation was created (seconds since epoch)
    pub timestamp: u64,

    /// HMAC-SHA256 signature (proves holder possesses chunk)
    pub signature: Vec<u8>,
}

/// Errors that can occur during attestation operations
#[derive(Debug, Error)]
pub enum AttestationError {
    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Attestation expired: age {age_secs}s exceeds maximum {max_age_secs}s")]
    AttestationExpired { age_secs: u64, max_age_secs: u64 },

    #[error("Invalid timestamp: {0}")]
    InvalidTimestamp(String),

    #[error("Invalid identity key: must be 32 bytes")]
    InvalidIdentityKey,

    #[error("System time error: {0}")]
    SystemTimeError(String),
}

impl Attestation {
    /// Create a new attestation with signature.
    ///
    /// The holder creates this attestation to prove they possess a chunk.
    /// The attestation is signed with the holder's identity key.
    ///
    /// # Message Format
    ///
    /// The signed message is: `owner || chunk_index || holder || timestamp`
    ///
    /// # Arguments
    ///
    /// * `owner` - Owner's contract hash
    /// * `chunk_index` - Which chunk (0-based)
    /// * `holder` - Holder's contract hash
    /// * `holder_identity_key` - Holder's identity key for signing (32 bytes)
    ///
    /// # Returns
    ///
    /// Signed attestation ready for transmission to owner
    ///
    /// # Errors
    ///
    /// - `InvalidIdentityKey`: Key is not 32 bytes
    /// - `SystemTimeError`: Cannot get current timestamp
    pub fn create(
        owner: &str,
        chunk_index: u32,
        holder: &str,
        holder_identity_key: &[u8],
    ) -> Result<Self, AttestationError> {
        if holder_identity_key.len() != 32 {
            return Err(AttestationError::InvalidIdentityKey);
        }

        // Get current timestamp
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AttestationError::SystemTimeError(e.to_string()))?
            .as_secs();

        // Sign: owner || chunk_index || holder || timestamp
        let signature =
            sign_attestation(owner, chunk_index, holder, timestamp, holder_identity_key);

        Ok(Self {
            owner: owner.to_string(),
            chunk_index,
            holder: holder.to_string(),
            timestamp,
            signature,
        })
    }

    /// Verify this attestation's signature.
    ///
    /// The owner calls this to verify that a holder's attestation is authentic
    /// and was signed by the claimed holder.
    ///
    /// # Arguments
    ///
    /// * `holder_identity_key` - Holder's identity key for verification (32 bytes)
    ///
    /// # Returns
    ///
    /// `Ok(())` if signature is valid and attestation is not expired
    ///
    /// # Errors
    ///
    /// - `SignatureVerificationFailed`: Signature doesn't match or key is wrong
    /// - `AttestationExpired`: Attestation is older than MAX_ATTESTATION_AGE_SECS
    /// - `InvalidIdentityKey`: Key is not 32 bytes
    /// - `SystemTimeError`: Cannot get current time
    pub fn verify(&self, holder_identity_key: &[u8]) -> Result<(), AttestationError> {
        if holder_identity_key.len() != 32 {
            return Err(AttestationError::InvalidIdentityKey);
        }

        // Check if attestation has expired
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AttestationError::SystemTimeError(e.to_string()))?
            .as_secs();

        let age_secs = now.saturating_sub(self.timestamp);
        if age_secs > MAX_ATTESTATION_AGE_SECS {
            return Err(AttestationError::AttestationExpired {
                age_secs,
                max_age_secs: MAX_ATTESTATION_AGE_SECS,
            });
        }

        // Verify signature
        if verify_attestation_signature(
            &self.owner,
            self.chunk_index,
            &self.holder,
            self.timestamp,
            &self.signature,
            holder_identity_key,
        ) {
            Ok(())
        } else {
            Err(AttestationError::SignatureVerificationFailed)
        }
    }

    /// Check if this attestation is expired.
    ///
    /// # Returns
    ///
    /// `true` if attestation is older than MAX_ATTESTATION_AGE_SECS
    pub fn is_expired(&self) -> bool {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|duration| {
                let now = duration.as_secs();
                let age = now.saturating_sub(self.timestamp);
                age > MAX_ATTESTATION_AGE_SECS
            })
            .unwrap_or(true) // Treat time errors as expired
    }

    /// Get age of this attestation in seconds.
    ///
    /// # Returns
    ///
    /// Age in seconds, or None if current time cannot be determined
    pub fn age_secs(&self) -> Option<u64> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|duration| {
                let now = duration.as_secs();
                now.saturating_sub(self.timestamp)
            })
    }
}

/// Sign an attestation message with HMAC-SHA256.
///
/// # Arguments
///
/// * `owner` - Owner's contract hash
/// * `chunk_index` - Chunk index
/// * `holder` - Holder's contract hash
/// * `timestamp` - Unix timestamp in seconds
/// * `identity_key` - Identity key for signing (32 bytes)
///
/// # Returns
///
/// HMAC-SHA256 signature bytes
fn sign_attestation(
    owner: &str,
    chunk_index: u32,
    holder: &str,
    timestamp: u64,
    identity_key: &[u8],
) -> Vec<u8> {
    let key = hmac::Key::new(hmac::HMAC_SHA256, identity_key);

    // Sign: owner || chunk_index || holder || timestamp
    let mut message = Vec::new();
    message.extend_from_slice(owner.as_bytes());
    message.extend_from_slice(&chunk_index.to_le_bytes());
    message.extend_from_slice(holder.as_bytes());
    message.extend_from_slice(&timestamp.to_le_bytes());

    let signature = hmac::sign(&key, &message);
    signature.as_ref().to_vec()
}

/// Verify an attestation's HMAC signature.
///
/// # Arguments
///
/// * `owner` - Owner's contract hash
/// * `chunk_index` - Chunk index
/// * `holder` - Holder's contract hash
/// * `timestamp` - Unix timestamp in seconds
/// * `signature` - Signature to verify
/// * `identity_key` - Identity key for verification (32 bytes)
///
/// # Returns
///
/// `true` if signature is valid, `false` otherwise
fn verify_attestation_signature(
    owner: &str,
    chunk_index: u32,
    holder: &str,
    timestamp: u64,
    signature: &[u8],
    identity_key: &[u8],
) -> bool {
    let key = hmac::Key::new(hmac::HMAC_SHA256, identity_key);

    // Reconstruct message: owner || chunk_index || holder || timestamp
    let mut message = Vec::new();
    message.extend_from_slice(owner.as_bytes());
    message.extend_from_slice(&chunk_index.to_le_bytes());
    message.extend_from_slice(holder.as_bytes());
    message.extend_from_slice(&timestamp.to_le_bytes());

    hmac::verify(&key, &message, signature).is_ok()
}

/// Record a verified attestation in the replication health tracker.
///
/// This is a convenience function that verifies an attestation and records
/// it in the health tracker if valid.
///
/// # Arguments
///
/// * `attestation` - The attestation to verify and record
/// * `holder_identity_key` - Holder's identity key for verification (32 bytes)
/// * `health` - Replication health tracker to update
///
/// # Returns
///
/// `Ok(true)` if attestation was verified and recorded successfully,
/// `Ok(false)` if attestation failed verification (recorded as failed),
/// `Err` if there's a system error
///
/// # Example
///
/// ```ignore
/// let attestation = receive_attestation_from_holder();
/// let mut health = ReplicationHealth::new();
///
/// match record_attestation(&attestation, &holder_key, &mut health) {
///     Ok(true) => println!("Attestation verified and recorded"),
///     Ok(false) => println!("Attestation failed verification"),
///     Err(e) => println!("Error: {}", e),
/// }
/// ```
pub fn record_attestation(
    attestation: &Attestation,
    holder_identity_key: &[u8],
    health: &mut ReplicationHealth,
) -> Result<bool, AttestationError> {
    match attestation.verify(holder_identity_key) {
        Ok(()) => {
            // Attestation verified successfully
            health.record_attestation(attestation.chunk_index, &attestation.holder, true);
            Ok(true)
        }
        Err(AttestationError::SignatureVerificationFailed) => {
            // Attestation failed verification - record as failed but don't error
            health.record_attestation(attestation.chunk_index, &attestation.holder, false);
            Ok(false)
        }
        Err(AttestationError::AttestationExpired { .. }) => {
            // Expired attestations are recorded as failed
            health.record_attestation(attestation.chunk_index, &attestation.holder, false);
            Ok(false)
        }
        Err(e) => {
            // Other errors (system errors, invalid keys) are propagated
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_identity_key() -> Vec<u8> {
        vec![42u8; 32]
    }

    #[test]
    fn test_create_attestation() {
        let identity_key = test_identity_key();
        let attestation = Attestation::create(
            "owner-contract-hash",
            5,
            "holder-contract-hash",
            &identity_key,
        )
        .unwrap();

        assert_eq!(attestation.owner, "owner-contract-hash");
        assert_eq!(attestation.chunk_index, 5);
        assert_eq!(attestation.holder, "holder-contract-hash");
        assert!(!attestation.signature.is_empty());
        assert!(attestation.timestamp > 0);
    }

    #[test]
    fn test_verify_valid_attestation() {
        let identity_key = test_identity_key();
        let attestation = Attestation::create(
            "owner-contract-hash",
            5,
            "holder-contract-hash",
            &identity_key,
        )
        .unwrap();

        // Verification should succeed with correct key
        assert!(attestation.verify(&identity_key).is_ok());
    }

    #[test]
    fn test_verify_wrong_key() {
        let identity_key = test_identity_key();
        let attestation = Attestation::create(
            "owner-contract-hash",
            5,
            "holder-contract-hash",
            &identity_key,
        )
        .unwrap();

        // Verification should fail with wrong key
        let wrong_key = vec![99u8; 32];
        assert!(matches!(
            attestation.verify(&wrong_key),
            Err(AttestationError::SignatureVerificationFailed)
        ));
    }

    #[test]
    fn test_verify_tampered_attestation() {
        let identity_key = test_identity_key();
        let mut attestation = Attestation::create(
            "owner-contract-hash",
            5,
            "holder-contract-hash",
            &identity_key,
        )
        .unwrap();

        // Tamper with chunk index
        attestation.chunk_index = 99;

        // Verification should fail
        assert!(matches!(
            attestation.verify(&identity_key),
            Err(AttestationError::SignatureVerificationFailed)
        ));
    }

    #[test]
    fn test_attestation_not_expired_when_fresh() {
        let identity_key = test_identity_key();
        let attestation = Attestation::create(
            "owner-contract-hash",
            5,
            "holder-contract-hash",
            &identity_key,
        )
        .unwrap();

        assert!(!attestation.is_expired());
        assert!(attestation.verify(&identity_key).is_ok());
    }

    #[test]
    fn test_attestation_expired_when_old() {
        let identity_key = test_identity_key();
        let mut attestation = Attestation::create(
            "owner-contract-hash",
            5,
            "holder-contract-hash",
            &identity_key,
        )
        .unwrap();

        // Set timestamp to 8 days ago (exceeds 7 day max)
        let eight_days_ago = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - (8 * 24 * 60 * 60);
        attestation.timestamp = eight_days_ago;

        // Recreate signature with old timestamp
        attestation.signature = sign_attestation(
            &attestation.owner,
            attestation.chunk_index,
            &attestation.holder,
            attestation.timestamp,
            &identity_key,
        );

        assert!(attestation.is_expired());
        assert!(matches!(
            attestation.verify(&identity_key),
            Err(AttestationError::AttestationExpired { .. })
        ));
    }

    #[test]
    fn test_invalid_identity_key_create() {
        let short_key = vec![1u8; 16]; // Only 16 bytes, not 32
        let result =
            Attestation::create("owner-contract-hash", 5, "holder-contract-hash", &short_key);

        assert!(matches!(result, Err(AttestationError::InvalidIdentityKey)));
    }

    #[test]
    fn test_invalid_identity_key_verify() {
        let identity_key = test_identity_key();
        let attestation = Attestation::create(
            "owner-contract-hash",
            5,
            "holder-contract-hash",
            &identity_key,
        )
        .unwrap();

        let short_key = vec![1u8; 16];
        assert!(matches!(
            attestation.verify(&short_key),
            Err(AttestationError::InvalidIdentityKey)
        ));
    }

    #[test]
    fn test_age_secs() {
        let identity_key = test_identity_key();
        let attestation = Attestation::create(
            "owner-contract-hash",
            5,
            "holder-contract-hash",
            &identity_key,
        )
        .unwrap();

        let age = attestation.age_secs();
        assert!(age.is_some());
        // Fresh attestation should be less than 1 second old
        assert!(age.unwrap() < 1);
    }

    #[test]
    fn test_different_holders_different_signatures() {
        let identity_key = test_identity_key();

        let attestation1 =
            Attestation::create("owner-contract-hash", 5, "holder-A", &identity_key).unwrap();

        let attestation2 =
            Attestation::create("owner-contract-hash", 5, "holder-B", &identity_key).unwrap();

        // Different holders should produce different signatures
        assert_ne!(attestation1.signature, attestation2.signature);
    }

    #[test]
    fn test_different_chunks_different_signatures() {
        let identity_key = test_identity_key();

        let attestation1 = Attestation::create(
            "owner-contract-hash",
            1,
            "holder-contract-hash",
            &identity_key,
        )
        .unwrap();

        let attestation2 = Attestation::create(
            "owner-contract-hash",
            2,
            "holder-contract-hash",
            &identity_key,
        )
        .unwrap();

        // Different chunks should produce different signatures
        assert_ne!(attestation1.signature, attestation2.signature);
    }

    #[test]
    fn test_record_attestation_success() {
        let identity_key = test_identity_key();
        let attestation = Attestation::create(
            "owner-contract-hash",
            5,
            "holder-contract-hash",
            &identity_key,
        )
        .unwrap();

        let mut health = ReplicationHealth::new();
        health.update_total_chunks(10);

        // Record valid attestation
        let result = record_attestation(&attestation, &identity_key, &mut health).unwrap();
        assert!(result); // Should return true for successful verification

        // Check that it was recorded as confirmed
        assert_eq!(health.confirmed_replicas(5), 1);
    }

    #[test]
    fn test_record_attestation_wrong_key() {
        let identity_key = test_identity_key();
        let attestation = Attestation::create(
            "owner-contract-hash",
            5,
            "holder-contract-hash",
            &identity_key,
        )
        .unwrap();

        let mut health = ReplicationHealth::new();
        health.update_total_chunks(10);

        // Try to record with wrong key
        let wrong_key = vec![99u8; 32];
        let result = record_attestation(&attestation, &wrong_key, &mut health).unwrap();
        assert!(!result); // Should return false for failed verification

        // Check that it was recorded as failed
        assert_eq!(health.confirmed_replicas(5), 0);
    }

    #[test]
    fn test_record_attestation_expired() {
        let identity_key = test_identity_key();
        let mut attestation = Attestation::create(
            "owner-contract-hash",
            5,
            "holder-contract-hash",
            &identity_key,
        )
        .unwrap();

        // Set timestamp to 8 days ago
        let eight_days_ago = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - (8 * 24 * 60 * 60);
        attestation.timestamp = eight_days_ago;

        // Recreate signature with old timestamp
        attestation.signature = sign_attestation(
            &attestation.owner,
            attestation.chunk_index,
            &attestation.holder,
            attestation.timestamp,
            &identity_key,
        );

        let mut health = ReplicationHealth::new();
        health.update_total_chunks(10);

        // Try to record expired attestation
        let result = record_attestation(&attestation, &identity_key, &mut health).unwrap();
        assert!(!result); // Should return false for expired attestation

        // Check that it was recorded as failed
        assert_eq!(health.confirmed_replicas(5), 0);
    }

    #[test]
    fn test_record_multiple_attestations() {
        let identity_key = test_identity_key();
        let mut health = ReplicationHealth::new();
        health.update_total_chunks(10);

        // Record attestations from multiple holders for same chunk
        for i in 0..3 {
            let holder = format!("holder-{}", i);
            let attestation =
                Attestation::create("owner-contract-hash", 5, &holder, &identity_key).unwrap();

            let result = record_attestation(&attestation, &identity_key, &mut health).unwrap();
            assert!(result);
        }

        // Should have 3 confirmed replicas for chunk 5
        assert_eq!(health.confirmed_replicas(5), 3);
    }

    #[test]
    fn test_record_attestation_invalid_key() {
        let identity_key = test_identity_key();
        let attestation = Attestation::create(
            "owner-contract-hash",
            5,
            "holder-contract-hash",
            &identity_key,
        )
        .unwrap();

        let mut health = ReplicationHealth::new();
        health.update_total_chunks(10);

        // Try to record with invalid key (wrong length)
        let invalid_key = vec![99u8; 16]; // Only 16 bytes, not 32
        let result = record_attestation(&attestation, &invalid_key, &mut health);

        // Should return error (not Ok(false))
        assert!(matches!(result, Err(AttestationError::InvalidIdentityKey)));
    }

    // Property-based tests for cryptographic properties
    #[cfg(test)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// Property: Same inputs always produce same signature (HMAC determinism)
            #[test]
            fn prop_deterministic_signature(
                owner in "[a-z]{10,20}",
                chunk_index in 0u32..1000,
                holder in "[a-z]{10,20}",
                key_byte in 0u8..=255,
            ) {
                let identity_key = vec![key_byte; 32];

                // Create two attestations with same inputs
                let att1 = Attestation::create(&owner, chunk_index, &holder, &identity_key).unwrap();
                // Wait a tiny bit to ensure different timestamps
                std::thread::sleep(std::time::Duration::from_millis(1));
                let att2 = Attestation::create(&owner, chunk_index, &holder, &identity_key).unwrap();

                // Signatures should be different due to different timestamps
                // But verification should work for both
                prop_assert!(att1.verify(&identity_key).is_ok());
                prop_assert!(att2.verify(&identity_key).is_ok());

                // With same timestamp, signatures should match
                let mut att3 = att1.clone();
                att3.signature = sign_attestation(&att3.owner, att3.chunk_index, &att3.holder, att3.timestamp, &identity_key);
                prop_assert_eq!(att1.signature, att3.signature);
            }

            /// Property: Different keys produce different signatures (key isolation)
            #[test]
            fn prop_key_isolation(
                owner in "[a-z]{10,20}",
                chunk_index in 0u32..1000,
                holder in "[a-z]{10,20}",
                key1_byte in 0u8..=255,
                key2_byte in 0u8..=255,
            ) {
                prop_assume!(key1_byte != key2_byte);

                let key1 = vec![key1_byte; 32];
                let key2 = vec![key2_byte; 32];

                // Same timestamp for fair comparison
                let timestamp = 1234567890;
                let sig1 = sign_attestation(&owner, chunk_index, &holder, timestamp, &key1);
                let sig2 = sign_attestation(&owner, chunk_index, &holder, timestamp, &key2);

                // Different keys must produce different signatures
                prop_assert_ne!(sig1, sig2);
            }

            /// Property: Tampering detection - any change invalidates signature
            #[test]
            fn prop_tamper_detection(
                owner in "[a-z]{10,20}",
                chunk_index in 0u32..1000,
                holder in "[a-z]{10,20}",
                key_byte in 0u8..=255,
            ) {
                let identity_key = vec![key_byte; 32];
                let mut att = Attestation::create(&owner, chunk_index, &holder, &identity_key).unwrap();

                // Original should verify
                prop_assert!(att.verify(&identity_key).is_ok());

                // Tamper with chunk index
                let original_chunk = att.chunk_index;
                att.chunk_index = original_chunk.wrapping_add(1);
                prop_assert!(att.verify(&identity_key).is_err());

                // Restore and tamper with holder
                att.chunk_index = original_chunk;
                att.holder = format!("tampered-{}", att.holder);
                prop_assert!(att.verify(&identity_key).is_err());
            }

            /// Property: Verification succeeds with correct key, fails with wrong key
            #[test]
            fn prop_correct_key_required(
                owner in "[a-z]{10,20}",
                chunk_index in 0u32..1000,
                holder in "[a-z]{10,20}",
                correct_key_byte in 0u8..=255,
                wrong_key_byte in 0u8..=255,
            ) {
                prop_assume!(correct_key_byte != wrong_key_byte);

                let correct_key = vec![correct_key_byte; 32];
                let wrong_key = vec![wrong_key_byte; 32];

                let att = Attestation::create(&owner, chunk_index, &holder, &correct_key).unwrap();

                // Correct key should verify
                prop_assert!(att.verify(&correct_key).is_ok());

                // Wrong key should fail
                prop_assert!(att.verify(&wrong_key).is_err());
            }
        }
    }
}
