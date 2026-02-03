//! Poll Handling (Signal Protocol v8)
//!
//! Uses native Signal polls for voting on proposals.
//! Structured voting with multiple choice options.
//!
//! See: .beads/signal-integration.bead § Voting: Native Signal Polls

use super::traits::*;
use std::collections::HashMap;

/// Poll manager for proposals
pub struct PollManager<C: SignalClient> {
    client: C,
    group_id: GroupId,
    active_polls: HashMap<u64, PollProposal>,
}

/// Proposal being voted on
pub struct PollProposal {
    pub proposal_type: ProposalType,
    pub poll_id: u64,
    pub timeout: u64,
    pub threshold: f32,
    pub quorum: f32,
}

/// Proposal types
#[derive(Debug, Clone, PartialEq)]
pub enum ProposalType {
    ConfigChange { key: String, value: String },
    Federation { target_group: String },
    Other { description: String },
}

impl<C: SignalClient> PollManager<C> {
    pub fn new(client: C, group_id: GroupId) -> Self {
        Self {
            client,
            group_id,
            active_polls: HashMap::new(),
        }
    }

    /// Create poll for proposal
    pub async fn create_proposal_poll(
        &mut self,
        proposal: PollProposal,
        question: String,
        options: Vec<String>,
    ) -> SignalResult<u64> {
        let poll = Poll { question, options };

        let poll_id = self.client.create_poll(&self.group_id, &poll).await?;
        self.active_polls.insert(poll_id, proposal);

        Ok(poll_id)
    }

    /// Process poll vote (ephemeral, not persisted)
    ///
    /// CRITICAL: Individual votes MUST NEVER be persisted.
    /// See: .beads/security-constraints.bead § Vote Privacy
    pub async fn process_vote(&self, _vote: &PollVote) -> SignalResult<()> {
        // TODO: Aggregate vote counts (not individual voter identities)
        // 1. Update aggregate counts (approve: N, reject: M)
        // 2. Check quorum and threshold
        // 3. If passed, update Freenet with OUTCOME only (no voter info)
        // 4. Never persist who voted for what

        Ok(())
    }

    /// Check if poll has reached quorum and threshold
    pub fn check_poll_outcome(&self, poll_id: u64, votes: &VoteAggregate) -> Option<PollOutcome> {
        let proposal = self.active_polls.get(&poll_id)?;

        let total_voters = votes.total_voters();
        let total_members = votes.total_members;

        // Check quorum (% of members who must vote)
        let participation_rate = total_voters as f32 / total_members as f32;
        if participation_rate < proposal.quorum {
            return Some(PollOutcome::QuorumNotMet {
                participation_rate,
                required_quorum: proposal.quorum,
            });
        }

        // Check threshold (% of votes to pass)
        let approve_rate = votes.approve as f32 / total_voters as f32;
        let outcome = if approve_rate >= proposal.threshold {
            PollOutcome::Passed {
                approve_count: votes.approve,
                reject_count: votes.reject,
            }
        } else {
            PollOutcome::Failed {
                approve_count: votes.approve,
                reject_count: votes.reject,
            }
        };

        Some(outcome)
    }
}

/// Vote aggregate (only counts, never individual voters)
pub struct VoteAggregate {
    pub approve: u32,
    pub reject: u32,
    pub total_members: u32,
}

impl VoteAggregate {
    pub fn total_voters(&self) -> u32 {
        self.approve + self.reject
    }
}

/// Poll outcome
#[derive(Debug, PartialEq)]
pub enum PollOutcome {
    Passed {
        approve_count: u32,
        reject_count: u32,
    },
    Failed {
        approve_count: u32,
        reject_count: u32,
    },
    QuorumNotMet {
        participation_rate: f32,
        required_quorum: f32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::mock::MockSignalClient;

    #[tokio::test]
    async fn test_create_poll() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let group = GroupId(vec![1, 2, 3]);
        let member = ServiceId("user1".to_string());

        client.add_group_member(&group, &member).await.unwrap();

        let mut manager = PollManager::new(client, group);

        let proposal = PollProposal {
            proposal_type: ProposalType::ConfigChange {
                key: "threshold".to_string(),
                value: "0.7".to_string(),
            },
            poll_id: 0,
            timeout: 172800, // 48 hours
            threshold: 0.7,
            quorum: 0.5,
        };

        let poll_id = manager
            .create_proposal_poll(
                proposal,
                "Change threshold to 70%?".to_string(),
                vec!["Approve".to_string(), "Reject".to_string()],
            )
            .await
            .unwrap();

        assert_eq!(poll_id, 0);
    }

    #[test]
    fn test_poll_outcome_passed() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let group = GroupId(vec![1, 2, 3]);
        let manager = PollManager::new(client, group);

        let proposal = PollProposal {
            proposal_type: ProposalType::ConfigChange {
                key: "test".to_string(),
                value: "value".to_string(),
            },
            poll_id: 0,
            timeout: 172800,
            threshold: 0.7, // 70% to pass
            quorum: 0.5,    // 50% must vote
        };

        let poll_id = 0;
        let mut manager = manager;
        manager.active_polls.insert(poll_id, proposal);

        let votes = VoteAggregate {
            approve: 8,      // 80% of voters
            reject: 2,       // 20% of voters
            total_members: 10,
        };

        let outcome = manager.check_poll_outcome(poll_id, &votes).unwrap();

        assert_eq!(
            outcome,
            PollOutcome::Passed {
                approve_count: 8,
                reject_count: 2
            }
        );
    }

    #[test]
    fn test_poll_outcome_failed() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let group = GroupId(vec![1, 2, 3]);
        let manager = PollManager::new(client, group);

        let proposal = PollProposal {
            proposal_type: ProposalType::ConfigChange {
                key: "test".to_string(),
                value: "value".to_string(),
            },
            poll_id: 0,
            timeout: 172800,
            threshold: 0.7, // 70% to pass
            quorum: 0.5,
        };

        let poll_id = 0;
        let mut manager = manager;
        manager.active_polls.insert(poll_id, proposal);

        let votes = VoteAggregate {
            approve: 3,        // 30% of voters (below 70% threshold)
            reject: 7,         // 70% of voters
            total_members: 10,
        };

        let outcome = manager.check_poll_outcome(poll_id, &votes).unwrap();

        assert_eq!(
            outcome,
            PollOutcome::Failed {
                approve_count: 3,
                reject_count: 7
            }
        );
    }

    #[test]
    fn test_poll_outcome_quorum_not_met() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let group = GroupId(vec![1, 2, 3]);
        let manager = PollManager::new(client, group);

        let proposal = PollProposal {
            proposal_type: ProposalType::ConfigChange {
                key: "test".to_string(),
                value: "value".to_string(),
            },
            poll_id: 0,
            timeout: 172800,
            threshold: 0.7,
            quorum: 0.6, // 60% must vote
        };

        let poll_id = 0;
        let mut manager = manager;
        manager.active_polls.insert(poll_id, proposal);

        let votes = VoteAggregate {
            approve: 4,        // Only 50% participation
            reject: 1,
            total_members: 10,
        };

        let outcome = manager.check_poll_outcome(poll_id, &votes).unwrap();

        assert!(matches!(outcome, PollOutcome::QuorumNotMet { .. }));
    }
}
