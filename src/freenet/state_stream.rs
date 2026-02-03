//! Real-time state stream monitoring for Freenet contracts.
//!
//! Per freenet-integration.bead:
//! - Real-time stream (NOT polling)
//! - Subscription-based via client proxy
//! - tokio::select! for event loop integration

use crate::freenet::traits::{ContractHash, ContractState, StateChange};
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

/// Real-time state stream for contract updates.
///
/// Per freenet-integration.bead: "Real-Time Stream (REQUIRED - never poll)"
pub struct StateStream {
    receiver: mpsc::UnboundedReceiver<StateChange>,
    contract: ContractHash,
}

impl StateStream {
    /// Create a new state stream for a contract.
    pub fn new(contract: ContractHash) -> (Self, StateStreamSender) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let stream = Self { receiver, contract };
        let sender = StateStreamSender { sender, contract };
        (stream, sender)
    }

    /// Get the contract this stream monitors.
    pub fn contract(&self) -> &ContractHash {
        &self.contract
    }
}

impl Stream for StateStream {
    type Item = StateChange;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

/// Sender side of state stream for testing.
pub struct StateStreamSender {
    sender: mpsc::UnboundedSender<StateChange>,
    contract: ContractHash,
}

impl StateStreamSender {
    /// Send a state change event (for testing).
    pub fn send(&self, new_state: ContractState) -> Result<(), String> {
        let change = StateChange {
            contract: self.contract,
            new_state,
        };
        self.sender
            .send(change)
            .map_err(|_| "Failed to send state change".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_state_stream_creation() {
        let contract = ContractHash::from_bytes(&[1u8; 32]);
        let (stream, _sender) = StateStream::new(contract);
        assert_eq!(stream.contract(), &contract);
    }

    #[tokio::test]
    async fn test_state_stream_receives_updates() {
        let contract = ContractHash::from_bytes(&[1u8; 32]);
        let (mut stream, sender) = StateStream::new(contract);

        // Send a state change
        let new_state = ContractState {
            data: vec![1, 2, 3],
        };
        sender.send(new_state.clone()).unwrap();

        // Receive it
        let received = stream.next().await;
        assert!(received.is_some());
        let change = received.unwrap();
        assert_eq!(change.contract, contract);
        assert_eq!(change.new_state, new_state);
    }

    #[tokio::test]
    async fn test_state_stream_multiple_updates() {
        let contract = ContractHash::from_bytes(&[1u8; 32]);
        let (mut stream, sender) = StateStream::new(contract);

        // Send multiple updates
        for i in 0..5 {
            let state = ContractState {
                data: vec![i as u8],
            };
            sender.send(state).unwrap();
        }

        // Receive all updates
        for i in 0..5 {
            let received = stream.next().await;
            assert!(received.is_some());
            let change = received.unwrap();
            assert_eq!(change.new_state.data, vec![i as u8]);
        }
    }

    #[tokio::test]
    async fn test_state_stream_closes_when_sender_dropped() {
        let contract = ContractHash::from_bytes(&[1u8; 32]);
        let (mut stream, sender) = StateStream::new(contract);

        // Drop sender
        drop(sender);

        // Stream should end
        let received = stream.next().await;
        assert!(received.is_none());
    }

    #[tokio::test]
    async fn test_state_stream_with_tokio_select() {
        let contract = ContractHash::from_bytes(&[1u8; 32]);
        let (mut stream, sender) = StateStream::new(contract);

        // Spawn task to send after delay
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            sender.send(ContractState { data: vec![42] }).unwrap();
        });

        // Use tokio::select! to wait for either stream or timeout
        let result = tokio::select! {
            change = stream.next() => {
                assert!(change.is_some());
                assert_eq!(change.unwrap().new_state.data, vec![42]);
                "received"
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                "timeout"
            }
        };

        assert_eq!(result, "received");
    }

    #[tokio::test]
    async fn test_state_stream_concurrent_senders() {
        let contract = ContractHash::from_bytes(&[1u8; 32]);
        let (mut stream, sender) = StateStream::new(contract);

        // Spawn multiple tasks sending concurrently
        let mut handles = vec![];
        for i in 0..10 {
            let sender_clone = sender.sender.clone();
            let handle = tokio::spawn(async move {
                let change = StateChange {
                    contract,
                    new_state: ContractState {
                        data: vec![i as u8],
                    },
                };
                sender_clone.send(change).unwrap();
            });
            handles.push(handle);
        }

        // Wait for all to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Receive all 10 updates
        let mut received_values = vec![];
        for _ in 0..10 {
            if let Some(change) = stream.next().await {
                received_values.push(change.new_state.data[0]);
            }
        }

        assert_eq!(received_values.len(), 10);
        // All values 0-9 should be present (order may vary)
        let mut sorted = received_values.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[tokio::test]
    async fn test_state_stream_backpressure() {
        let contract = ContractHash::from_bytes(&[1u8; 32]);
        let (mut stream, sender) = StateStream::new(contract);

        // Send many updates without consuming
        for i in 0..100 {
            let state = ContractState {
                data: vec![i as u8],
            };
            // Should not block or fail
            sender.send(state).unwrap();
        }

        // Now consume all
        let mut count = 0;
        while let Some(_) = stream.next().await {
            count += 1;
            if count >= 100 {
                break;
            }
        }

        assert_eq!(count, 100);
    }
}
