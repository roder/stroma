/// Cryptographic primitives for Stroma privacy-preserving protocols
///
/// This module implements:
/// - PSI-CA (Private Set Intersection - Cardinality Only) for federation discovery
/// - Commutative encryption for double-blinding
///
/// See: docs/ALGORITHMS.md § "External Federation: Private Set Intersection Algorithm"

pub mod psi_ca;

pub use psi_ca::{PsiProtocol, PsiError, FederationThreshold};
