//! Mock Freenet client for testing.
//!
//! Per testing-standards.bead: "Mock-friendly trait abstractions for testing"

use super::traits::*;
use async_trait::async_trait;
use futures::stream;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock Freenet client for testing.
#[derive(Clone)]
pub struct MockFreenetClient {
    state: Arc<Mutex<MockState>>,
}

struct MockState {
    contracts: HashMap<ContractHash, ContractState>,
}

impl MockFreenetClient {
    /// Create new mock client.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState {
                contracts: HashMap::new(),
            })),
        }
    }

    /// Put a contract state (for test setup).
    pub fn put_state(&self, contract: ContractHash, state: ContractState) {
        let mut s = self.state.lock().unwrap();
        s.contracts.insert(contract, state);
    }
}

impl Default for MockFreenetClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FreenetClient for MockFreenetClient {
    async fn get_state(&self, contract: &ContractHash) -> FreenetResult<ContractState> {
        let state = self.state.lock().unwrap();
        state
            .contracts
            .get(contract)
            .cloned()
            .ok_or(FreenetError::ContractNotFound)
    }

    async fn apply_delta(
        &self,
        contract: &ContractHash,
        delta: &ContractDelta,
    ) -> FreenetResult<()> {
        let mut state = self.state.lock().unwrap();

        // Get current state
        let _current = state
            .contracts
            .get(contract)
            .cloned()
            .ok_or(FreenetError::ContractNotFound)?;

        // In a real implementation, we'd apply the delta to the state
        // For now, just store the delta data as new state
        let new_state = ContractState {
            data: delta.data.clone(),
        };

        state.contracts.insert(*contract, new_state);
        Ok(())
    }

    async fn subscribe(
        &self,
        _contract: &ContractHash,
    ) -> FreenetResult<Box<dyn futures::Stream<Item = StateChange> + Send + Unpin>> {
        // Return empty stream for mock
        Ok(Box::new(stream::empty()))
    }

    async fn deploy_contract(
        &self,
        _code: &[u8],
        initial_state: &[u8],
    ) -> FreenetResult<ContractHash> {
        let hash = ContractHash::from_bytes(&[0u8; 32]); // Mock hash
        let state = ContractState {
            data: initial_state.to_vec(),
        };

        let mut s = self.state.lock().unwrap();
        s.contracts.insert(hash, state);

        Ok(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_put_and_get() {
        let client = MockFreenetClient::new();
        let hash = ContractHash::from_bytes(&[1u8; 32]);
        let state = ContractState {
            data: vec![1, 2, 3],
        };

        client.put_state(hash, state.clone());

        let retrieved = client.get_state(&hash).await.unwrap();
        assert_eq!(retrieved, state);
    }

    #[tokio::test]
    async fn test_mock_get_not_found() {
        let client = MockFreenetClient::new();
        let hash = ContractHash::from_bytes(&[1u8; 32]);

        let result = client.get_state(&hash).await;
        assert!(matches!(result, Err(FreenetError::ContractNotFound)));
    }

    #[tokio::test]
    async fn test_mock_apply_delta() {
        let client = MockFreenetClient::new();
        let hash = ContractHash::from_bytes(&[1u8; 32]);

        // Put initial state
        let initial = ContractState {
            data: vec![1, 2, 3],
        };
        client.put_state(hash, initial);

        // Apply delta
        let delta = ContractDelta {
            data: vec![4, 5, 6],
        };
        client.apply_delta(&hash, &delta).await.unwrap();

        // Verify state changed
        let new_state = client.get_state(&hash).await.unwrap();
        assert_eq!(new_state.data, vec![4, 5, 6]);
    }

    #[tokio::test]
    async fn test_mock_deploy_contract() {
        let client = MockFreenetClient::new();

        let code = b"contract code";
        let initial_state = b"initial state";

        let hash = client.deploy_contract(code, initial_state).await.unwrap();

        // Verify contract was deployed with initial state
        let state = client.get_state(&hash).await.unwrap();
        assert_eq!(state.data, initial_state.to_vec());
    }
}
