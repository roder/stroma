/// Cryptographic primitives for Stroma privacy-preserving protocols
///
/// This module implements:
/// - PSI-CA (Private Set Intersection - Cardinality Only) for federation discovery
/// - Commutative encryption for double-blinding
/// - Unified key derivation from BIP-39 mnemonic (keyring)
///
/// See: docs/ALGORITHMS.md § "External Federation: Private Set Intersection Algorithm"
pub mod keyring;
pub mod psi_ca;

pub use keyring::{KeyringError, StromaKeyring};
pub use psi_ca::{FederationThreshold, PsiError, PsiProtocol};
