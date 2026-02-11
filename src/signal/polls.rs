//! Poll Handling (Signal Protocol v8)
//!
//! Uses native Signal polls for voting on proposals.
//! Structured voting with multiple choice options.
//!
//! See: .beads/signal-integration.bead § Voting: Native Signal Polls

use super::stroma_store::StromaStore;
use super::traits::*;
use hkdf::Hkdf;
use ring::hmac;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Poll manager for proposals
pub struct PollManager<C: SignalClient> {
    client: C,
    group_id: GroupId,
    active_polls: HashMap<u64, PollProposal>,
    /// Vote aggregates (only counts, NEVER individual voter identities).
    /// Per GAP-02: Individual votes MUST NOT be persisted.
    vote_aggregates: HashMap<u64, VoteAggregate>,
    /// Voter deduplication map (HMAC'd voter identities -> selected options)
    /// poll_id -> (HMAC(voter_ACI) -> selected_options)
    /// Stored encrypted in SQLite, zeroized on poll completion
    voter_selections: HashMap<u64, HashMap<String, Vec<u32>>>,
    /// Encrypted store for persistence (optional)
    store: Option<StromaStore>,
    /// HMAC pepper derived from operator ACI (zeroized on drop)
    pepper: VoterPepper,
}

/// Proposal being voted on
#[derive(Clone, Serialize, Deserialize)]
pub struct PollProposal {
    pub proposal_type: ProposalType,
    pub poll_id: u64,
    pub timeout: u64,
    pub threshold: f32,
    pub quorum: f32,
}

/// Proposal types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProposalType {
    ConfigChange { key: String, value: String },
    Federation { target_group: String },
    Other { description: String },
}

/// HMAC pepper for voter identity masking (zeroized on drop)
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
struct VoterPepper([u8; 32]);

impl<C: SignalClient> PollManager<C> {
    /// Create a new PollManager
    ///
    /// # Arguments
    /// * `client` - Signal client
    /// * `group_id` - Group ID for polls
    /// * `aci_key` - Operator's ACI key for deriving HMAC pepper (32 bytes)
    /// * `store` - Optional encrypted store for persistence
    pub fn new(
        client: C,
        group_id: GroupId,
        aci_key: &[u8],
        store: Option<StromaStore>,
    ) -> Result<Self, SignalError> {
        let pepper = derive_voter_pepper(aci_key)?;

        Ok(Self {
            client,
            group_id,
            active_polls: HashMap::new(),
            vote_aggregates: HashMap::new(),
            voter_selections: HashMap::new(),
            store,
            pepper,
        })
    }

    /// Initialize vote aggregate for a new poll.
    pub fn init_vote_aggregate(&mut self, poll_id: u64, total_members: u32) {
        self.vote_aggregates.insert(
            poll_id,
            VoteAggregate {
                approve: 0,
                reject: 0,
                total_members,
            },
        );
    }

    /// Get vote aggregate for a poll (if it exists).
    pub fn get_vote_aggregate(&self, poll_id: u64) -> Option<&VoteAggregate> {
        self.vote_aggregates.get(&poll_id)
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

    /// Process poll vote with voter deduplication
    ///
    /// CRITICAL: Voter identities are HMAC'd before storage.
    /// See: .beads/security-constraints.bead § Vote Privacy
    ///
    /// This method:
    /// 1. HMACs voter ACI to create privacy-preserving identifier
    /// 2. Checks for previous vote by this voter (vote change)
    /// 3. If vote changed: decrements old options, increments new ones
    /// 4. Stores HMAC'd voter -> selected_options (encrypted in SQLite)
    /// 5. Never stores raw voter identities
    ///
    /// # Arguments
    /// * `vote` - The poll vote to process
    /// * `voter_aci` - Voter's Signal ACI (will be HMAC'd, not stored raw)
    pub async fn process_vote(&mut self, vote: &PollVote, voter_aci: &str) -> SignalResult<()> {
        let poll_id = vote.poll_id;

        // HMAC the voter ACI for privacy-preserving deduplication
        let voter_hmac = hmac_voter_identity(voter_aci, &self.pepper);

        // Get or create aggregate for this poll
        let aggregate = self
            .vote_aggregates
            .entry(poll_id)
            .or_insert_with(|| VoteAggregate {
                approve: 0,
                reject: 0,
                total_members: 0, // Will be set during poll creation
            });

        // Get or create voter selections map for this poll
        let selections = self.voter_selections.entry(poll_id).or_insert_with(HashMap::new);

        // Check if voter has voted before (vote change)
        if let Some(previous_vote) = selections.get(&voter_hmac) {
            // Decrement old selections
            for option in previous_vote {
                match option {
                    0 => aggregate.approve = aggregate.approve.saturating_sub(1),
                    1 => aggregate.reject = aggregate.reject.saturating_sub(1),
                    _ => {}
                }
            }
        }

        // Increment new selections
        for option in &vote.selected_options {
            match option {
                0 => aggregate.approve += 1,
                1 => aggregate.reject += 1,
                _ => {
                    // Unknown option, ignore
                }
            }
        }

        // Store HMAC'd voter -> selected_options (for future deduplication)
        selections.insert(voter_hmac, vote.selected_options.clone());

        // Persist state to encrypted store
        self.persist_poll_state().await?;

        Ok(())
    }

    /// Terminate a poll (closes voting).
    ///
    /// Per proposal-system.bead:
    /// - Sends PollTerminate message to close the poll
    /// - Prevents late votes after timeout expires
    /// - Visual feedback in Signal UI
    pub async fn terminate_poll(&self, poll_timestamp: u64) -> SignalResult<()> {
        self.client
            .terminate_poll(&self.group_id, poll_timestamp)
            .await
    }

    /// Get proposal by poll_id.
    ///
    /// Returns the PollProposal if it exists in active_polls.
    pub fn get_proposal(&self, poll_id: u64) -> Option<&PollProposal> {
        self.active_polls.get(&poll_id)
    }

    /// Check if poll has reached quorum and threshold
    ///
    /// IMPORTANT: When this returns a definitive outcome (Passed/Failed),
    /// the caller MUST call zeroize_poll() to remove voter dedup data.
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

    /// Zeroize poll data after outcome is determined
    ///
    /// Removes:
    /// - Active poll entry
    /// - Vote aggregates
    /// - Voter deduplication map (HMAC'd identities)
    /// - Persisted state from encrypted store
    pub async fn zeroize_poll(&mut self, poll_id: u64) -> SignalResult<()> {
        // Remove from in-memory structures
        self.active_polls.remove(&poll_id);
        self.vote_aggregates.remove(&poll_id);

        // Remove voter selections for this poll (clears HMAC'd identities)
        self.voter_selections.remove(&poll_id);

        // Persist the updated state (removes poll from encrypted store)
        self.persist_poll_state().await?;

        Ok(())
    }

    /// Persist poll state to encrypted store
    pub async fn persist_poll_state(&self) -> SignalResult<()> {
        if let Some(store) = &self.store {
            let state = PollState {
                active_polls: self.active_polls.clone(),
                vote_aggregates: self.vote_aggregates.clone(),
                voter_selections: self.voter_selections.clone(),
            };

            let serialized = serde_json::to_vec(&state)
                .map_err(|e| SignalError::Store(format!("Serialization failed: {}", e)))?;

            // Store in encrypted SQLite under a fixed key
            // TODO: Use StromaStore's actual persistence API once available
            // For now, we'll use a placeholder that assumes StromaStore has a key-value interface
            let _ = store; // Placeholder - actual implementation will call store.save()
            let _ = serialized;

            Ok(())
        } else {
            // No store configured, state is ephemeral
            Ok(())
        }
    }

    /// Restore poll state from encrypted store
    pub async fn restore_poll_state(&mut self) -> SignalResult<()> {
        if let Some(store) = &self.store {
            // TODO: Use StromaStore's actual persistence API once available
            // For now, we'll use a placeholder
            let _ = store;

            // Placeholder - actual implementation will call store.load()
            // let serialized = store.load(POLL_STATE_KEY).await?;
            // let state: PollState = serde_json::from_slice(&serialized)?;
            // self.active_polls = state.active_polls;
            // self.vote_aggregates = state.vote_aggregates;
            // self.voter_selections = state.voter_selections;

            Ok(())
        } else {
            // No store configured, nothing to restore
            Ok(())
        }
    }
}

/// Serializable poll state for persistence
#[derive(Serialize, Deserialize)]
struct PollState {
    active_polls: HashMap<u64, PollProposal>,
    vote_aggregates: HashMap<u64, VoteAggregate>,
    voter_selections: HashMap<u64, HashMap<String, Vec<u32>>>,
}

/// Derive HMAC pepper from operator ACI key
///
/// Uses HKDF-SHA256 with context separation:
/// - Salt: "stroma-voter-dedup-v1"
/// - Info: "hmac-pepper"
fn derive_voter_pepper(aci_key: &[u8]) -> Result<VoterPepper, SignalError> {
    const SALT: &[u8] = b"stroma-voter-dedup-v1";
    const INFO: &[u8] = b"hmac-pepper";

    let hkdf = Hkdf::<Sha256>::new(Some(SALT), aci_key);
    let mut pepper = [0u8; 32];
    hkdf.expand(INFO, &mut pepper)
        .map_err(|e| SignalError::Store(format!("Pepper derivation failed: {}", e)))?;

    Ok(VoterPepper(pepper))
}

/// Compute HMAC of voter ACI for privacy-preserving deduplication
///
/// Returns hex-encoded HMAC as string for use as HashMap key
fn hmac_voter_identity(voter_aci: &str, pepper: &VoterPepper) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, &pepper.0);
    let tag = hmac::sign(&key, voter_aci.as_bytes());
    hex::encode(tag.as_ref())
}

/// Vote aggregate (only counts, never individual voters)
#[derive(Clone, Serialize, Deserialize)]
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

        let aci_key = [42u8; 32];
        let mut manager = PollManager::new(client, group, &aci_key, None).unwrap();

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
        let aci_key = [42u8; 32];
        let manager = PollManager::new(client, group, &aci_key, None).unwrap();

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
            approve: 8, // 80% of voters
            reject: 2,  // 20% of voters
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
        let aci_key = [42u8; 32];
        let manager = PollManager::new(client, group, &aci_key, None).unwrap();

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
            approve: 3, // 30% of voters (below 70% threshold)
            reject: 7,  // 70% of voters
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
        let aci_key = [42u8; 32];
        let manager = PollManager::new(client, group, &aci_key, None).unwrap();

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
            approve: 4, // Only 50% participation
            reject: 1,
            total_members: 10,
        };

        let outcome = manager.check_poll_outcome(poll_id, &votes).unwrap();

        assert!(matches!(outcome, PollOutcome::QuorumNotMet { .. }));
    }

    #[tokio::test]
    async fn test_voter_deduplication() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let group = GroupId(vec![1, 2, 3]);
        let aci_key = [42u8; 32];
        let mut manager = PollManager::new(client, group, &aci_key, None).unwrap();

        let poll_id = 1;
        manager.init_vote_aggregate(poll_id, 10);

        // Voter 1 votes approve
        let vote1 = PollVote {
            poll_id,
            selected_options: vec![0], // Approve
        };
        manager.process_vote(&vote1, "voter1_aci").await.unwrap();

        // Check vote count
        let agg = manager.get_vote_aggregate(poll_id).unwrap();
        assert_eq!(agg.approve, 1);
        assert_eq!(agg.reject, 0);

        // Voter 1 changes vote to reject (deduplication)
        let vote2 = PollVote {
            poll_id,
            selected_options: vec![1], // Reject
        };
        manager.process_vote(&vote2, "voter1_aci").await.unwrap();

        // Check that approve was decremented and reject was incremented
        let agg = manager.get_vote_aggregate(poll_id).unwrap();
        assert_eq!(agg.approve, 0);
        assert_eq!(agg.reject, 1);

        // Different voter votes approve
        let vote3 = PollVote {
            poll_id,
            selected_options: vec![0], // Approve
        };
        manager.process_vote(&vote3, "voter2_aci").await.unwrap();

        // Check that both votes are counted
        let agg = manager.get_vote_aggregate(poll_id).unwrap();
        assert_eq!(agg.approve, 1);
        assert_eq!(agg.reject, 1);
    }

    #[tokio::test]
    async fn test_zeroize_poll() {
        let client = MockSignalClient::new(ServiceId("bot".to_string()));
        let group = GroupId(vec![1, 2, 3]);
        let aci_key = [42u8; 32];
        let mut manager = PollManager::new(client, group, &aci_key, None).unwrap();

        let poll_id = 1;

        // Create a proposal and add it to active_polls
        let proposal = PollProposal {
            proposal_type: ProposalType::ConfigChange {
                key: "test".to_string(),
                value: "value".to_string(),
            },
            poll_id,
            timeout: 172800,
            threshold: 0.7,
            quorum: 0.5,
        };
        manager.active_polls.insert(poll_id, proposal);
        manager.init_vote_aggregate(poll_id, 10);

        // Add a vote
        let vote = PollVote {
            poll_id,
            selected_options: vec![0],
        };
        manager.process_vote(&vote, "voter1_aci").await.unwrap();

        // Verify data exists
        assert!(manager.active_polls.contains_key(&poll_id));
        assert!(manager.vote_aggregates.contains_key(&poll_id));
        assert!(manager.voter_selections.contains_key(&poll_id));

        // Zeroize the poll
        manager.zeroize_poll(poll_id).await.unwrap();

        // Verify all data is removed
        assert!(!manager.active_polls.contains_key(&poll_id));
        assert!(!manager.vote_aggregates.contains_key(&poll_id));
        assert!(!manager.voter_selections.contains_key(&poll_id));
    }
}
