//! Chunk storage and encryption for Stroma's Reciprocal Persistence Network.
//!
//! This module implements encryption, chunking, and reassembly of bot state.
//!
//! ## Design
//!
//! - **Encryption**: AES-256-GCM with key derived from Signal ACI via HKDF
//! - **Chunk Size**: 64KB (validated in Q12 spike)
//! - **Replication**: 1 local + 2 remote replicas per chunk
//! - **Signature**: HMAC-SHA256 using ACI identity key for verification
//!
//! ## Security Properties
//!
//! - Chunks are encrypted BEFORE distribution (holders cannot read)
//! - Need ALL chunks + ACI key to reconstruct (single chunk = useless ciphertext)
//! - Signature verification prevents tampering
//! - Key derivation uses HKDF-SHA256 from Signal ACI
//!
//! ## References
//!
//! - Architecture: docs/PERSISTENCE.md § Recovery
//! - Security: .beads/security-constraints.bead

use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::hmac;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Chunk size: 64KB (Q12 validated - optimal balance)
pub const CHUNK_SIZE: usize = 64 * 1024;

/// Encryption context string for HKDF
const ENCRYPTION_CONTEXT: &[u8] = b"stroma-persistence-v1-encryption";

/// Signing context string for HKDF
const SIGNING_CONTEXT: &[u8] = b"stroma-persistence-v1-signing";

/// A single encrypted chunk of bot state.
///
/// Chunks are distributed to remote holders for persistence.
/// Each chunk contains a portion of the encrypted state plus metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chunk {
    /// Owner's contract hash (whose state this chunk belongs to)
    pub owner: String,

    /// Chunk index (0-based, for reassembly ordering)
    pub index: u32,

    /// Encrypted chunk data (64KB max, last chunk may be smaller)
    pub data: Vec<u8>,

    /// HMAC-SHA256 signature (proves authenticity)
    pub signature: Vec<u8>,

    /// Nonce for AES-256-GCM decryption (12 bytes)
    pub nonce: Vec<u8>,
}

/// Errors that can occur during chunk operations
#[derive(Debug, Error)]
pub enum ChunkError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Invalid chunk: {0}")]
    InvalidChunk(String),

    #[error("Missing chunks: expected {expected}, got {actual}")]
    MissingChunks { expected: usize, actual: usize },

    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),
}

/// Encrypt and chunk bot state for distribution.
///
/// # Flow
///
/// 1. Derive encryption key from ACI via HKDF
/// 2. Encrypt full state with AES-256-GCM
/// 3. Split ciphertext into 64KB chunks
/// 4. Sign each chunk with HMAC-SHA256
/// 5. Return chunks ready for distribution
///
/// # Arguments
///
/// * `owner` - Owner's contract hash
/// * `state` - Bot state to encrypt (plaintext bytes)
/// * `aci_key` - Signal ACI key (32 bytes)
///
/// # Returns
///
/// Vector of encrypted chunks with signatures
///
/// # Security
///
/// - Uses AES-256-GCM with random nonce per encryption
/// - Derives encryption key via HKDF-SHA256 from ACI
/// - Signs each chunk with derived HMAC key
/// - Chunks are useless without ALL chunks + ACI key
pub fn encrypt_and_chunk(
    owner: &str,
    state: &[u8],
    aci_key: &[u8],
) -> Result<Vec<Chunk>, ChunkError> {
    if aci_key.len() != 32 {
        return Err(ChunkError::KeyDerivationFailed(
            "ACI key must be 32 bytes".to_string(),
        ));
    }

    // Derive encryption key from ACI using HKDF
    let encryption_key = derive_key(aci_key, ENCRYPTION_CONTEXT)?;
    let signing_key = derive_key(aci_key, SIGNING_CONTEXT)?;

    // Generate random nonce for this encryption session
    let nonce_bytes = generate_nonce();
    let nonce = Nonce::try_assume_unique_for_key(&nonce_bytes)
        .map_err(|_| ChunkError::EncryptionFailed("Failed to create nonce".to_string()))?;

    // Encrypt the full state
    let unbound_key = UnboundKey::new(&AES_256_GCM, &encryption_key)
        .map_err(|e| ChunkError::EncryptionFailed(format!("Key creation failed: {}", e)))?;
    let key = LessSafeKey::new(unbound_key);

    let mut ciphertext = state.to_vec();
    key.seal_in_place_append_tag(nonce, Aad::empty(), &mut ciphertext)
        .map_err(|e| ChunkError::EncryptionFailed(format!("Encryption failed: {}", e)))?;

    // Split into 64KB chunks
    let num_chunks = (ciphertext.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;
    let mut chunks = Vec::with_capacity(num_chunks);

    for (index, chunk_data) in ciphertext.chunks(CHUNK_SIZE).enumerate() {
        // Sign this chunk
        let signature = sign_chunk(owner, index as u32, chunk_data, &signing_key);

        chunks.push(Chunk {
            owner: owner.to_string(),
            index: index as u32,
            data: chunk_data.to_vec(),
            signature,
            nonce: nonce_bytes.to_vec(),
        });
    }

    Ok(chunks)
}

/// Decrypt and reassemble chunks into original state.
///
/// # Flow
///
/// 1. Sort chunks by index
/// 2. Verify all chunks present (no gaps)
/// 3. Verify signature on each chunk
/// 4. Concatenate chunk data
/// 5. Derive encryption key from ACI
/// 6. Decrypt ciphertext with AES-256-GCM
/// 7. Return plaintext state
///
/// # Arguments
///
/// * `chunks` - All chunks for this state (order doesn't matter)
/// * `aci_key` - Signal ACI key for decryption (32 bytes)
///
/// # Returns
///
/// Decrypted state bytes (plaintext)
///
/// # Errors
///
/// - `MissingChunks`: Not all chunks present
/// - `SignatureVerificationFailed`: Chunk tampered with
/// - `DecryptionFailed`: Wrong key or corrupted data
pub fn decrypt_and_reassemble(chunks: &[Chunk], aci_key: &[u8]) -> Result<Vec<u8>, ChunkError> {
    if aci_key.len() != 32 {
        return Err(ChunkError::KeyDerivationFailed(
            "ACI key must be 32 bytes".to_string(),
        ));
    }

    if chunks.is_empty() {
        return Err(ChunkError::InvalidChunk("No chunks provided".to_string()));
    }

    // Derive keys
    let encryption_key = derive_key(aci_key, ENCRYPTION_CONTEXT)?;
    let signing_key = derive_key(aci_key, SIGNING_CONTEXT)?;

    // Sort chunks by index
    let mut sorted_chunks = chunks.to_vec();
    sorted_chunks.sort_by_key(|c| c.index);

    // Verify all chunks present (no gaps)
    let expected_chunks = sorted_chunks.len();
    for (i, chunk) in sorted_chunks.iter().enumerate() {
        if chunk.index != i as u32 {
            return Err(ChunkError::MissingChunks {
                expected: expected_chunks,
                actual: i,
            });
        }
    }

    // Get owner and nonce from first chunk (all chunks have same owner/nonce)
    let owner = &sorted_chunks[0].owner;
    let nonce_bytes = &sorted_chunks[0].nonce;

    // Verify signature on each chunk
    for chunk in &sorted_chunks {
        if !verify_chunk_signature(
            owner,
            chunk.index,
            &chunk.data,
            &chunk.signature,
            &signing_key,
        ) {
            return Err(ChunkError::SignatureVerificationFailed);
        }
    }

    // Concatenate all chunk data
    let ciphertext: Vec<u8> = sorted_chunks
        .iter()
        .flat_map(|c| c.data.iter())
        .copied()
        .collect();

    // Decrypt
    let nonce = Nonce::try_assume_unique_for_key(nonce_bytes)
        .map_err(|_| ChunkError::DecryptionFailed("Invalid nonce".to_string()))?;

    let unbound_key = UnboundKey::new(&AES_256_GCM, &encryption_key)
        .map_err(|e| ChunkError::DecryptionFailed(format!("Key creation failed: {}", e)))?;
    let key = LessSafeKey::new(unbound_key);

    let mut plaintext = ciphertext;
    key.open_in_place(nonce, Aad::empty(), &mut plaintext)
        .map_err(|e| ChunkError::DecryptionFailed(format!("Decryption failed: {}", e)))?;

    // Remove the authentication tag (last 16 bytes)
    let tag_len = AES_256_GCM.tag_len();
    if plaintext.len() < tag_len {
        return Err(ChunkError::DecryptionFailed(
            "Ciphertext too short".to_string(),
        ));
    }
    plaintext.truncate(plaintext.len() - tag_len);

    Ok(plaintext)
}

/// Derive a key from ACI using HKDF-SHA256.
///
/// # Arguments
///
/// * `aci_key` - Signal ACI key (input keying material)
/// * `context` - Context string for key separation
///
/// # Returns
///
/// 32-byte derived key
fn derive_key(aci_key: &[u8], context: &[u8]) -> Result<Vec<u8>, ChunkError> {
    use hkdf::Hkdf;
    use sha2::Sha256;

    let hkdf = Hkdf::<Sha256>::new(None, aci_key);
    let mut okm = vec![0u8; 32]; // 32 bytes for AES-256
    hkdf.expand(context, &mut okm)
        .map_err(|e| ChunkError::KeyDerivationFailed(format!("HKDF expand failed: {}", e)))?;
    Ok(okm)
}

/// Generate a random 12-byte nonce for AES-GCM.
///
/// # Returns
///
/// 12-byte nonce
fn generate_nonce() -> [u8; 12] {
    use ring::rand::{SecureRandom, SystemRandom};

    let rng = SystemRandom::new();
    let mut nonce = [0u8; 12];
    rng.fill(&mut nonce).expect("RNG failure");
    nonce
}

/// Sign a chunk with HMAC-SHA256.
///
/// # Arguments
///
/// * `owner` - Owner's contract hash
/// * `index` - Chunk index
/// * `data` - Chunk data
/// * `signing_key` - Derived signing key (32 bytes)
///
/// # Returns
///
/// HMAC signature bytes
fn sign_chunk(owner: &str, index: u32, data: &[u8], signing_key: &[u8]) -> Vec<u8> {
    let key = hmac::Key::new(hmac::HMAC_SHA256, signing_key);

    // Sign: owner || index || data
    let mut message = Vec::new();
    message.extend_from_slice(owner.as_bytes());
    message.extend_from_slice(&index.to_le_bytes());
    message.extend_from_slice(data);

    let signature = hmac::sign(&key, &message);
    signature.as_ref().to_vec()
}

/// Verify a chunk's HMAC signature.
///
/// # Arguments
///
/// * `owner` - Owner's contract hash
/// * `index` - Chunk index
/// * `data` - Chunk data
/// * `signature` - Claimed signature
/// * `signing_key` - Derived signing key (32 bytes)
///
/// # Returns
///
/// `true` if signature is valid
fn verify_chunk_signature(
    owner: &str,
    index: u32,
    data: &[u8],
    signature: &[u8],
    signing_key: &[u8],
) -> bool {
    let key = hmac::Key::new(hmac::HMAC_SHA256, signing_key);

    // Reconstruct message: owner || index || data
    let mut message = Vec::new();
    message.extend_from_slice(owner.as_bytes());
    message.extend_from_slice(&index.to_le_bytes());
    message.extend_from_slice(data);

    hmac::verify(&key, &message, signature).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_aci_key() -> Vec<u8> {
        vec![42u8; 32] // Dummy 32-byte ACI key
    }

    #[test]
    fn test_encrypt_and_chunk_small_state() {
        let owner = "test-bot-123";
        let state = b"small test state";
        let aci_key = test_aci_key();

        let chunks = encrypt_and_chunk(owner, state, &aci_key).unwrap();

        assert_eq!(chunks.len(), 1, "Small state should produce 1 chunk");
        assert_eq!(chunks[0].owner, owner);
        assert_eq!(chunks[0].index, 0);
        assert!(!chunks[0].data.is_empty());
        assert!(!chunks[0].signature.is_empty());
    }

    #[test]
    fn test_encrypt_and_chunk_large_state() {
        let owner = "test-bot-456";
        let state = vec![7u8; CHUNK_SIZE * 3 + 1000]; // 3.1 chunks worth
        let aci_key = test_aci_key();

        let chunks = encrypt_and_chunk(owner, &state, &aci_key).unwrap();

        // Should produce 4 chunks (3 full + 1 partial)
        assert!(
            chunks.len() == 4 || chunks.len() == 5,
            "Large state should produce multiple chunks"
        );
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i as u32);
            assert_eq!(chunk.owner, owner);
        }
    }

    #[test]
    fn test_decrypt_and_reassemble() {
        let owner = "test-bot-789";
        let original_state = b"this is the original state data";
        let aci_key = test_aci_key();

        // Encrypt and chunk
        let chunks = encrypt_and_chunk(owner, original_state, &aci_key).unwrap();

        // Decrypt and reassemble
        let decrypted_state = decrypt_and_reassemble(&chunks, &aci_key).unwrap();

        assert_eq!(decrypted_state, original_state);
    }

    #[test]
    fn test_decrypt_with_wrong_key() {
        let owner = "test-bot-wrong-key";
        let state = b"secret state";
        let aci_key = test_aci_key();
        let wrong_key = vec![99u8; 32];

        let chunks = encrypt_and_chunk(owner, state, &aci_key).unwrap();

        // Should fail with wrong key (signature verification fails first)
        let result = decrypt_and_reassemble(&chunks, &wrong_key);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ChunkError::SignatureVerificationFailed) | Err(ChunkError::DecryptionFailed(_))
        ));
    }

    #[test]
    fn test_signature_verification_fails_on_tamper() {
        let owner = "test-bot-tamper";
        let state = b"state to tamper with";
        let aci_key = test_aci_key();

        let mut chunks = encrypt_and_chunk(owner, state, &aci_key).unwrap();

        // Tamper with chunk data
        chunks[0].data[0] ^= 0xFF;

        // Should fail signature verification
        let result = decrypt_and_reassemble(&chunks, &aci_key);
        assert!(matches!(
            result,
            Err(ChunkError::SignatureVerificationFailed)
        ));
    }

    #[test]
    fn test_missing_chunk() {
        let owner = "test-bot-missing";
        let state = vec![1u8; CHUNK_SIZE * 2]; // 2 chunks
        let aci_key = test_aci_key();

        let mut chunks = encrypt_and_chunk(owner, &state, &aci_key).unwrap();

        // Remove middle chunk
        chunks.remove(1);

        // Should fail with missing chunks error
        let result = decrypt_and_reassemble(&chunks, &aci_key);
        assert!(matches!(result, Err(ChunkError::MissingChunks { .. })));
    }

    #[test]
    fn test_chunks_order_independence() {
        let owner = "test-bot-order";
        let state = vec![42u8; CHUNK_SIZE * 3];
        let aci_key = test_aci_key();

        let chunks = encrypt_and_chunk(owner, &state, &aci_key).unwrap();

        // Reverse chunk order
        let mut reversed_chunks = chunks.clone();
        reversed_chunks.reverse();

        // Should still decrypt correctly
        let decrypted = decrypt_and_reassemble(&reversed_chunks, &aci_key).unwrap();
        assert_eq!(decrypted, state);
    }

    #[test]
    fn test_invalid_aci_key_length() {
        let owner = "test-bot";
        let state = b"test";
        let bad_key = vec![1u8; 16]; // Wrong size (not 32 bytes)

        let result = encrypt_and_chunk(owner, state, &bad_key);
        assert!(matches!(result, Err(ChunkError::KeyDerivationFailed(_))));
    }
}
