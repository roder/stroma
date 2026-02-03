//! Trait abstractions for Freenet operations.
//!
//! Following testing-standards.bead: "Trait abstraction for Freenet operations"
//! Enables mock implementations for unit testing.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Contract hash identifier (32 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContractHash([u8; 32]);

impl ContractHash {
    /// Create from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes[..32]);
        Self(hash)
    }

    /// Get bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for ContractHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

/// Contract state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractState {
    pub data: Vec<u8>,
}

/// Contract delta (state update).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractDelta {
    pub data: Vec<u8>,
}

/// State change event from subscription stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateChange {
    pub contract: ContractHash,
    pub new_state: ContractState,
}

/// Result type for Freenet operations.
pub type FreenetResult<T> = Result<T, FreenetError>;

/// Freenet operation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreenetError {
    /// Contract not found.
    ContractNotFound,
    /// Failed to apply delta.
    DeltaApplicationFailed,
    /// Subscription failed.
    SubscriptionFailed,
    /// Contract deployment failed.
    DeploymentFailed,
    /// Other error with message.
    Other(String),
}

impl fmt::Display for FreenetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContractNotFound => write!(f, "Contract not found"),
            Self::DeltaApplicationFailed => write!(f, "Failed to apply delta"),
            Self::SubscriptionFailed => write!(f, "Subscription failed"),
            Self::DeploymentFailed => write!(f, "Contract deployment failed"),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for FreenetError {}

/// Trait abstraction for Freenet operations.
///
/// Per testing-standards.bead: "Trait abstraction for Freenet operations"
/// Enables mock implementations for testing.
#[async_trait]
pub trait FreenetClient: Send + Sync {
    /// Get current state of a contract.
    async fn get_state(&self, contract: &ContractHash) -> FreenetResult<ContractState>;

    /// Apply a delta to a contract.
    async fn apply_delta(
        &self,
        contract: &ContractHash,
        delta: &ContractDelta,
    ) -> FreenetResult<()>;

    /// Subscribe to state changes for a contract.
    async fn subscribe(
        &self,
        contract: &ContractHash,
    ) -> FreenetResult<Box<dyn futures::Stream<Item = StateChange> + Send + Unpin>>;

    /// Deploy a new contract.
    async fn deploy_contract(
        &self,
        code: &[u8],
        initial_state: &[u8],
    ) -> FreenetResult<ContractHash>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_hash_from_bytes() {
        let bytes = [42u8; 32];
        let hash = ContractHash::from_bytes(&bytes);
        assert_eq!(hash.as_bytes(), &bytes);
    }

    #[test]
    fn test_contract_hash_equality() {
        let hash1 = ContractHash::from_bytes(&[1u8; 32]);
        let hash2 = ContractHash::from_bytes(&[1u8; 32]);
        let hash3 = ContractHash::from_bytes(&[2u8; 32]);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_contract_hash_display() {
        let bytes = [0x42u8; 32];
        let hash = ContractHash::from_bytes(&bytes);
        let display = format!("{}", hash);
        assert!(display.len() == 64); // 32 bytes * 2 hex chars
        assert!(display.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_freenet_error_display() {
        assert_eq!(
            format!("{}", FreenetError::ContractNotFound),
            "Contract not found"
        );
        assert_eq!(
            format!("{}", FreenetError::Other("test error".to_string())),
            "test error"
        );
    }

    #[test]
    fn test_contract_state_serialization() {
        let state = ContractState {
            data: vec![1, 2, 3, 4],
        };

        // Test round-trip serialization
        let serialized = serde_json::to_string(&state).unwrap();
        let deserialized: ContractState = serde_json::from_str(&serialized).unwrap();
        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_contract_delta_serialization() {
        let delta = ContractDelta {
            data: vec![5, 6, 7, 8],
        };

        // Test round-trip serialization
        let serialized = serde_json::to_string(&delta).unwrap();
        let deserialized: ContractDelta = serde_json::from_str(&serialized).unwrap();
        assert_eq!(delta, deserialized);
    }

    #[test]
    fn test_state_change_equality() {
        let hash = ContractHash::from_bytes(&[1u8; 32]);
        let state = ContractState {
            data: vec![1, 2, 3],
        };

        let change1 = StateChange {
            contract: hash,
            new_state: state.clone(),
        };
        let change2 = StateChange {
            contract: hash,
            new_state: state.clone(),
        };

        assert_eq!(change1, change2);
    }
}
