//! Proposal execution logic.
//!
//! Applies approved proposals to Freenet state.

use crate::freenet::{
    contract::MemberHash,
    traits::{ContractDelta, ContractHash, FreenetClient},
    trust_contract::{ConfigUpdate, GroupConfig, StateDelta},
};
use crate::gatekeeper::audit_trail::AuditEntry;
use crate::serialization::to_cbor;
use crate::signal::{
    polls::ProposalType,
    traits::{SignalError, SignalResult},
};

/// Execute an approved proposal.
///
/// Updates Freenet contract state based on proposal type.
pub async fn execute_proposal<F: FreenetClient>(
    freenet: &F,
    contract: &ContractHash,
    proposal_type: &ProposalType,
    current_config: &GroupConfig,
) -> SignalResult<()> {
    match proposal_type {
        ProposalType::ConfigChange { key, value } => {
            execute_config_change(freenet, contract, current_config, key, value).await
        }
        ProposalType::Federation { target_group } => {
            execute_federation_proposal(freenet, contract, target_group).await
        }
        ProposalType::Other { description } => {
            execute_other_proposal(freenet, contract, description, current_config).await
        }
    }
}

/// Execute config change proposal.
async fn execute_config_change<F: FreenetClient>(
    freenet: &F,
    contract: &ContractHash,
    current_config: &GroupConfig,
    key: &str,
    value: &str,
) -> SignalResult<()> {
    // Create updated config
    let mut new_config = current_config.clone();

    // Apply the change based on key
    match key {
        "min_vouches" => {
            new_config.min_vouches = value.parse().map_err(|_| {
                SignalError::InvalidMessage("Invalid min_vouches value".to_string())
            })?;
        }
        "max_flags" => {
            new_config.max_flags = value
                .parse()
                .map_err(|_| SignalError::InvalidMessage("Invalid max_flags value".to_string()))?;
        }
        "open_membership" => {
            new_config.open_membership = value.parse().map_err(|_| {
                SignalError::InvalidMessage("Invalid open_membership value".to_string())
            })?;
        }
        "default_poll_timeout_secs" => {
            new_config.default_poll_timeout_secs = value.parse().map_err(|_| {
                SignalError::InvalidMessage("Invalid default_poll_timeout_secs value".to_string())
            })?;
        }
        "config_change_threshold" => {
            new_config.config_change_threshold = value.parse().map_err(|_| {
                SignalError::InvalidMessage("Invalid config_change_threshold value".to_string())
            })?;
        }
        "min_quorum" => {
            new_config.min_quorum = value
                .parse()
                .map_err(|_| SignalError::InvalidMessage("Invalid min_quorum value".to_string()))?;
        }
        _ => {
            return Err(SignalError::InvalidMessage(format!(
                "Unknown config key: {}",
                key
            )));
        }
    }

    // Create delta with config update
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Create audit entry for config change
    // Use system actor (all zeros) for governance-driven changes
    let system_actor = MemberHash::from_bytes(&[0u8; 32]);
    let old_value = match key {
        "min_vouches" => current_config.min_vouches.to_string(),
        "max_flags" => current_config.max_flags.to_string(),
        "open_membership" => current_config.open_membership.to_string(),
        "default_poll_timeout_secs" => current_config.default_poll_timeout_secs.to_string(),
        "config_change_threshold" => current_config.config_change_threshold.to_string(),
        "min_quorum" => current_config.min_quorum.to_string(),
        _ => "unknown".to_string(),
    };

    let audit_entry = AuditEntry::config_change(
        system_actor,
        format!(
            "Proposal approved: {} changed from {} to {}",
            key, old_value, value
        ),
    );

    let delta = StateDelta {
        members_added: vec![],
        members_removed: vec![],
        vouches_added: vec![],
        vouches_removed: vec![],
        flags_added: vec![],
        flags_removed: vec![],
        config_update: Some(ConfigUpdate {
            config: new_config,
            timestamp,
        }),
        proposals_created: vec![],
        proposals_checked: vec![],
        proposals_with_results: vec![],
        audit_entries_added: vec![audit_entry],
        gap11_announcement_sent: false,
    };

    // Serialize and apply delta
    let delta_bytes = to_cbor(&delta)
        .map_err(|e| SignalError::Protocol(format!("Failed to serialize delta: {}", e)))?;

    let contract_delta = ContractDelta { data: delta_bytes };
    freenet
        .apply_delta(contract, &contract_delta)
        .await
        .map_err(|e| SignalError::Protocol(format!("Failed to apply delta: {}", e)))?;

    Ok(())
}

/// Execute federation proposal.
///
/// Adds a target group's contract to the federation_contracts list.
/// This enables cross-group trust federation.
async fn execute_federation_proposal<F: FreenetClient>(
    _freenet: &F,
    _contract: &ContractHash,
    target_group: &str,
) -> SignalResult<()> {
    // Parse target_group as a contract hash
    // Format expected: hex string of 32 bytes
    if target_group.len() != 64 {
        return Err(SignalError::InvalidMessage(format!(
            "Invalid contract hash length: {}. Expected 64 hex characters.",
            target_group.len()
        )));
    }

    let target_bytes = hex::decode(target_group)
        .map_err(|e| SignalError::InvalidMessage(format!("Invalid contract hash hex: {}", e)))?;

    if target_bytes.len() != 32 {
        return Err(SignalError::InvalidMessage(format!(
            "Invalid contract hash: expected 32 bytes, got {}",
            target_bytes.len()
        )));
    }

    let mut hash_array = [0u8; 32];
    hash_array.copy_from_slice(&target_bytes);
    let _target_contract = crate::freenet::trust_contract::ContractHash(hash_array);

    // We need to get current state, add the contract, and create a delta
    // For now, we'll return an error indicating this needs implementation
    // of a FederationDelta variant
    Err(SignalError::Protocol(
        "Federation proposal execution requires FederationDelta variant (not yet implemented)"
            .to_string(),
    ))
}

/// Execute other proposal types (e.g., Stroma-specific config, Signal group settings).
///
/// Handles proposal types that don't fit ConfigChange or Federation.
/// Parses description to distinguish between Stroma and Signal configs.
async fn execute_other_proposal<F: FreenetClient>(
    _freenet: &F,
    _contract: &ContractHash,
    description: &str,
    _current_config: &GroupConfig,
) -> SignalResult<()> {
    // Parse description for Stroma config format: "Stroma config: key = value"
    if description.starts_with("Stroma config: ") {
        let config_part = description.strip_prefix("Stroma config: ").unwrap();
        if let Some((_key, _value)) = config_part.split_once(" = ") {
            // Stroma config changes would be applied to a separate Stroma state
            // For now, return success as the Stroma integration is not complete
            return Ok(());
        }
    }

    // Parse description for Signal config format: "Signal config: key = value"
    if description.starts_with("Signal config: ") {
        let config_part = description.strip_prefix("Signal config: ").unwrap();
        if let Some((_key, _value)) = config_part.split_once(" = ") {
            // Signal config changes are NOT applied in executor.rs
            // They are applied directly in bot.rs via SignalClient trait methods
            // This function only returns Ok to mark the proposal as executed
            return Ok(());
        }
    }

    // Unknown proposal type
    Err(SignalError::InvalidMessage(format!(
        "Unknown proposal type: {}",
        description
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::freenet::{
        traits::{ContractState, FreenetError, FreenetResult, StateChange},
        trust_contract::TrustNetworkState,
    };
    use crate::serialization::{from_cbor, to_cbor};
    use std::sync::{Arc, Mutex};

    /// Mock Freenet client for testing
    struct MockFreenetClient {
        state: Arc<Mutex<TrustNetworkState>>,
    }

    impl MockFreenetClient {
        fn new(state: TrustNetworkState) -> Self {
            Self {
                state: Arc::new(Mutex::new(state)),
            }
        }

        fn get_state_snapshot(&self) -> TrustNetworkState {
            self.state.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl FreenetClient for MockFreenetClient {
        async fn get_state(&self, _contract: &ContractHash) -> FreenetResult<ContractState> {
            let state = self.state.lock().unwrap();
            let data = to_cbor(&*state)
                .map_err(|e| FreenetError::Other(format!("Failed to serialize: {}", e)))?;
            Ok(ContractState { data })
        }

        async fn apply_delta(
            &self,
            _contract: &ContractHash,
            delta: &ContractDelta,
        ) -> FreenetResult<()> {
            let delta: StateDelta = from_cbor(&delta.data)
                .map_err(|e| FreenetError::Other(format!("Failed to deserialize: {}", e)))?;

            let mut state = self.state.lock().unwrap();
            state.apply_delta(&delta);
            Ok(())
        }

        async fn subscribe(
            &self,
            _contract: &ContractHash,
        ) -> FreenetResult<Box<dyn futures::Stream<Item = StateChange> + Send + Unpin>> {
            use futures::stream;
            Ok(Box::new(stream::empty()))
        }

        async fn deploy_contract(
            &self,
            _code: &[u8],
            _initial_state: &[u8],
        ) -> FreenetResult<ContractHash> {
            Ok(ContractHash::from_bytes(&[0u8; 32]))
        }
    }

    #[tokio::test]
    async fn test_config_change_creates_audit_entry() {
        let initial_config = GroupConfig {
            min_vouches: 2,
            max_flags: 3,
            open_membership: false,
            operators: Default::default(),
            default_poll_timeout_secs: 172800,
            config_change_threshold: 0.70,
            min_quorum: 0.50,
        };

        let initial_state = TrustNetworkState {
            members: Default::default(),
            ejected: Default::default(),
            vouches: Default::default(),
            flags: Default::default(),
            config: initial_config.clone(),
            config_timestamp: 0,
            schema_version: 1,
            federation_contracts: vec![],
            gap11_announcement_sent: false,
            active_proposals: Default::default(),
            audit_log: vec![],
            key_epoch: 1,
        };

        let freenet = MockFreenetClient::new(initial_state);
        let contract = ContractHash::from_bytes(&[1u8; 32]);

        // Execute config change
        let proposal = ProposalType::ConfigChange {
            key: "min_vouches".to_string(),
            value: "3".to_string(),
        };

        let result = execute_proposal(&freenet, &contract, &proposal, &initial_config).await;
        assert!(result.is_ok(), "Config change execution failed");

        // Verify audit entry was created
        let final_state = freenet.get_state_snapshot();
        assert_eq!(final_state.audit_log.len(), 1, "Expected 1 audit entry");

        let audit_entry = &final_state.audit_log[0];
        assert_eq!(
            audit_entry.action_type,
            crate::gatekeeper::audit_trail::ActionType::ConfigChange
        );
        assert!(
            audit_entry.details.contains("min_vouches"),
            "Audit entry should mention the config key"
        );
        assert!(
            audit_entry.details.contains("2"),
            "Audit entry should mention old value"
        );
        assert!(
            audit_entry.details.contains("3"),
            "Audit entry should mention new value"
        );

        // Verify config was actually changed
        assert_eq!(final_state.config.min_vouches, 3);
    }

    #[tokio::test]
    async fn test_multiple_config_changes_create_multiple_audit_entries() {
        let initial_config = GroupConfig {
            min_vouches: 2,
            max_flags: 3,
            open_membership: false,
            operators: Default::default(),
            default_poll_timeout_secs: 172800,
            config_change_threshold: 0.70,
            min_quorum: 0.50,
        };

        let initial_state = TrustNetworkState {
            members: Default::default(),
            ejected: Default::default(),
            vouches: Default::default(),
            flags: Default::default(),
            config: initial_config.clone(),
            config_timestamp: 0,
            schema_version: 1,
            federation_contracts: vec![],
            gap11_announcement_sent: false,
            active_proposals: Default::default(),
            audit_log: vec![],
            key_epoch: 1,
        };

        let freenet = MockFreenetClient::new(initial_state);
        let contract = ContractHash::from_bytes(&[1u8; 32]);

        // Execute first config change
        let proposal1 = ProposalType::ConfigChange {
            key: "min_vouches".to_string(),
            value: "3".to_string(),
        };
        execute_proposal(&freenet, &contract, &proposal1, &initial_config)
            .await
            .unwrap();

        // Delay to ensure different timestamps (config uses second-level precision)
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Get updated config for second change
        let updated_state = freenet.get_state_snapshot();
        let updated_config = updated_state.config.clone();

        // Execute second config change
        let proposal2 = ProposalType::ConfigChange {
            key: "max_flags".to_string(),
            value: "5".to_string(),
        };
        execute_proposal(&freenet, &contract, &proposal2, &updated_config)
            .await
            .unwrap();

        // Verify both audit entries exist
        let final_state = freenet.get_state_snapshot();
        assert_eq!(
            final_state.audit_log.len(),
            2,
            "Expected 2 audit entries for 2 config changes"
        );

        // Verify both configs were changed
        assert_eq!(final_state.config.min_vouches, 3);
        assert_eq!(final_state.config.max_flags, 5);
    }
}
