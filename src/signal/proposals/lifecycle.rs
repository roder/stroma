//! Proposal lifecycle management.
//!
//! Handles creation, monitoring, termination, and execution of proposals.

use super::command::{ProposalSubcommand, ProposeArgs};
use crate::freenet::{trust_contract::GroupConfig, FreenetClient};
use crate::signal::{
    polls::{PollManager, PollProposal, ProposalType},
    traits::{SignalClient, SignalResult},
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
    _freenet: &F,
    args: ProposeArgs,
    config: &GroupConfig,
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

    // 5. TODO: Store in Freenet with expires_at
    //
    // Per proposal-system.bead, proposals should be persisted in Freenet:
    //
    // ```rust
    // pub struct ActiveProposal {
    //     proposal: PollProposal,
    //     poll_timestamp: u64,        // Poll creation timestamp
    //     expires_at: u64,            // created_at + timeout
    //     approve_count: u32,         // Vote aggregates only (no individual votes)
    //     reject_count: u32,
    //     checked: bool,              // False until timeout expires
    //     result: Option<ProposalResult>,
    // }
    // ```
    //
    // This requires:
    // 1. Add `active_proposals: HashMap<u64, ActiveProposal>` to TrustNetworkState
    // 2. Create ProposalDelta variant in StateDelta
    // 3. Implement storage: freenet.store_proposal(poll_id, active_proposal).await?
    // 4. Implement retrieval for checking expired proposals
    // 5. Implement marking as checked after ProposalExpired is handled
    //
    // For now, proposals work in-memory via PollManager.active_polls.
    // State stream monitoring generates ProposalExpired events from external source.

    Ok(poll_id)
}
