//! STARK-based vouch verification for Stroma
//!
//! This module implements zero-knowledge proofs for vouch verification using
//! winterfell STARKs. Proofs are generated bot-side (not in Wasm) to verify:
//! - Effective vouches calculation: |Vouchers| - |Voucher_Flaggers|
//! - Regular flags calculation: |Flaggers| - |Voucher_Flaggers|
//! - Standing calculation: Effective_Vouches - Regular_Flags
//!
//! ## Performance Requirements
//! - Proof generation: < 10 seconds
//! - Proof size: < 100KB
//! - Verification: < 100ms (target)
//!
//! ## Security Properties
//! - Completeness: Valid claims produce verifying proofs
//! - Soundness: Invalid claims cannot produce verifying proofs
//! - Determinism: Same input produces same proof (given same randomness)

pub mod circuit;
pub mod prover;
pub mod types;
pub mod verifier;

#[cfg(test)]
mod proptests;

pub use circuit::VouchAir;
pub use prover::prove_vouch_claim;
pub use types::{VouchClaim, VouchProof};
pub use verifier::verify_vouch_proof;
