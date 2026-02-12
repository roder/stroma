//! Proposal lifecycle management.
//!
//! Handles creation, monitoring, termination, and execution of proposals.

use super::command::{ProposalSubcommand, ProposeArgs};
use super::executor::execute_proposal;
use crate::freenet::{
    traits::{ContractDelta, ContractHash},
    trust_contract::{ActiveProposal, GroupConfig, ProposalResult, StateDelta, TrustNetworkState},
    FreenetClient,
};
use crate::serialization::{from_cbor, to_cbor};
use crate::signal::{
    polls::{PollManager, PollOutcome, PollProposal, ProposalType},
    traits::{GroupId, SignalClient, SignalError, SignalResult},
};
use futures::StreamExt;

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
        ProposalSubcommand::Signal { key, value } => {
            let question = format!("Change Signal {} to {}?", key, value);
            (
                ProposalType::Other {
                    description: format!("Signal config: {} = {}", key, value),
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

/// Monitor Freenet state stream for expired proposals.
///
/// This function subscribes to the Freenet state stream and checks for proposals
/// that have expired (expires_at <= current_time). When found, it triggers the
/// terminate → execute → announce workflow.
///
/// Per proposal-system.bead: Use real-time stream monitoring (NOT polling).
pub async fn monitor_proposals<C: SignalClient, F: FreenetClient>(
    poll_manager: &mut PollManager<C>,
    freenet: &F,
    contract: &ContractHash,
    group_id: &GroupId,
) -> SignalResult<()> {
    // Subscribe to Freenet state stream
    let mut stream = freenet.subscribe(contract).await.map_err(|e| {
        SignalError::Protocol(format!("Failed to subscribe to state stream: {}", e))
    })?;

    // Monitor for state changes
    while let Some(change) = stream.next().await {
        // Parse the new state
        let state: TrustNetworkState = from_cbor(&change.new_state.data)
            .map_err(|e| SignalError::Protocol(format!("Failed to deserialize state: {}", e)))?;

        // Get current time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check for expired proposals
        for (poll_id, proposal) in &state.active_proposals {
            if !proposal.checked && proposal.expires_at <= now {
                // Proposal has expired - trigger workflow
                tracing::info!(
                    poll_id = poll_id,
                    expires_at = proposal.expires_at,
                    "Proposal expired, triggering terminate→execute→announce flow"
                );

                // Execute the complete workflow
                if let Err(e) = process_expired_proposal(
                    poll_manager,
                    freenet,
                    contract,
                    group_id,
                    *poll_id,
                    proposal,
                    &state.config,
                )
                .await
                {
                    tracing::error!(
                        poll_id = poll_id,
                        error = %e,
                        "Failed to process expired proposal"
                    );
                }
            }
        }
    }

    Ok(())
}

/// Process an expired proposal: terminate → check outcome → execute → announce → mark checked.
///
/// Complete workflow for a single expired proposal.
async fn process_expired_proposal<C: SignalClient, F: FreenetClient>(
    poll_manager: &mut PollManager<C>,
    freenet: &F,
    contract: &ContractHash,
    _group_id: &GroupId,
    poll_id: u64,
    proposal: &ActiveProposal,
    config: &GroupConfig,
) -> SignalResult<()> {
    // Step 1: Terminate the poll (close voting in Signal)
    tracing::info!(poll_id = poll_id, "Terminating poll");
    poll_manager.terminate_poll(proposal.poll_timestamp).await?;

    // Step 2: Get vote aggregate from poll_manager
    let votes = poll_manager
        .get_vote_aggregate(poll_id)
        .ok_or_else(|| SignalError::Protocol(format!("No vote aggregate for poll {}", poll_id)))?;

    // Step 3: Check poll outcome (quorum + threshold)
    tracing::info!(
        poll_id = poll_id,
        approve = votes.approve,
        reject = votes.reject,
        total_members = votes.total_members,
        "Checking poll outcome"
    );

    let outcome = poll_manager.check_poll_outcome(poll_id, votes);

    // Step 4: Handle outcome
    let (result, should_execute) = match outcome {
        Some(PollOutcome::Passed {
            approve_count,
            reject_count,
        }) => {
            tracing::info!(
                poll_id = poll_id,
                approve = approve_count,
                reject = reject_count,
                "Proposal PASSED"
            );
            (
                ProposalResult::Passed {
                    approve_count,
                    reject_count,
                },
                true,
            )
        }
        Some(PollOutcome::Failed {
            approve_count,
            reject_count,
        }) => {
            tracing::info!(
                poll_id = poll_id,
                approve = approve_count,
                reject = reject_count,
                "Proposal FAILED"
            );
            (
                ProposalResult::Failed {
                    approve_count,
                    reject_count,
                },
                false,
            )
        }
        Some(PollOutcome::QuorumNotMet {
            participation_rate,
            required_quorum: _,
        }) => {
            tracing::info!(
                poll_id = poll_id,
                participation_rate = participation_rate,
                "Proposal FAILED: Quorum not met"
            );
            (ProposalResult::QuorumNotMet { participation_rate }, false)
        }
        None => {
            return Err(SignalError::Protocol(format!(
                "Failed to determine outcome for poll {}",
                poll_id
            )));
        }
    };

    // Step 5: Execute if passed
    if should_execute {
        tracing::info!(poll_id = poll_id, "Executing approved proposal");

        // Parse proposal type from stored strings
        let proposal_type =
            parse_proposal_type(&proposal.proposal_type, &proposal.proposal_details)?;

        execute_proposal(freenet, contract, &proposal_type, config).await?;
    }

    // Step 6: Mark proposal as checked in Freenet
    tracing::info!(poll_id = poll_id, "Marking proposal as checked");
    mark_proposal_checked(freenet, contract, poll_id, result).await?;

    Ok(())
}

/// Parse ProposalType from stored strings.
fn parse_proposal_type(
    proposal_type_str: &str,
    proposal_details: &str,
) -> SignalResult<ProposalType> {
    match proposal_type_str {
        "ConfigChange" => {
            // Format: "key=value"
            let parts: Vec<&str> = proposal_details.splitn(2, '=').collect();
            if parts.len() != 2 {
                return Err(SignalError::InvalidMessage(format!(
                    "Invalid ConfigChange details: {}",
                    proposal_details
                )));
            }
            Ok(ProposalType::ConfigChange {
                key: parts[0].to_string(),
                value: parts[1].to_string(),
            })
        }
        "Federation" => Ok(ProposalType::Federation {
            target_group: proposal_details.to_string(),
        }),
        "Other" => Ok(ProposalType::Other {
            description: proposal_details.to_string(),
        }),
        _ => Err(SignalError::InvalidMessage(format!(
            "Unknown proposal type: {}",
            proposal_type_str
        ))),
    }
}

/// Mark a proposal as checked with result in Freenet.
async fn mark_proposal_checked<F: FreenetClient>(
    freenet: &F,
    contract: &ContractHash,
    poll_id: u64,
    result: ProposalResult,
) -> SignalResult<()> {
    let delta = StateDelta {
        members_added: vec![],
        members_removed: vec![],
        vouches_added: vec![],
        vouches_removed: vec![],
        flags_added: vec![],
        flags_removed: vec![],
        config_update: None,
        proposals_created: vec![],
        proposals_checked: vec![poll_id],
        proposals_with_results: vec![(poll_id, result)],
        audit_entries_added: vec![],
        gap11_announcement_sent: false,
    };

    let delta_bytes = to_cbor(&delta)
        .map_err(|e| SignalError::Protocol(format!("Failed to serialize delta: {}", e)))?;

    let contract_delta = ContractDelta { data: delta_bytes };
    freenet
        .apply_delta(contract, &contract_delta)
        .await
        .map_err(|e| SignalError::Protocol(format!("Failed to apply delta: {}", e)))?;

    Ok(())
}
