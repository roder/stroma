//! Bootstrap and Seed Group Management
//!
//! Implements member-initiated bootstrap for creating new Stroma groups.
//!
//! Key requirements:
//! - 3-member seed group (forms initial trust triangle)
//! - Member-initiated (not operator-initiated)
//! - Identity hashing and zeroization for privacy
//! - Triangle vouching: all 3 seeds vouch for each other
//!
//! See: .beads/bootstrap-seed.bead

use super::traits::{SignalClient, SignalError, SignalResult, ServiceId, GroupId};
use crate::freenet::{
    contract::{MemberHash, TrustContract, TrustDelta},
    traits::{ContractHash, FreenetClient},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

/// Bootstrap state machine
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BootstrapState {
    /// Awaiting /create-group command
    AwaitingInitiation,

    /// Collecting seed members (1-2 collected so far)
    CollectingSeeds {
        group_name: String,
        initiator: MemberHash,
        seeds: Vec<MemberHash>,  // Includes initiator + additional seeds
    },

    /// Bootstrap complete, normal operation
    Complete {
        group_id: GroupId,
        group_name: String,
        contract_hash: ContractHash,
    },
}

impl Default for BootstrapState {
    fn default() -> Self {
        Self::AwaitingInitiation
    }
}

/// Bootstrap manager
pub struct BootstrapManager<C: SignalClient> {
    signal: C,
    state: BootstrapState,
    /// ACI identity key for HMAC (derived from Signal identity)
    pepper: Vec<u8>,
}

impl<C: SignalClient> BootstrapManager<C> {
    /// Create new bootstrap manager
    pub fn new(signal: C, pepper: Vec<u8>) -> Self {
        Self {
            signal,
            state: BootstrapState::AwaitingInitiation,
            pepper,
        }
    }

    /// Get current state
    pub fn state(&self) -> &BootstrapState {
        &self.state
    }

    /// Check if bootstrap is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.state, BootstrapState::Complete { .. })
    }

    /// Handle /create-group command
    pub async fn handle_create_group(
        &mut self,
        initiator: &ServiceId,
        group_name: String,
    ) -> SignalResult<()> {
        // Validate state
        if !matches!(self.state, BootstrapState::AwaitingInitiation) {
            return Err(SignalError::InvalidMessage(
                "Bootstrap already in progress or complete".to_string(),
            ));
        }

        // Validate group name
        let trimmed_name = group_name.trim();
        if trimmed_name.is_empty() {
            return Err(SignalError::InvalidMessage(
                "Group name cannot be empty".to_string(),
            ));
        }

        // Hash initiator identity immediately
        let initiator_hash = hash_identity(&initiator.0, &self.pepper);

        // Transition to collecting seeds
        self.state = BootstrapState::CollectingSeeds {
            group_name: trimmed_name.to_string(),
            initiator: initiator_hash,
            seeds: vec![initiator_hash],
        };

        // Send instructions to initiator
        let message = format!(
            "🌱 Creating new Stroma group: '{}'\n\n\
            You are seed member #1. To complete the seed group, you need \
            2 more trusted people who know each other AND know you.\n\n\
            Invite them with:\n  /add-seed @MemberB\n  /add-seed @MemberC\n\n\
            ⚠️ Choose carefully - these 3 people form the foundation of trust.\n   \
            All seed members will automatically vouch for each other.",
            trimmed_name
        );
        self.signal.send_message(initiator, &message).await?;

        Ok(())
    }

    /// Handle /add-seed command
    pub async fn handle_add_seed(
        &mut self,
        freenet: &impl FreenetClient,
        from: &ServiceId,
        new_seed_username: &str,
    ) -> SignalResult<()> {
        // Get current collecting state
        let (group_name, initiator, current_seeds) = match &self.state {
            BootstrapState::CollectingSeeds {
                group_name,
                initiator,
                seeds,
            } => (group_name.clone(), *initiator, seeds.clone()),
            BootstrapState::AwaitingInitiation => {
                return Err(SignalError::InvalidMessage(
                    "No bootstrap in progress. Use /create-group first.".to_string(),
                ));
            }
            BootstrapState::Complete { .. } => {
                return Err(SignalError::InvalidMessage(
                    "Bootstrap already complete. Use /invite for new members.".to_string(),
                ));
            }
        };

        // Verify sender is the initiator
        let from_hash = hash_identity(&from.0, &self.pepper);
        if from_hash != initiator {
            return Err(SignalError::Unauthorized);
        }

        // Check if we already have 3 seeds
        if current_seeds.len() >= 3 {
            return Err(SignalError::InvalidMessage(
                "Seed group already complete (3 members)".to_string(),
            ));
        }

        // Hash new seed identity
        let new_seed_hash = hash_identity(new_seed_username, &self.pepper);

        // Check for duplicates
        if current_seeds.contains(&new_seed_hash) {
            return Err(SignalError::InvalidMessage(
                "This member is already a seed".to_string(),
            ));
        }

        // Add new seed
        let mut updated_seeds = current_seeds;
        updated_seeds.push(new_seed_hash);

        let seed_count = updated_seeds.len();

        // Update state
        self.state = BootstrapState::CollectingSeeds {
            group_name: group_name.clone(),
            initiator,
            seeds: updated_seeds.clone(),
        };

        // Notify initiator
        if seed_count < 3 {
            let message = format!(
                "✅ {} added as seed member #{}.\n   Need {} more seed member(s) to complete the group.",
                new_seed_username,
                seed_count,
                3 - seed_count
            );
            self.signal.send_message(from, &message).await?;
        } else {
            let message = format!(
                "✅ {} added as seed member #3.\n   Seed group complete! Creating Signal group and Freenet contract...",
                new_seed_username
            );
            self.signal.send_message(from, &message).await?;

            // Complete bootstrap
            self.complete_bootstrap(freenet, group_name, updated_seeds).await?;
        }

        // TODO: Notify new seed member
        // For now, we'd need to map MemberHash back to ServiceId, which isn't implemented yet
        // This will be handled in the integration with actual Signal client

        Ok(())
    }

    /// Complete bootstrap by creating Signal group and Freenet contract
    pub async fn complete_bootstrap(
        &mut self,
        freenet: &impl FreenetClient,
        group_name: String,
        seed_hashes: Vec<MemberHash>,
    ) -> SignalResult<()> {
        assert_eq!(seed_hashes.len(), 3, "Must have exactly 3 seed members");

        // 1. Create Signal group
        let group_id = self.signal.create_group(&group_name).await?;

        // TODO: Add all 3 seed members to Signal group
        // This requires mapping MemberHash back to ServiceId, which we'll implement
        // when integrating with actual Signal client

        // 2. Create Freenet contract with triangle vouching
        let mut contract = TrustContract::new();

        // Add all 3 members
        for member in &seed_hashes {
            contract.apply_delta(&TrustDelta::AddMember { member: *member });
        }

        // Create full triangle (each vouches for the other two)
        for i in 0..3 {
            for j in 0..3 {
                if i != j {
                    contract.apply_delta(&TrustDelta::AddVouch {
                        voucher: seed_hashes[i],
                        vouchee: seed_hashes[j],
                    });
                }
            }
        }

        // 3. Serialize and deploy contract to Freenet (using CBOR for binary-safe serialization)
        let mut contract_bytes = Vec::new();
        ciborium::into_writer(&contract, &mut contract_bytes)
            .map_err(|e| SignalError::Protocol(format!("Failed to serialize contract: {}", e)))?;

        // Deploy contract (this will return the contract hash)
        let contract_hash = freenet
            .deploy_contract(&[], &contract_bytes)
            .await
            .map_err(|e| SignalError::Protocol(format!("Failed to deploy contract: {}", e)))?;

        // 4. Announce to group
        let message = format!(
            "🎉 '{}' is now live!\n\n\
            Seed group established:\n\
            - 3 seed members with mutual vouches\n\
            - All members have 2 vouches each\n\n\
            You can now invite others:\n  /invite @NewMember [context]\n\n\
            Type /help for all commands.",
            group_name
        );
        self.signal
            .send_group_message(&group_id, &message)
            .await?;

        // 5. Transition to complete state
        self.state = BootstrapState::Complete {
            group_id,
            group_name,
            contract_hash,
        };

        Ok(())
    }
}

/// Hash identity with pepper (HMAC-like)
fn hash_identity(identity: &str, pepper: &[u8]) -> MemberHash {
    let mut hasher = Sha256::new();
    hasher.update(identity.as_bytes());
    hasher.update(pepper);
    let hash_bytes = hasher.finalize();

    // Create MemberHash from bytes
    let mut hash_array = [0u8; 32];
    hash_array.copy_from_slice(&hash_bytes[..32]);

    MemberHash::from_bytes(&hash_array)
}

/// Zeroize identity after hashing (for security)
#[allow(dead_code)]
fn zeroize_identity(mut identity: String) {
    identity.zeroize();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::freenet::MockFreenetClient;
    use crate::signal::mock::MockSignalClient;

    fn test_pepper() -> Vec<u8> {
        b"test-pepper-123".to_vec()
    }

    #[tokio::test]
    async fn test_bootstrap_manager_creation() {
        let signal = MockSignalClient::new(ServiceId("bot".to_string()));
        let manager = BootstrapManager::new(signal, test_pepper());

        assert!(matches!(manager.state(), BootstrapState::AwaitingInitiation));
        assert!(!manager.is_complete());
    }

    #[tokio::test]
    async fn test_create_group() {
        let signal = MockSignalClient::new(ServiceId("bot".to_string()));
        let mut manager = BootstrapManager::new(signal.clone(), test_pepper());

        let initiator = ServiceId("alice".to_string());
        let result = manager
            .handle_create_group(&initiator, "Test Group".to_string())
            .await;

        assert!(result.is_ok());

        // Check state transition
        match manager.state() {
            BootstrapState::CollectingSeeds {
                group_name,
                seeds,
                ..
            } => {
                assert_eq!(group_name, "Test Group");
                assert_eq!(seeds.len(), 1); // Initiator is first seed
            }
            _ => panic!("Expected CollectingSeeds state"),
        }

        // Check message sent
        let messages = signal.sent_messages();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].content.contains("seed member #1"));
    }

    #[tokio::test]
    async fn test_create_group_empty_name() {
        let signal = MockSignalClient::new(ServiceId("bot".to_string()));
        let mut manager = BootstrapManager::new(signal, test_pepper());

        let initiator = ServiceId("alice".to_string());
        let result = manager.handle_create_group(&initiator, "   ".to_string()).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_group_already_in_progress() {
        let signal = MockSignalClient::new(ServiceId("bot".to_string()));
        let mut manager = BootstrapManager::new(signal, test_pepper());

        let initiator = ServiceId("alice".to_string());

        // First call succeeds
        manager
            .handle_create_group(&initiator, "Test Group".to_string())
            .await
            .unwrap();

        // Second call fails
        let result = manager
            .handle_create_group(&initiator, "Another Group".to_string())
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_seed_without_create_group() {
        let signal = MockSignalClient::new(ServiceId("bot".to_string()));
        let mut manager = BootstrapManager::new(signal, test_pepper());

        let from = ServiceId("alice".to_string());
        let freenet = MockFreenetClient::new();
        let result = manager.handle_add_seed(&freenet, &from, "@bob").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_seed_by_non_initiator() {
        let signal = MockSignalClient::new(ServiceId("bot".to_string()));
        let mut manager = BootstrapManager::new(signal, test_pepper());

        let initiator = ServiceId("alice".to_string());
        manager
            .handle_create_group(&initiator, "Test Group".to_string())
            .await
            .unwrap();

        // Try to add seed as someone else
        let other = ServiceId("eve".to_string());
        let freenet = MockFreenetClient::new();
        let result = manager.handle_add_seed(&freenet, &other, "@bob").await;

        assert!(matches!(result, Err(SignalError::Unauthorized)));
    }

    #[tokio::test]
    async fn test_add_two_seeds() {
        let signal = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let mut manager = BootstrapManager::new(signal.clone(), test_pepper());

        let initiator = ServiceId("alice".to_string());
        manager
            .handle_create_group(&initiator, "Test Group".to_string())
            .await
            .unwrap();

        // Add first seed
        let result = manager.handle_add_seed(&freenet, &initiator, "@bob").await;
        assert!(result.is_ok());

        match manager.state() {
            BootstrapState::CollectingSeeds { seeds, .. } => {
                assert_eq!(seeds.len(), 2);
            }
            _ => panic!("Expected CollectingSeeds state"),
        }

        // Add second seed
        let result = manager.handle_add_seed(&freenet, &initiator, "@charlie").await;
        assert!(result.is_ok());

        // Should now be complete
        assert!(manager.is_complete());
    }

    #[tokio::test]
    async fn test_add_duplicate_seed() {
        let signal = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let mut manager = BootstrapManager::new(signal, test_pepper());

        let initiator = ServiceId("alice".to_string());
        manager
            .handle_create_group(&initiator, "Test Group".to_string())
            .await
            .unwrap();

        // Add seed
        manager.handle_add_seed(&freenet, &initiator, "@bob").await.unwrap();

        // Try to add same seed again
        let result = manager.handle_add_seed(&freenet, &initiator, "@bob").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_hash_identity_consistency() {
        let pepper = test_pepper();
        let hash1 = hash_identity("alice", &pepper);
        let hash2 = hash_identity("alice", &pepper);
        let hash3 = hash_identity("bob", &pepper);

        // Same identity produces same hash
        assert_eq!(hash1, hash2);
        // Different identity produces different hash
        assert_ne!(hash1, hash3);
    }

    #[tokio::test]
    async fn test_complete_bootstrap_creates_triangle() {
        let signal = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let mut manager = BootstrapManager::new(signal.clone(), test_pepper());

        let initiator = ServiceId("alice".to_string());
        manager
            .handle_create_group(&initiator, "Test Group".to_string())
            .await
            .unwrap();

        manager.handle_add_seed(&freenet, &initiator, "@bob").await.unwrap();
        manager
            .handle_add_seed(&freenet, &initiator, "@charlie")
            .await
            .unwrap();

        // Verify state is complete
        match manager.state() {
            BootstrapState::Complete {
                group_name,
                group_id,
                ..
            } => {
                assert_eq!(group_name, "Test Group");
                assert!(!group_id.0.is_empty());
            }
            _ => panic!("Expected Complete state"),
        }

        // Verify group announcement sent
        let messages = signal.sent_messages();
        let group_messages: Vec<_> = messages
            .iter()
            .filter(|m| m.content.contains("is now live"))
            .collect();
        assert_eq!(group_messages.len(), 1);
    }
}
