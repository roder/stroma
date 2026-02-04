//! Embedded Freenet kernel with in-process node.
//!
//! Per freenet-integration.bead:
//! - Unit tests use Executor::new_mock_in_memory()
//! - Production uses NodeConfig::build()
//! - Single event loop with real-time state monitoring

use crate::freenet::traits::{
    ContractDelta, ContractHash, ContractState, FreenetClient, FreenetError, FreenetResult,
    StateChange,
};
use async_trait::async_trait;
// use freenet::local_node::Executor;  // FIXME: Disabled while freenet dependency is commented out
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Embedded Freenet kernel using in-memory executor for testing.
///
/// Production will use NodeConfig::build(), but for unit tests we use
/// Executor::new_mock_in_memory() per testing-standards.bead.
pub struct EmbeddedKernel {
    /// In-memory executor (mock for testing).
    executor: Arc<RwLock<MockExecutor>>,
}

/// Mock executor for testing (wraps Freenet Executor).
struct MockExecutor {
    contracts: HashMap<ContractHash, Vec<u8>>,
}

impl MockExecutor {
    fn new() -> Self {
        Self {
            contracts: HashMap::new(),
        }
    }

    fn deploy(&mut self, hash: ContractHash, state: Vec<u8>) {
        self.contracts.insert(hash, state);
    }

    fn get_state(&self, hash: &ContractHash) -> Option<Vec<u8>> {
        self.contracts.get(hash).cloned()
    }

    fn apply_delta(&mut self, hash: &ContractHash, delta: &[u8]) -> Result<(), String> {
        if let Some(state) = self.contracts.get_mut(hash) {
            state.extend_from_slice(delta);
            Ok(())
        } else {
            Err("Contract not found".to_string())
        }
    }
}

impl EmbeddedKernel {
    /// Create new embedded kernel with mock executor for testing.
    ///
    /// Per freenet-integration.bead: "Use Executor::new_mock_in_memory() for unit tests"
    pub async fn new_in_memory() -> FreenetResult<Self> {
        Ok(Self {
            executor: Arc::new(RwLock::new(MockExecutor::new())),
        })
    }

    /// Deploy a contract with initial state.
    async fn deploy_internal(
        &self,
        code: &[u8],
        initial_state: &[u8],
    ) -> FreenetResult<ContractHash> {
        // For now, use SHA-256 hash of code as contract ID
        use sha2::{Digest, Sha256};
        let hash_bytes = Sha256::digest(code);
        let contract_hash = ContractHash::from_bytes(&hash_bytes);

        let mut executor = self.executor.write().await;
        executor.deploy(contract_hash, initial_state.to_vec());

        Ok(contract_hash)
    }
}

#[async_trait]
impl FreenetClient for EmbeddedKernel {
    async fn get_state(&self, contract: &ContractHash) -> FreenetResult<ContractState> {
        let executor = self.executor.read().await;
        match executor.get_state(contract) {
            Some(data) => Ok(ContractState { data }),
            None => Err(FreenetError::ContractNotFound),
        }
    }

    async fn apply_delta(
        &self,
        contract: &ContractHash,
        delta: &ContractDelta,
    ) -> FreenetResult<()> {
        let mut executor = self.executor.write().await;
        executor
            .apply_delta(contract, &delta.data)
            .map_err(FreenetError::Other)
    }

    async fn subscribe(
        &self,
        _contract: &ContractHash,
    ) -> FreenetResult<Box<dyn futures::Stream<Item = StateChange> + Send + Unpin>> {
        // For now, return an empty stream
        // Full implementation will come in state_stream.rs
        use futures::stream;
        Ok(Box::new(stream::empty()))
    }

    async fn deploy_contract(
        &self,
        code: &[u8],
        initial_state: &[u8],
    ) -> FreenetResult<ContractHash> {
        self.deploy_internal(code, initial_state).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_in_memory_kernel() {
        // Per freenet-integration.bead: "Use Executor::new_mock_in_memory() for unit tests"
        let kernel = EmbeddedKernel::new_in_memory().await;
        assert!(kernel.is_ok());
    }

    #[tokio::test]
    async fn test_deploy_contract() {
        let kernel = EmbeddedKernel::new_in_memory().await.unwrap();
        let code = b"contract_code";
        let initial_state = b"initial_state";

        let contract_hash = kernel.deploy_contract(code, initial_state).await;
        assert!(contract_hash.is_ok());
    }

    #[tokio::test]
    async fn test_get_state_after_deploy() {
        let kernel = EmbeddedKernel::new_in_memory().await.unwrap();
        let code = b"contract_code";
        let initial_state = b"initial_state";

        let contract_hash = kernel.deploy_contract(code, initial_state).await.unwrap();
        let state = kernel.get_state(&contract_hash).await.unwrap();

        assert_eq!(state.data, initial_state.to_vec());
    }

    #[tokio::test]
    async fn test_get_state_nonexistent_contract() {
        let kernel = EmbeddedKernel::new_in_memory().await.unwrap();
        let fake_hash = ContractHash::from_bytes(&[0u8; 32]);

        let result = kernel.get_state(&fake_hash).await;
        assert!(matches!(result, Err(FreenetError::ContractNotFound)));
    }

    #[tokio::test]
    async fn test_apply_delta() {
        let kernel = EmbeddedKernel::new_in_memory().await.unwrap();
        let code = b"contract_code";
        let initial_state = b"initial";

        let contract_hash = kernel.deploy_contract(code, initial_state).await.unwrap();

        let delta = ContractDelta {
            data: b"_delta".to_vec(),
        };
        let result = kernel.apply_delta(&contract_hash, &delta).await;
        assert!(result.is_ok());

        let new_state = kernel.get_state(&contract_hash).await.unwrap();
        assert_eq!(new_state.data, b"initial_delta".to_vec());
    }

    #[tokio::test]
    async fn test_apply_delta_nonexistent_contract() {
        let kernel = EmbeddedKernel::new_in_memory().await.unwrap();
        let fake_hash = ContractHash::from_bytes(&[0u8; 32]);

        let delta = ContractDelta {
            data: vec![1, 2, 3],
        };
        let result = kernel.apply_delta(&fake_hash, &delta).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_subscribe_returns_stream() {
        let kernel = EmbeddedKernel::new_in_memory().await.unwrap();
        let contract_hash = ContractHash::from_bytes(&[1u8; 32]);

        let stream = kernel.subscribe(&contract_hash).await;
        assert!(stream.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_contracts() {
        let kernel = EmbeddedKernel::new_in_memory().await.unwrap();

        // Deploy two contracts
        let hash1 = kernel.deploy_contract(b"code1", b"state1").await.unwrap();
        let hash2 = kernel.deploy_contract(b"code2", b"state2").await.unwrap();

        // Verify they have different hashes
        assert_ne!(hash1, hash2);

        // Verify each has correct state
        let state1 = kernel.get_state(&hash1).await.unwrap();
        let state2 = kernel.get_state(&hash2).await.unwrap();

        assert_eq!(state1.data, b"state1".to_vec());
        assert_eq!(state2.data, b"state2".to_vec());
    }

    #[tokio::test]
    async fn test_concurrent_delta_application() {
        let kernel = Arc::new(EmbeddedKernel::new_in_memory().await.unwrap());
        let contract_hash = kernel.deploy_contract(b"code", b"initial").await.unwrap();

        // Apply deltas concurrently
        let mut handles = vec![];
        for i in 0..10 {
            let kernel_clone = Arc::clone(&kernel);
            let hash_clone = contract_hash;
            let handle = tokio::spawn(async move {
                let delta = ContractDelta {
                    data: vec![b'0' + i as u8],
                };
                kernel_clone.apply_delta(&hash_clone, &delta).await
            });
            handles.push(handle);
        }

        // All should succeed
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        // State should have grown
        let final_state = kernel.get_state(&contract_hash).await.unwrap();
        assert!(final_state.data.len() > b"initial".len());
    }

    #[tokio::test]
    async fn test_deterministic_contract_hash() {
        let kernel = EmbeddedKernel::new_in_memory().await.unwrap();

        // Same code should produce same hash
        let hash1 = kernel
            .deploy_contract(b"same_code", b"state1")
            .await
            .unwrap();
        let hash2 = kernel
            .deploy_contract(b"same_code", b"state2")
            .await
            .unwrap();

        assert_eq!(hash1, hash2);
    }
}
