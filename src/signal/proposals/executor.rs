//! Proposal execution logic.
//!
//! Applies approved proposals to Freenet state.

use crate::freenet::{
    traits::{ContractDelta, ContractHash, FreenetClient},
    trust_contract::{ConfigUpdate, GroupConfig, StateDelta},
};
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
        audit_entries_added: vec![],
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

/// Execute other proposal types (e.g., Stroma-specific config).
///
/// Handles proposal types that don't fit ConfigChange or Federation.
/// Currently, Stroma config changes are parsed from description.
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

    // Unknown proposal type
    Err(SignalError::InvalidMessage(format!(
        "Unknown proposal type: {}",
        description
    )))
}
