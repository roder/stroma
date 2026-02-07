//! Proposal lifecycle management.
//!
//! Handles creation, monitoring, termination, and execution of proposals.

use super::command::{ProposalSubcommand, ProposeArgs};
use crate::freenet::{
    traits::ContractDelta,
    trust_contract::{ActiveProposal, GroupConfig, StateDelta},
    FreenetClient,
};
use crate::serialization::to_cbor;
use crate::signal::{
    polls::{PollManager, PollProposal, ProposalType},
    traits::{SignalClient, SignalError, SignalResult},
};

/// Create a proposal poll.
///
/// Steps:
/// 1. Determine timeout (args.timeout OR config.default_poll_timeout)
/// 2. Create PollProposal with timeout/threshold/quorum from config
/// 3. Create Signal poll via poll_manager
/// 4. Store in Freenet with expires_at timestamp
/// 5. Return poll_id
pub async fn create_proposal<C: SignalClient, F: FreenetClient>(
    poll_manager: &mut PollManager<C>,
    freenet: &F,
    args: ProposeArgs,
    config: &GroupConfig,
    contract_hash: &crate::freenet::traits::ContractHash,
) -> SignalResult<u64> {
    // 1. Determine timeout
    let timeout = args
        .timeout
        .unwrap_or_else(|| config.default_poll_timeout());

    // 2. Create proposal type and question
    let (proposal_type, question) = match &args.subcommand {
        ProposalSubcommand::Config { key, value } => {
            let question = format!("Change {} to {}?", key, value);
            (
                ProposalType::ConfigChange {
                    key: key.clone(),
                    value: value.clone(),
                },
                question,
            )
        }
        ProposalSubcommand::Stroma { key, value } => {
            let question = format!("Change Stroma {} to {}?", key, value);
            (
                ProposalType::Other {
                    description: format!("Stroma config: {} = {}", key, value),
                },
                question,
            )
        }
    };

    // 3. Create PollProposal struct
    // Note: poll_id will be filled in after creation, using 0 as placeholder
    let timeout_secs = timeout.as_secs();
    let threshold = config.config_change_threshold;
    let quorum = config.min_quorum;

    // Encode proposal type and details for storage (before moving proposal_type)
    let (proposal_type_str, proposal_details) = match &proposal_type {
        ProposalType::ConfigChange { key, value } => {
            ("ConfigChange".to_string(), format!("{}={}", key, value))
        }
        ProposalType::Federation { target_group } => {
            ("Federation".to_string(), target_group.clone())
        }
        ProposalType::Other { description } => ("Other".to_string(), description.clone()),
    };

    let proposal = PollProposal {
        proposal_type,
        poll_id: 0, // Will be updated by poll_manager
        timeout: timeout_secs,
        threshold,
        quorum,
    };

    // 4. Create poll via PollManager
    let options = vec!["Approve".to_string(), "Reject".to_string()];
    let poll_id = poll_manager
        .create_proposal_poll(proposal, question, options)
        .await?;

    // 5. Store in Freenet with expires_at
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let expires_at = now + timeout_secs;

    let active_proposal = ActiveProposal {
        poll_id,
        proposal_type: proposal_type_str,
        proposal_details,
        poll_timestamp: now,
        expires_at,
        timeout_secs,
        threshold,
        quorum,
        checked: false,
        result: None,
    };

    // Create delta to add proposal to Freenet
    let delta = StateDelta {
        members_added: vec![],
        members_removed: vec![],
        vouches_added: vec![],
        vouches_removed: vec![],
        flags_added: vec![],
        flags_removed: vec![],
        config_update: None,
        proposals_created: vec![(poll_id, active_proposal)],
        proposals_checked: vec![],
        proposals_with_results: vec![],
        audit_entries_added: vec![],
        gap11_announcement_sent: false,
    };

    // Serialize and apply delta
    let delta_bytes = to_cbor(&delta)
        .map_err(|e| SignalError::Protocol(format!("Failed to serialize proposal delta: {}", e)))?;

    let contract_delta = ContractDelta { data: delta_bytes };
    freenet
        .apply_delta(contract_hash, &contract_delta)
        .await
        .map_err(|e| {
            SignalError::Protocol(format!("Failed to store proposal in Freenet: {}", e))
        })?;

    Ok(poll_id)
}
