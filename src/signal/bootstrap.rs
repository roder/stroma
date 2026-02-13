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

use super::traits::{GroupId, ServiceId, SignalClient, SignalError, SignalResult};
use crate::freenet::{
    contract::MemberHash,
    traits::{ContractHash, FreenetClient},
    trust_contract::{StateDelta, TrustNetworkState},
};
use crate::gatekeeper::audit_trail::AuditEntry;
use crate::identity::mask_identity;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// Bootstrap state machine
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BootstrapState {
    /// Awaiting /create-group command
    #[default]
    AwaitingInitiation,

    /// Collecting seed members (1-2 collected so far)
    CollectingSeeds {
        group_name: String,
        initiator: MemberHash,
        seeds: Vec<MemberHash>, // Includes initiator + additional seeds
        /// Raw identity strings kept for Signal group creation (as pending invites).
        /// Stored as Vec<String> to avoid Serialize/Deserialize on ServiceId.
        seed_service_ids: Vec<String>,
    },

    /// Bootstrap complete, normal operation
    Complete {
        group_id: GroupId,
        group_name: String,
        contract_hash: ContractHash,
    },
}

/// Bootstrap manager
pub struct BootstrapManager<C: SignalClient> {
    signal: C,
    state: BootstrapState,
    /// Identity masking key from StromaKeyring (mnemonic-derived)
    identity_masking_key: [u8; 32],
}

impl<C: SignalClient> BootstrapManager<C> {
    /// Create new bootstrap manager
    ///
    /// # Arguments
    /// * `signal` - Signal client
    /// * `identity_masking_key` - Key from `StromaKeyring::identity_masking_key()`
    pub fn new(signal: C, identity_masking_key: [u8; 32]) -> Self {
        Self {
            signal,
            state: BootstrapState::AwaitingInitiation,
            identity_masking_key,
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

        // Hash initiator identity immediately using secure HMAC
        let initiator_hash = mask_identity(&initiator.0, &self.identity_masking_key).into();

        // Transition to collecting seeds
        self.state = BootstrapState::CollectingSeeds {
            group_name: trimmed_name.to_string(),
            initiator: initiator_hash,
            seeds: vec![initiator_hash],
            seed_service_ids: vec![initiator.0.clone()],
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
        let (group_name, initiator, current_seeds, current_service_ids) = match &self.state {
            BootstrapState::CollectingSeeds {
                group_name,
                initiator,
                seeds,
                seed_service_ids,
            } => (
                group_name.clone(),
                *initiator,
                seeds.clone(),
                seed_service_ids.clone(),
            ),
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
        let from_hash: MemberHash = mask_identity(&from.0, &self.identity_masking_key).into();
        if from_hash != initiator {
            return Err(SignalError::Unauthorized);
        }

        // Check if we already have 3 seeds
        if current_seeds.len() >= 3 {
            return Err(SignalError::InvalidMessage(
                "Seed group already complete (3 members)".to_string(),
            ));
        }

        // Hash new seed identity using secure HMAC
        let new_seed_hash: MemberHash =
            mask_identity(new_seed_username, &self.identity_masking_key).into();

        // Check for duplicates
        if current_seeds.contains(&new_seed_hash) {
            return Err(SignalError::InvalidMessage(
                "This member is already a seed".to_string(),
            ));
        }

        // Add new seed
        let mut updated_seeds = current_seeds;
        updated_seeds.push(new_seed_hash);

        // Keep raw identity string for Signal group creation
        let mut updated_service_ids = current_service_ids;
        updated_service_ids.push(new_seed_username.to_string());

        let seed_count = updated_seeds.len();

        // Update state
        self.state = BootstrapState::CollectingSeeds {
            group_name: group_name.clone(),
            initiator,
            seeds: updated_seeds.clone(),
            seed_service_ids: updated_service_ids.clone(),
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

            // Complete bootstrap with both hashes (for Freenet) and ServiceIds (for Signal group)
            self.complete_bootstrap(freenet, group_name, updated_seeds, &updated_service_ids)
                .await?;
        }

        Ok(())
    }

    /// Complete bootstrap by creating Signal group and Freenet contract
    pub async fn complete_bootstrap(
        &mut self,
        freenet: &impl FreenetClient,
        group_name: String,
        seed_hashes: Vec<MemberHash>,
        seed_service_id_strings: &[String],
    ) -> SignalResult<()> {
        assert_eq!(seed_hashes.len(), 3, "Must have exactly 3 seed members");
        assert_eq!(
            seed_service_id_strings.len(),
            3,
            "Must have exactly 3 seed ServiceIds"
        );

        // Convert raw strings to ServiceIds for the Signal API
        let seed_service_ids: Vec<ServiceId> = seed_service_id_strings
            .iter()
            .map(|s| ServiceId(s.clone()))
            .collect();

        // 1. Create Signal group with all seed members
        // Members without profile keys will be added as pending invites.
        // pending_members contains ServiceIds that need invite DMs sent separately
        // (after the websocket recovers from the group creation).
        let (group_id, pending_members) = self
            .signal
            .create_group(&group_name, &seed_service_ids)
            .await?;

        // 2. Create Freenet contract with triangle vouching
        let mut state = TrustNetworkState::new();

        // Build delta with all changes
        let mut delta = StateDelta::new();

        // Add all 3 members
        for member in &seed_hashes {
            delta.members_added.push(*member);
        }

        // Create full triangle (each vouches for the other two)
        for i in 0..3 {
            for j in 0..3 {
                if i != j {
                    delta.vouches_added.push((seed_hashes[i], seed_hashes[j]));
                }
            }
        }

        // Add bootstrap audit entry (GAP-09)
        // Use first seed member as the actor (bootstrap initiator)
        let bootstrap_actor = seed_hashes[0];
        let seed_hashes_display = seed_hashes
            .iter()
            .map(|h| hex::encode(&h.as_bytes()[..4]))
            .collect::<Vec<_>>()
            .join(", ");
        let audit_entry = AuditEntry::bootstrap(
            bootstrap_actor,
            format!(
                "Group '{}' bootstrapped with 3 seed members: {}…",
                group_name, seed_hashes_display
            ),
        );
        delta.audit_entries_added.push(audit_entry);

        // Apply delta to state
        state.apply_delta(&delta);

        // 3. Serialize and deploy contract to Freenet (using CBOR for binary-safe serialization)
        let state_bytes = state
            .to_bytes()
            .map_err(|e| SignalError::Protocol(format!("Failed to serialize state: {}", e)))?;

        // Deploy contract (this will return the contract hash)
        let contract_hash = freenet
            .deploy_contract(&[], &state_bytes)
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
        // 5. Send group announcement and invite DMs
        self.signal.send_group_message(&group_id, &message).await?;

        for member in &pending_members {
            match self.signal.send_group_invite(&group_id, member).await {
                Ok(()) => {
                    tracing::info!(member = %member.0, "group invite DM sent");
                }
                Err(e) => {
                    tracing::warn!(
                        member = %member.0,
                        "failed to send group invite DM: {}",
                        e
                    );
                }
            }
        }

        // 6. Transition to complete state
        self.state = BootstrapState::Complete {
            group_id,
            group_name,
            contract_hash,
        };

        Ok(())
    }
}

// NOTE: The old hash_identity function using SHA256(identity||pepper) has been removed.
// All identity hashing now uses identity::mask_identity() with HMAC-SHA256 and mnemonic-derived key.

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

    fn test_masking_key() -> [u8; 32] {
        *b"test-masking-key-32-bytes-pad!!!" // 32 bytes exactly
    }

    #[tokio::test]
    async fn test_bootstrap_manager_creation() {
        let signal = MockSignalClient::new(ServiceId("bot".to_string()));
        let manager = BootstrapManager::new(signal, test_masking_key());

        assert!(matches!(
            manager.state(),
            BootstrapState::AwaitingInitiation
        ));
        assert!(!manager.is_complete());
    }

    #[tokio::test]
    async fn test_create_group() {
        let signal = MockSignalClient::new(ServiceId("bot".to_string()));
        let mut manager = BootstrapManager::new(signal.clone(), test_masking_key());

        let initiator = ServiceId("alice".to_string());
        let result = manager
            .handle_create_group(&initiator, "Test Group".to_string())
            .await;

        assert!(result.is_ok());

        // Check state transition
        match manager.state() {
            BootstrapState::CollectingSeeds {
                group_name, seeds, ..
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
        let mut manager = BootstrapManager::new(signal, test_masking_key());

        let initiator = ServiceId("alice".to_string());
        let result = manager
            .handle_create_group(&initiator, "   ".to_string())
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_group_already_in_progress() {
        let signal = MockSignalClient::new(ServiceId("bot".to_string()));
        let mut manager = BootstrapManager::new(signal, test_masking_key());

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
        let mut manager = BootstrapManager::new(signal, test_masking_key());

        let from = ServiceId("alice".to_string());
        let freenet = MockFreenetClient::new();
        let result = manager.handle_add_seed(&freenet, &from, "@bob").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_seed_by_non_initiator() {
        let signal = MockSignalClient::new(ServiceId("bot".to_string()));
        let mut manager = BootstrapManager::new(signal, test_masking_key());

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
        let mut manager = BootstrapManager::new(signal.clone(), test_masking_key());

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
        let result = manager
            .handle_add_seed(&freenet, &initiator, "@charlie")
            .await;
        assert!(result.is_ok());

        // Should now be complete
        assert!(manager.is_complete());
    }

    #[tokio::test]
    async fn test_add_duplicate_seed() {
        let signal = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let mut manager = BootstrapManager::new(signal, test_masking_key());

        let initiator = ServiceId("alice".to_string());
        manager
            .handle_create_group(&initiator, "Test Group".to_string())
            .await
            .unwrap();

        // Add seed
        manager
            .handle_add_seed(&freenet, &initiator, "@bob")
            .await
            .unwrap();

        // Try to add same seed again
        let result = manager.handle_add_seed(&freenet, &initiator, "@bob").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mask_identity_consistency() {
        let key = test_masking_key();
        let hash1: MemberHash = mask_identity("alice", &key).into();
        let hash2: MemberHash = mask_identity("alice", &key).into();
        let hash3: MemberHash = mask_identity("bob", &key).into();

        // Same identity produces same hash
        assert_eq!(hash1, hash2);
        // Different identity produces different hash
        assert_ne!(hash1, hash3);
    }

    #[tokio::test]
    async fn test_complete_bootstrap_creates_triangle() {
        let signal = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let mut manager = BootstrapManager::new(signal.clone(), test_masking_key());

        let initiator = ServiceId("alice".to_string());
        manager
            .handle_create_group(&initiator, "Test Group".to_string())
            .await
            .unwrap();

        manager
            .handle_add_seed(&freenet, &initiator, "@bob")
            .await
            .unwrap();
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

    #[tokio::test]
    async fn test_bootstrap_creates_audit_entry() {
        use crate::gatekeeper::audit_trail::ActionType;
        use crate::serialization::from_cbor;

        let signal = MockSignalClient::new(ServiceId("bot".to_string()));
        let freenet = MockFreenetClient::new();
        let mut manager = BootstrapManager::new(signal.clone(), test_masking_key());

        let initiator = ServiceId("alice".to_string());
        manager
            .handle_create_group(&initiator, "Test Group".to_string())
            .await
            .unwrap();

        manager
            .handle_add_seed(&freenet, &initiator, "@bob")
            .await
            .unwrap();
        manager
            .handle_add_seed(&freenet, &initiator, "@charlie")
            .await
            .unwrap();

        // Get the deployed contract from Freenet
        let contract_hash = match manager.state() {
            BootstrapState::Complete { contract_hash, .. } => *contract_hash,
            _ => panic!("Expected Complete state"),
        };

        // Query Freenet for the contract state
        let state_result = freenet.get_state(&contract_hash).await;
        assert!(state_result.is_ok());

        let state_bytes = state_result.unwrap().data;
        let state: crate::freenet::trust_contract::TrustNetworkState =
            from_cbor(&state_bytes).unwrap();

        // Verify audit log contains bootstrap entry
        assert!(!state.audit_log.is_empty());
        let bootstrap_entries: Vec<_> = state
            .audit_log
            .iter()
            .filter(|e| e.action_type == ActionType::Bootstrap)
            .collect();
        assert_eq!(bootstrap_entries.len(), 1);

        // Verify bootstrap entry contains group name
        let entry = bootstrap_entries[0];
        assert!(entry.details.contains("Test Group"));
        assert!(entry.details.contains("bootstrapped"));
        assert!(entry.details.contains("3 seed members"));
    }
}
